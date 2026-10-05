//! The tui's state and input handling.

use std::collections::BTreeSet;
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use log::LevelFilter;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use regex::{Regex, RegexBuilder};
use tokio::sync::{broadcast, watch};

use crate::AppEvent;
#[cfg(feature = "machine-vision")]
use crate::camera::status::CameraStatus;
use crate::ioboard::discovery::IoBoardStatus;
use crate::logging::store::{LogStore, LogStoreInner};
use crate::tui::log_view::LogView;

const HOST_ADDRESSES_REFRESH_INTERVAL: Duration = Duration::from_secs(5);

/// From least to most verbose.
const LEVELS: [LevelFilter; 6] = [
    LevelFilter::Off,
    LevelFilter::Error,
    LevelFilter::Warn,
    LevelFilter::Info,
    LevelFilter::Debug,
    LevelFilter::Trace,
];

pub struct App {
    pub store: Arc<LogStore>,
    pub io_boards: watch::Receiver<Vec<IoBoardStatus>>,
    #[cfg(feature = "machine-vision")]
    pub cameras: watch::Receiver<Vec<CameraStatus>>,
    app_event_tx: broadcast::Sender<AppEvent>,
    app_event_rx: broadcast::Receiver<AppEvent>,
    pub shutting_down: bool,
    pub started: Instant,
    /// Interface name and address, for every non-loopback IPv4 interface.
    pub host_addresses: Vec<(String, Ipv4Addr)>,
    host_addresses_refreshed: Option<Instant>,
    pub log_view: LogView,
    /// The source of the current message filter.
    pub filter: String,
    pub popup: Option<Popup>,
}

pub enum Popup {
    Levels(LevelsMenu),
    Filter(TextInput),
    Capacity(TextInput),
}

pub struct LevelsMenu {
    pub selected: usize,
    /// The target being entered, when adding a target.
    pub adding: Option<TextInput>,
}

#[derive(Default)]
pub struct TextInput {
    pub value: String,
    pub error: Option<String>,
}

impl TextInput {
    fn new(value: String) -> Self {
        Self { value, error: None }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .contains(KeyModifiers::CONTROL) =>
            {
                self.value.push(c)
            }
            KeyCode::Backspace => {
                self.value.pop();
            }
            _ => {}
        }
    }
}

/// A row in the levels menu.
pub struct LevelRow {
    /// `None` for the global rule.
    pub target: Option<String>,
    pub depth: usize,
    pub label: String,
    /// The row's own rule, if any.
    pub rule: Option<LevelFilter>,
    /// The level that applies to the row's target.
    pub effective: LevelFilter,
    /// How many records the target emitted, if it has emitted any.
    pub count: Option<u64>,
}

/// The rows of the levels menu: the global rule, then every target and its parent modules, as a tree.
pub fn level_rows(store: &LogStoreInner) -> Vec<LevelRow> {
    let rules = store.rules();

    // sorted by segment, so children follow their parent
    let mut paths: BTreeSet<Vec<&str>> = BTreeSet::new();
    for target in store
        .targets()
        .keys()
        .map(String::as_str)
        .chain(rules.targets())
    {
        let segments: Vec<&str> = target.split("::").collect();
        for length in 1..=segments.len() {
            paths.insert(segments[..length].to_vec());
        }
    }

    let global = LevelRow {
        target: None,
        depth: 0,
        label: "(default)".to_string(),
        rule: rules.rule(None),
        effective: rules.level_for(""),
        count: None,
    };
    let targets = paths.into_iter().map(|segments| {
        let target = segments.join("::");
        LevelRow {
            depth: segments.len() - 1,
            label: segments.last().unwrap().to_string(),
            rule: rules.rule(Some(&target)),
            effective: rules.level_for(&target),
            count: store.targets().get(&target).copied(),
            target: Some(target),
        }
    });

    std::iter::once(global)
        .chain(targets)
        .collect()
}

/// What the tui should do after handling a key.
#[derive(Debug, PartialEq)]
pub enum KeyResult {
    Continue,
    /// Quit was requested again while shutting down, exit immediately.
    ForceExit,
}

