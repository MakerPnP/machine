//! Renders the tui.

use std::time::Duration;

use log::{Level, LevelFilter};
use ratatui::backend::Backend;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Cell, Clear, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState,
};
use ratatui::{Frame, Terminal};

use crate::ioboard::discovery::{ClaimStatus, IoBoardState, IoBoardStatus};
use crate::logging::store::LogEntry;
use crate::tui::app::{App, LevelRow, LevelsMenu, Popup, TextInput, level_rows};
use crate::tui::log_view::Scroll;

const HELP: &str =
    " ↑↓ PgUp PgDn scroll · Ctrl+Home first · Ctrl+End follow · F2 levels · F3 or / filter · F4 capacity · q quit ";

/// What's rendered from the log store, copied so the store isn't locked while rendering.
struct LogSnapshot {
    lines: Vec<Line<'static>>,
    shown: usize,
    kept: usize,
    capacity: usize,
    level_rows: Option<Vec<LevelRow>>,
}

struct Areas {
    status: Rect,
    log: Rect,
    help: Rect,
}

fn layout(area: Rect, io_board_count: usize) -> Areas {
    // borders, host line, table header and a row per board
    let status_height = 2 + 1 + 1 + io_board_count.max(1) as u16;
    let [status, log, help] = Layout::vertical([
        Constraint::Length(status_height),
        Constraint::Min(3),
        Constraint::Length(1),
    ])
    .areas(area);
    Areas { status, log, help }
}

pub fn draw<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<(), B::Error> {
    let io_boards = app.io_boards.borrow().clone();

    let size = terminal.size()?;
    let areas = layout(Rect::new(0, 0, size.width, size.height), io_boards.len());
    app.log_view
        .set_height(areas.log.height.saturating_sub(2) as usize);

    let snapshot = {
        let store = app.store.lock();
        app.log_view.sync(&store);
        LogSnapshot {
            lines: app
                .log_view
                .visible()
                .filter_map(|seq| store.get(seq))
                .map(log_line)
                .collect(),
            shown: app.log_view.len(),
            kept: store.len(),
            capacity: store.capacity(),
            level_rows: matches!(app.popup, Some(Popup::Levels(_))).then(|| level_rows(&store)),
        }
    };

    terminal.draw(|frame| {
        let areas = layout(frame.area(), io_boards.len());
        render_status(frame, areas.status, app, &io_boards);
        render_log(frame, areas.log, app, &snapshot);
        frame.render_widget(Paragraph::new(HELP).reversed(), areas.help);

        match &app.popup {
            None => {}
            Some(Popup::Levels(menu)) => render_levels(
                frame,
                menu,
                snapshot
                    .level_rows
                    .as_deref()
                    .unwrap_or_default(),
            ),
            Some(Popup::Filter(input)) => render_input(
                frame,
                " Message filter (regular expression, case-insensitive, empty for none) ",
                input,
            ),
            Some(Popup::Capacity(input)) => render_input(frame, " Log entries to keep ", input),
        }
    })?;
    Ok(())
}

fn render_status(frame: &mut Frame, area: Rect, app: &App, io_boards: &[IoBoardStatus]) {
    let block = Block::bordered().title(" Status ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [host_area, table_area] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(inner);

    let mut host = vec![Span::from("Host: ").bold()];
    if app.host_addresses.is_empty() {
        host.push(Span::from("no IPv4 addresses").dim());
    }
    for (index, (name, ip)) in app.host_addresses.iter().enumerate() {
        if index > 0 {
            host.push(Span::from(", "));
        }
        host.push(Span::from(ip.to_string()));
        host.push(Span::from(format!(" ({})", name)).dim());
    }
    host.push(Span::from("   Uptime: ").bold());
    host.push(Span::from(format_duration(app.started.elapsed())));
    if app.shutting_down {
        host.push(
            Span::from("   SHUTTING DOWN")
                .bold()
                .red(),
        );
    }
    frame.render_widget(Paragraph::new(Line::from(host)), host_area);

    let header = Row::new([
        "Name",
        "Serial",
        "Address",
        "Local address",
        "Claim",
        "State",
        "Last seen",
    ])
    .bold();
    let rows: Vec<Row> = if io_boards.is_empty() {
        vec![Row::new([Cell::from("no io boards configured").dim()])]
    } else {
        io_boards
            .iter()
            .map(io_board_row)
            .collect()
    };
    let widths = [
        Constraint::Min(8),
        Constraint::Length(26),
        Constraint::Length(21),
        Constraint::Length(21),
        Constraint::Length(28),
        Constraint::Length(15),
        Constraint::Length(10),
    ];
    frame.render_widget(Table::new(rows, widths).header(header), table_area);
}

fn io_board_row(status: &IoBoardStatus) -> Row<'static> {
    let serial = match (&status.board, status.expected_serial_number) {
        (Some(board), _) => Cell::from(board.serial_number.to_string()),
        (None, Some(expected)) => Cell::from(expected.to_string()).dim(),
        (None, None) => Cell::from("any").dim(),
    };
    let (state, color) = match status.state {
        IoBoardState::Waiting => ("waiting", Color::Yellow),
        IoBoardState::Claiming => ("claiming", Color::Cyan),
        IoBoardState::ReleasingStale => ("releasing stale", Color::Magenta),
        IoBoardState::Connected => ("connected", Color::Green),
    };
    let dash = || Cell::from("-").dim();
    match &status.board {
        None => Row::new([
            Cell::from(status.name.clone()),
            serial,
            dash(),
            dash(),
            dash(),
            Cell::from(state).fg(color),
            dash(),
        ]),
        Some(board) => {
            let claim = match board.claim {
                ClaimStatus::Unclaimed => Cell::from("unclaimed"),
                ClaimStatus::Us => Cell::from("us").green(),
                ClaimStatus::Stale(endpoint) => Cell::from(format!("stale {}", endpoint)).magenta(),
                ClaimStatus::Other(endpoint) => Cell::from(format!("other {}", endpoint)).red(),
            };
            Row::new([
                Cell::from(status.name.clone()),
                serial,
                Cell::from(board.address.to_string()),
                Cell::from(board.local_address.to_string()),
                claim,
                Cell::from(state).fg(color),
                Cell::from(format!("{:.1}s ago", board.last_seen.elapsed().as_secs_f32())),
            ])
        }
    }
}

