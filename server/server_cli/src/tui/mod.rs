//! A terminal ui showing the server's status above its log.
//!
//! Runs on its own thread, so rendering and input handling don't block the async runtime.  The terminal is restored
//! when the tui is dropped.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use log::error;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyEventKind};

use crate::tui::app::{App, KeyResult};

pub mod app;
pub mod log_view;
mod ui;

/// How often the tui is redrawn when there is no input.
const REDRAW_INTERVAL: Duration = Duration::from_millis(100);

/// How many log entries are written to stderr when the server exits.
pub const EXIT_LOG_ENTRIES: usize = 100;

pub struct Tui {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Tui {
    pub fn spawn(app: App) -> std::io::Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let thread = std::thread::Builder::new()
            .name("tui".to_string())
            .spawn({
                let stop = stop.clone();
                move || {
                    // also installs a panic hook that restores the terminal
                    let terminal = ratatui::init();
                    let result = run(terminal, app, &stop);
                    ratatui::restore();
                    if let Err(e) = result {
                        error!("Tui failed. error: {}", e);
                    }
                }
            })?;
        Ok(Self {
            stop,
            thread: Some(thread),
        })
    }
}

impl Drop for Tui {
    /// Stops the tui and restores the terminal.
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn run(mut terminal: DefaultTerminal, mut app: App, stop: &AtomicBool) -> std::io::Result<()> {
    while !stop.load(Ordering::Relaxed) {
        app.update();
        ui::draw(&mut terminal, &mut app)?;

        if !event::poll(REDRAW_INTERVAL)? {
            continue;
        }
        // handle all pending input before redrawing
        loop {
            if let Event::Key(key) = event::read()? {
                // e.g. on Windows, key releases are reported too
                if key.kind != KeyEventKind::Release && app.handle_key(key) == KeyResult::ForceExit {
                    ratatui::restore();
                    let _ = app
                        .store
                        .write_tail(EXIT_LOG_ENTRIES, &mut std::io::stderr());
                    eprintln!("Shutdown did not complete, exiting immediately");
                    std::process::exit(1);
                }
            }
            if !event::poll(Duration::ZERO)? {
                break;
            }
        }
    }
    Ok(())
}