impl App {
    pub fn new(
        store: Arc<LogStore>,
        io_boards: watch::Receiver<Vec<IoBoardStatus>>,
        #[cfg(feature = "machine-vision")] cameras: watch::Receiver<Vec<CameraStatus>>,
        app_event_tx: broadcast::Sender<AppEvent>,
        filter: Option<String>,
    ) -> Self {
        let mut app = Self {
            store,
            io_boards,
            #[cfg(feature = "machine-vision")]
            cameras,
            app_event_rx: app_event_tx.subscribe(),
            app_event_tx,
            shutting_down: false,
            started: Instant::now(),
            host_addresses: Vec::new(),
            host_addresses_refreshed: None,
            log_view: LogView::new(),
            filter: String::new(),
            popup: None,
        };
        if let Some(filter) = filter {
            match build_regex(&filter) {
                Ok(regex) => {
                    app.log_view.set_regex(regex);
                    app.filter = filter;
                }
                Err(e) => log::warn!(
                    "RUST_LOG: invalid filter, ignoring it. filter: '{}', error: {}",
                    filter,
                    e
                ),
            }
        }
        app
    }

    /// Updates state that doesn't depend on input.
    pub fn update(&mut self) {
        while let Ok(event) = self.app_event_rx.try_recv() {
            match event {
                AppEvent::Shutdown => self.shutting_down = true,
            }
        }

        if self
            .host_addresses_refreshed
            .is_none_or(|refreshed| refreshed.elapsed() >= HOST_ADDRESSES_REFRESH_INTERVAL)
        {
            self.host_addresses = host_addresses();
            self.host_addresses_refreshed = Some(Instant::now());
        }
    }

    fn request_shutdown(&mut self) -> KeyResult {
        if self.shutting_down {
            return KeyResult::ForceExit;
        }
        self.shutting_down = true;
        let _ = self
            .app_event_tx
            .send(AppEvent::Shutdown);
        KeyResult::Continue
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> KeyResult {
        let ctrl = key
            .modifiers
            .contains(KeyModifiers::CONTROL);
        if ctrl && key.code == KeyCode::Char('c') {
            return self.request_shutdown();
        }

        match self.popup.take() {
            None => return self.handle_main_key(key, ctrl),
            Some(Popup::Levels(menu)) => self.popup = self.handle_levels_key(menu, key),
            Some(Popup::Filter(input)) => self.popup = self.handle_filter_key(input, key),
            Some(Popup::Capacity(input)) => self.popup = self.handle_capacity_key(input, key),
        }
        KeyResult::Continue
    }

    fn handle_main_key(&mut self, key: KeyEvent, ctrl: bool) -> KeyResult {
        match key.code {
            KeyCode::Up => self.log_view.up(1),
            KeyCode::Down => self.log_view.down(1),
            KeyCode::PageUp => self.log_view.page_up(),
            KeyCode::PageDown => self.log_view.page_down(),
            KeyCode::Home if ctrl => self.log_view.first(),
            KeyCode::End if ctrl => self.log_view.follow(),
            KeyCode::F(2) => {
                self.popup = Some(Popup::Levels(LevelsMenu {
                    selected: 0,
                    adding: None,
                }))
            }
            KeyCode::F(3) | KeyCode::Char('/') => self.popup = Some(Popup::Filter(TextInput::new(self.filter.clone()))),
            KeyCode::F(4) => {
                let capacity = self.store.lock().capacity();
                self.popup = Some(Popup::Capacity(TextInput::new(capacity.to_string())))
            }
            KeyCode::Char('q') => return self.request_shutdown(),
            _ => {}
        }
        KeyResult::Continue
    }

    fn handle_levels_key(&mut self, mut menu: LevelsMenu, key: KeyEvent) -> Option<Popup> {
        if let Some(mut input) = menu.adding.take() {
            match key.code {
                KeyCode::Esc => {}
                KeyCode::Enter => {
                    let target = input.value.trim();
                    if !target.is_empty() {
                        let mut store = self.store.lock();
                        store.add_target(target);
                        if let Some(index) = level_rows(&store)
                            .iter()
                            .position(|row| row.target.as_deref() == Some(target))
                        {
                            menu.selected = index;
                        }
                    }
                }
                _ => {
                    input.handle_key(key);
                    menu.adding = Some(input);
                }
            }
            return Some(Popup::Levels(menu));
        }

        let mut store = self.store.lock();
        let rows = level_rows(&store);
        menu.selected = menu.selected.min(rows.len() - 1);
        let row = &rows[menu.selected];

        let change_level = |store: &mut LogStoreInner, step: isize| {
            let current = row.rule.unwrap_or(row.effective);
            let index = LEVELS
                .iter()
                .position(|level| *level == current)
                .unwrap_or_default();
            let index = index
                .saturating_add_signed(step)
                .min(LEVELS.len() - 1);
            store.set_rule(row.target.as_deref(), Some(LEVELS[index]));
        };

        match key.code {
            KeyCode::Esc | KeyCode::F(2) => return None,
            KeyCode::Up => menu.selected = menu.selected.saturating_sub(1),
            KeyCode::Down => menu.selected = (menu.selected + 1).min(rows.len() - 1),
            KeyCode::PageUp => menu.selected = menu.selected.saturating_sub(10),
            KeyCode::PageDown => menu.selected = (menu.selected + 10).min(rows.len() - 1),
            KeyCode::Home => menu.selected = 0,
            KeyCode::End => menu.selected = rows.len() - 1,
            KeyCode::Left => change_level(&mut store, -1),
            KeyCode::Right => change_level(&mut store, 1),
            KeyCode::Delete | KeyCode::Backspace => store.set_rule(row.target.as_deref(), None),
            KeyCode::Char('a') => menu.adding = Some(TextInput::default()),
            _ => {}
        }
        Some(Popup::Levels(menu))
    }

    fn handle_filter_key(&mut self, mut input: TextInput, key: KeyEvent) -> Option<Popup> {
        match key.code {
            KeyCode::Esc => None,
            KeyCode::Enter => match build_regex(&input.value) {
                Ok(regex) => {
                    self.log_view.set_regex(regex);
                    self.filter = input.value;
                    None
                }
                Err(e) => {
                    input.error = Some(e.to_string());
                    Some(Popup::Filter(input))
                }
            },
            _ => {
                input.handle_key(key);
                input.error = None;
                Some(Popup::Filter(input))
            }
        }
    }

    fn handle_capacity_key(&mut self, mut input: TextInput, key: KeyEvent) -> Option<Popup> {
        match key.code {
            KeyCode::Esc => None,
            KeyCode::Enter => match input.value.trim().parse::<usize>() {
                Ok(capacity) if capacity > 0 => {
                    self.store.lock().set_capacity(capacity);
                    None
                }
                _ => {
                    input.error = Some("enter a number greater than 0".to_string());
                    Some(Popup::Capacity(input))
                }
            },
            _ => {
                input.handle_key(key);
                input.error = None;
                Some(Popup::Capacity(input))
            }
        }
    }
}

/// Matches messages case-insensitively, `None` for an empty filter.
fn build_regex(filter: &str) -> Result<Option<Regex>, regex::Error> {
    if filter.is_empty() {
        return Ok(None);
    }
    RegexBuilder::new(filter)
        .case_insensitive(true)
        .build()
        .map(Some)
}

fn host_addresses() -> Vec<(String, Ipv4Addr)> {
    let Ok(interfaces) = if_addrs::get_if_addrs() else {
        return Vec::new();
    };
    let mut addresses: Vec<(String, Ipv4Addr)> = interfaces
        .into_iter()
        .filter(|interface| !interface.is_loopback())
        .filter_map(|interface| match interface.ip() {
            std::net::IpAddr::V4(ip) => Some((interface.name, ip)),
            std::net::IpAddr::V6(_) => None,
        })
        .collect();
    addresses.sort_by_key(|(_, ip)| *ip);
    addresses
}

#[cfg(test)]
mod tests {
    use log::Level;

