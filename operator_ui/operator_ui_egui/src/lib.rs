#![warn(clippy::all, rust_2018_idioms)]

mod app;
pub use app::OperatorUiApp;
pub mod config;
pub mod profiling;
pub mod runtime;
pub mod task;
pub mod ui_commands;

pub mod net;

pub mod workspace;

pub mod ui_common;

pub const LOGO: &[u8] = include_bytes!("../../../assets/logos/makerpnp_icon_1_384x384.png");
