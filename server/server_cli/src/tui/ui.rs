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

#[cfg(feature = "machine-vision")]
use crate::camera::status::{CameraSourceStatus, CameraState, CameraStatus};
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

/// The io boards and cameras, copied so the channels aren't borrowed while rendering.
struct StatusSnapshot {
    io_boards: Vec<IoBoardStatus>,
    #[cfg(feature = "machine-vision")]
    cameras: Vec<CameraStatus>,
}

impl StatusSnapshot {
    fn io_boards_height(&self) -> u16 {
        // header and at least one row
        1 + self.io_boards.len().max(1) as u16
    }

    #[cfg(feature = "machine-vision")]
    fn cameras_height(&self) -> u16 {
        // a blank line separating them from the io boards, a header and at least one row
        1 + 1 + self.cameras.len().max(1) as u16
    }

    #[cfg(not(feature = "machine-vision"))]
    fn cameras_height(&self) -> u16 {
        0
    }

    /// The height of the status panel's contents.
    fn height(&self) -> u16 {
        // the host line, then the tables
        1 + self.io_boards_height() + self.cameras_height()
    }
}

fn layout(area: Rect, status: &StatusSnapshot) -> Areas {
    let status_height = 2 + status.height();
    let [status, log, help] = Layout::vertical([
        Constraint::Length(status_height),
        Constraint::Min(3),
        Constraint::Length(1),
    ])
    .areas(area);
    Areas { status, log, help }
}

pub fn draw<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<(), B::Error> {
    let status = StatusSnapshot {
        io_boards: app.io_boards.borrow().clone(),
        #[cfg(feature = "machine-vision")]
        cameras: app.cameras.borrow().clone(),
    };

    let size = terminal.size()?;
    let areas = layout(Rect::new(0, 0, size.width, size.height), &status);
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
        let areas = layout(frame.area(), &status);
        render_status(frame, areas.status, app, &status);
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

fn render_status(frame: &mut Frame, area: Rect, app: &App, status: &StatusSnapshot) {
    let block = Block::bordered().title(" Status ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [host_area, io_boards_area, cameras_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(status.io_boards_height()),
        Constraint::Length(status.cameras_height()),
    ])
    .areas(inner);

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
    let rows: Vec<Row> = if status.io_boards.is_empty() {
        vec![Row::new([Cell::from("no io boards configured").dim()])]
    } else {
        status
            .io_boards
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
    frame.render_widget(Table::new(rows, widths).header(header), io_boards_area);

    #[cfg(feature = "machine-vision")]
    render_cameras(frame, cameras_area, &status.cameras);
    #[cfg(not(feature = "machine-vision"))]
    let _ = cameras_area;
}

#[cfg(feature = "machine-vision")]
fn render_cameras(frame: &mut Frame, area: Rect, cameras: &[CameraStatus]) {
    // the first line separates the cameras from the io boards
    let [_, area] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);
    let header = Row::new(["Camera", "Name", "Source", "Device", "Width", "Height", "FPS", "Status"]).bold();
    let rows: Vec<Row> = if cameras.is_empty() {
        vec![Row::new([Cell::from("no cameras configured or detected").dim()])]
    } else {
        cameras.iter().map(camera_row).collect()
    };
    let widths = [
        Constraint::Length(6),
        Constraint::Length(24),
        Constraint::Length(7),
        Constraint::Min(20),
        Constraint::Length(6),
        Constraint::Length(6),
        Constraint::Length(5),
        Constraint::Length(9),
    ];
    frame.render_widget(Table::new(rows, widths).header(header), area);
}