    use super::*;
    use crate::logging::rules::LevelRules;
    use crate::logging::store::tests::push;

    #[test]
    fn level_rows_form_a_tree() {
        let store = LogStore::new(LevelRules::parse("warn,ergot=debug,other=off").rules, 10);
        push(&store, Level::Warn, "server_cli::ioboard::discovery", "1");
        push(&store, Level::Warn, "server_cli::operator", "2");
        push(&store, Level::Warn, "server_cli1", "3");
        push(&store, Level::Warn, "ergot::net_stack", "4");
        push(&store, Level::Warn, "ergot::net_stack", "5");

        let rows = level_rows(&store.lock());
        let rows: Vec<(usize, &str, Option<LevelFilter>, LevelFilter, Option<u64>)> = rows
            .iter()
            .map(|row| (row.depth, row.label.as_str(), row.rule, row.effective, row.count))
            .collect();

        use LevelFilter::*;
        assert_eq!(
            rows,
            [
                (0, "(default)", Some(Warn), Warn, None),
                (0, "ergot", Some(Debug), Debug, None),
                (1, "net_stack", None, Debug, Some(2)),
                (0, "other", Some(Off), Off, None),
                (0, "server_cli", None, Warn, None),
                (1, "ioboard", None, Warn, None),
                (2, "discovery", None, Warn, Some(1)),
                (1, "operator", None, Warn, Some(1)),
                (0, "server_cli1", None, Warn, Some(1)),
            ]
        );
    }
}
