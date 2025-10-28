#![warn(clippy::all, rust_2018_idioms)]

mod app;
pub use app::OperatorUiApp;
pub mod profiling;
pub mod task;
pub mod ui_commands;
pub mod config;
pub mod runtime;

pub mod net;

pub const LOGO: &[u8] = include_bytes!("../../../assets/logos/makerpnp_icon_1_384x384.png");
