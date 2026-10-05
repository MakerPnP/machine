//! Logging setup.
//!
//! In plain mode `env_logger` logs to the console.  In tui mode records are collected into a [`LogStore`] instead, using
//! the same `RUST_LOG` syntax, so they can be displayed and filtered by the tui.

use std::sync::Arc;

use log::{LevelFilter, warn};
use tracing_subscriber::layer::SubscriberExt;

use crate::logging::rules::LevelRules;
use crate::logging::store::LogStore;

pub mod rules;
pub mod store;

/// The global level used when `RUST_LOG` is not set.
fn verbosity_level(verbosity_level: u8) -> LevelFilter {
    match verbosity_level {
        0 => LevelFilter::Warn,
        1 => LevelFilter::Info,
        2 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    }
}

/// Logs to the console using `env_logger`.
pub fn init_plain(verbosity: u8) {
    let mut builder = env_logger::Builder::from_default_env();

    // Only override the default filter if RUST_LOG is NOT set
    if std::env::var_os("RUST_LOG").is_none() {
        builder.filter_level(verbosity_level(verbosity));
    }

    builder.init();

    console_subscriber::init();
}

/// Collects log records into a store, returning the store and the message filter from `RUST_LOG`, if any.
pub fn init_collector(verbosity: u8, capacity: usize) -> (Arc<LogStore>, Option<String>) {
    let spec = match std::env::var("RUST_LOG") {
        Ok(spec) => LevelRules::parse(&spec),
        Err(_) => rules::Spec {
            rules: LevelRules::with_global(verbosity_level(verbosity)),
            ..Default::default()
        },
    };

    let store = Arc::new(LogStore::new(spec.rules, capacity));
    store
        .install()
        .expect("no other logger installed");

    for error in spec.errors {
        warn!("RUST_LOG: {}", error);
    }

    // `console_subscriber::init` also logs tracing events to stdout, which would corrupt the tui, so only the console
    // layer is installed.
    let console_layer = console_subscriber::ConsoleLayer::builder()
        .with_default_env()
        .spawn();
    tracing::subscriber::set_global_default(tracing_subscriber::registry().with(console_layer))
        .expect("no other tracing subscriber installed");

    (store, spec.filter)
}