fn level_color(level: Level) -> Color {
    match level {
        Level::Error => Color::Red,
        Level::Warn => Color::Yellow,
        Level::Info => Color::Green,
        Level::Debug => Color::Blue,
        Level::Trace => Color::Magenta,
    }
}

fn log_line(entry: &LogEntry) -> Line<'static> {
    Line::from(vec![
        Span::from(entry.format_timestamp().to_string()).dim(),
        Span::from(" "),
        Span::from(format!("{:<5}", entry.level)).fg(level_color(entry.level)),
        Span::from(" "),
        Span::from(entry.target.clone()).dim(),
        Span::from(" "),
        // one line per entry
        Span::from(entry.message.replace('\n', " ⏎ ")),
    ])
}

fn render_log(frame: &mut Frame, area: Rect, app: &App, snapshot: &LogSnapshot) {
    let scroll = match app.log_view.scroll() {
        Scroll::Follow => Span::from(" FOLLOW ").green(),
        Scroll::Locked { .. } => Span::from(" LOCKED ").yellow().bold(),
    };
    let mut title = vec![Span::from(" Log "), scroll];
    if app.log_view.evicted() > 0 {
        title.push(Span::from(format!(" {} older entries evicted ", app.log_view.evicted())).red());
    }
    title.push(Span::from(format!(
        " {} shown · {} kept · capacity {} ",
        snapshot.shown, snapshot.kept, snapshot.capacity
    )));
    if !app.filter.is_empty() {
        title.push(Span::from(format!(" /{}/ ", app.filter)).cyan());
    }

    let block = Block::bordered().title(Line::from(title));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(snapshot.lines.clone()), inner);

    let mut scrollbar = ScrollbarState::new(
        snapshot
            .shown
            .saturating_sub(inner.height as usize),
    )
    .position(app.log_view.position());
    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight),
        area.inner(ratatui::layout::Margin {
            horizontal: 0,
            vertical: 1,
        }),
        &mut scrollbar,
    );
}

fn level_name(level: LevelFilter) -> &'static str {
    match level {
        LevelFilter::Off => "off",
        LevelFilter::Error => "error",
        LevelFilter::Warn => "warn",
        LevelFilter::Info => "info",
        LevelFilter::Debug => "debug",
        LevelFilter::Trace => "trace",
    }
}

fn level_filter_color(level: LevelFilter) -> Color {
    level
        .to_level()
        .map(level_color)
        .unwrap_or(Color::DarkGray)
}

