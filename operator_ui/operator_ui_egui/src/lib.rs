#![warn(clippy::all, rust_2018_idioms)]

// TODO replace these with dynamic configuration
//const REMOTE_ADDR: &str = "127.0.0.1:5000";
const REMOTE_ADDR: &str = "192.168.18.41:8001";
//const LOCAL_ADDR: &str = "0.0.0.0:5001";
const LOCAL_ADDR: &str = "192.168.18.41:8002";

// TODO remove `TARGET_FPS` it's value should come from the per-camera FPS configuration on the
//      server via camera discovery
const TARGET_FPS: f32 = 30.0;
const SCHEDULED_FPS_MIN: f32 = 5.0;
const SCHEDULED_FPS_MAX: f32 = 30.0;

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

pub mod fps_stats;

pub const LOGO: &[u8] = include_bytes!("../../../assets/logos/makerpnp_icon_1_384x384.png");

pub mod events;