#[cfg(feature = "machine-vision")]
fn camera_row(status: &CameraStatus) -> Row<'static> {
    let not_applicable = || Cell::from("N/A").dim();
    let (identifier, name, width, height, fps) = match &status.configured {
        Some(configured) => (
            Cell::from(configured.identifier.to_string()),
            Cell::from(configured.name.clone()),
            Cell::from(configured.width.to_string()),
            Cell::from(configured.height.to_string()),
            Cell::from(configured.fps.to_string()),
        ),
        None => (
            Cell::from("-").dim(),
            not_applicable(),
            not_applicable(),
            not_applicable(),
            not_applicable(),
        ),
    };
    let (source, device) = match &status.source {
        Some(CameraSourceStatus::OpenCV { index }) => (Cell::from("OpenCV"), Cell::from(format!("index {}", index))),
        Some(CameraSourceStatus::MediaRS { device_id, name }) => {
            let mut device = vec![Span::from(device_id.clone())];
            if let Some(name) = name {
                device.push(Span::from(format!(" ({})", name)).dim());
            }
            (Cell::from("MediaRS"), Cell::from(Line::from(device)))
        }
        None => (Cell::from("-").dim(), Cell::from("no supported source").dim()),
    };
    let state = match status.state {
        CameraState::New => Cell::from("new").cyan(),
        CameraState::Matched => Cell::from("matched").green(),
        CameraState::Missing => Cell::from("missing").red(),
        CameraState::Streaming => Cell::from("streaming").magenta().bold(),
    };
    Row::new([identifier, name, source, device, width, height, fps, state])
}

fn io_board_row(status: &IoBoardStatus) -> Row<'static> {
    let serial = match (&status.board, status.expected_serial_number) {
        (Some(board), _) => Cell::from(board.serial_number.to_string()),
        (None, Some(expected)) => Cell::from(expected.to_string()).dim(),
        (None, None) => Cell::from("any").dim(),
    };
    // an entry without a board has nothing to be offline
    let online = status
        .board
        .as_ref()
        .is_none_or(|board| board.online);
    let (state, color) = match (online, status.state) {
        (true, IoBoardState::Waiting) => ("waiting", Color::Yellow),
        (true, IoBoardState::Claiming) => ("claiming", Color::Cyan),
        (true, IoBoardState::ReleasingStale) => ("releasing stale", Color::Magenta),
        (true, IoBoardState::Connected) => ("connected", Color::Green),
        (false, _) => ("offline", Color::Red),
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
    #[cfg(feature = "machine-vision")]
    use crate::camera::status::ConfiguredCamera;
    use crate::ioboard::discovery::IoBoardDetails;
    use crate::logging::rules::LevelRules;
    use crate::logging::store::LogStore;
    use crate::logging::store::tests::push;
    #[cfg(feature = "machine-vision")]
    use operator_shared::camera::CameraIdentifier;

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
                    online: true,
                }),
            },
            IoBoardStatus {
                name: "io2".to_string(),
                expected_serial_number: None,
                state: IoBoardState::Waiting,
                board: None,
            },
        ]);
        #[cfg(feature = "machine-vision")]
        let (_cameras_tx, cameras_rx) = watch::channel(vec![
            CameraStatus {
                configured: Some(ConfiguredCamera {
                    identifier: CameraIdentifier::new(0),
                    name: "Top camera".to_string(),
                    width: 800,
                    height: 600,
                    fps: 30.0,
                }),
                source: Some(CameraSourceStatus::OpenCV { index: 0 }),
                state: CameraState::Streaming,
            },
            CameraStatus {
                configured: None,
                source: Some(CameraSourceStatus::MediaRS {
                    device_id: "usb-1.3".to_string(),
                    name: Some("USB2.0 Camera".to_string()),
                }),
                state: CameraState::New,
            },
        ]);
        let (app_event_tx, _) = broadcast::channel(4);
        let mut app = App::new(
            store,
            status_rx,
            #[cfg(feature = "machine-vision")]
            cameras_rx,
            app_event_tx,
            Some("message 4".to_string()),
        );
        let mut terminal = Terminal::new(TestBackend::new(140, 30)).unwrap();

        draw(&mut terminal, &mut app).unwrap();
        let screen_text = screen(&terminal);
        #[cfg(feature = "machine-vision")]
        {
            assert!(screen_text.contains("Top camera"), "{}", screen_text);
            assert!(screen_text.contains("streaming"), "{}", screen_text);
            assert!(screen_text.contains("usb-1.3 (USB2.0 Camera)"), "{}", screen_text);
            assert!(screen_text.contains("N/A"), "{}", screen_text);
        }
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