fn render_levels(frame: &mut Frame, menu: &LevelsMenu, rows: &[LevelRow]) {
    let area = centered(frame.area(), 80, 80);
    frame.render_widget(Clear, area);
    let block = Block::bordered()
        .title(" Log levels ")
        .title_bottom(" ←→ level · Del inherit · a add target · Esc close ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let input_height = if menu.adding.is_some() { 3 } else { 0 };
    let [table_area, input_area] =
        Layout::vertical([Constraint::Min(1), Constraint::Length(input_height)]).areas(inner);

    let header = Row::new(["Target", "Level", "Records"]).bold();
    let table_rows: Vec<Row> = rows
        .iter()
        .map(|row| {
            let level = match row.rule {
                Some(level) => Cell::from(level_name(level))
                    .fg(level_filter_color(level))
                    .bold(),
                None => Cell::from(format!("{} (inherited)", level_name(row.effective))).dim(),
            };
            let count = row
                .count
                .map(|count| count.to_string())
                .unwrap_or_default();
            Row::new([
                Cell::from(format!("{}{}", "  ".repeat(row.depth), row.label)),
                level,
                Cell::from(count),
            ])
        })
        .collect();
    let table = Table::new(
        table_rows,
        [Constraint::Min(20), Constraint::Length(17), Constraint::Length(10)],
    )
    .header(header)
    .row_highlight_style(Style::new().add_modifier(Modifier::REVERSED));
    let mut state = TableState::new().with_selected(Some(
        menu.selected
            .min(rows.len().saturating_sub(1)),
    ));
    frame.render_stateful_widget(table, table_area, &mut state);

    if let Some(input) = &menu.adding {
        render_text_input(frame, input_area, " Add target, e.g. ergot::net_stack ", input);
    }
}

fn render_input(frame: &mut Frame, title: &str, input: &TextInput) {
    let area = centered(frame.area(), 70, 0);
    let area = Rect { height: 4, ..area };
    frame.render_widget(Clear, area);
    render_text_input(frame, area, title, input);
}

/// Renders the input, with its error below it, and places the cursor at the end.
fn render_text_input(frame: &mut Frame, area: Rect, title: &str, input: &TextInput) {
    let block = Block::bordered()
        .title(title.to_string())
        .title_bottom(" Enter apply · Esc cancel ");
    let inner = block.inner(area);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);

    let mut lines = vec![Line::from(input.value.clone())];
    if let Some(error) = &input.error {
        lines.push(Line::from(error.clone()).red());
    }
    frame.render_widget(Paragraph::new(lines), inner);
    frame.set_cursor_position((
        (inner.x + input.value.chars().count() as u16).min(inner.right().saturating_sub(1)),
        inner.y,
    ));
}

/// A rect `percent_x` wide and `percent_y` high, centered in `area`.
fn centered(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let width = area.width * percent_x / 100;
    let height = area.height * percent_y / 100;
    Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    }
}

fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs();
    format!("{}:{:02}:{:02}", seconds / 3600, seconds / 60 % 60, seconds % 60)
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddrV4;
    use std::sync::Arc;

    use ioboard_shared::discovery::SerialNumber;
    use log::Level;
    use ratatui::backend::TestBackend;
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use tokio::sync::{broadcast, watch};

    use super::*;
    use crate::ioboard::discovery::IoBoardDetails;
    use crate::logging::rules::LevelRules;
    use crate::logging::store::LogStore;
    use crate::logging::store::tests::push;

    fn screen(terminal: &Terminal<TestBackend>) -> String {
        let buffer = terminal.backend().buffer();
        buffer
            .content()
            .chunks(buffer.area.width as usize)
            .map(|row| {
                row.iter()
                    .map(|cell| cell.symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn renders_status_log_and_levels() {
        let store = Arc::new(LogStore::new(LevelRules::with_global(LevelFilter::Info), 100));
        for i in 0..50 {
            push(
                &store,
                Level::Info,
                "server_cli::ioboard::discovery",
                &format!("message {}", i),
            );
        }
        let (_status_tx, status_rx) = watch::channel(vec![
            IoBoardStatus {
                name: "io1".to_string(),
                expected_serial_number: None,
                state: IoBoardState::Connected,
                board: Some(IoBoardDetails {
                    serial_number: SerialNumber([0xAB; 12]),
                    address: "192.168.1.50:5000"
                        .parse::<SocketAddrV4>()
                        .unwrap(),
                    local_address: "192.168.1.10:8100"
                        .parse::<SocketAddrV4>()
                        .unwrap(),
                    claim: ClaimStatus::Us,
                    last_seen: std::time::Instant::now(),
                }),
            },
            IoBoardStatus {
                name: "io2".to_string(),
                expected_serial_number: None,
                state: IoBoardState::Waiting,
                board: None,
            },
        ]);
        let (app_event_tx, _) = broadcast::channel(4);
        let mut app = App::new(store, status_rx, app_event_tx, Some("message 4".to_string()));
        let mut terminal = Terminal::new(TestBackend::new(140, 24)).unwrap();

        draw(&mut terminal, &mut app).unwrap();
        let screen_text = screen(&terminal);
        assert!(screen_text.contains("ABABABABABABABABABABABAB"), "{}", screen_text);
        assert!(screen_text.contains("connected"), "{}", screen_text);
        assert!(screen_text.contains("waiting"), "{}", screen_text);
        assert!(screen_text.contains("FOLLOW"), "{}", screen_text);
        assert!(screen_text.contains("message 49"), "{}", screen_text);
        assert!(!screen_text.contains("message 39"), "{}", screen_text);

        app.handle_key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE));
        app.handle_key(KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE));
        draw(&mut terminal, &mut app).unwrap();
        let screen_text = screen(&terminal);
        assert!(screen_text.contains("LOCKED"), "{}", screen_text);
        assert!(screen_text.contains("Log levels"), "{}", screen_text);
        assert!(screen_text.contains("discovery"), "{}", screen_text);
    }
}
