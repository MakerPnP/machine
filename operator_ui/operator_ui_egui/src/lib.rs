#![warn(clippy::all, rust_2018_idioms)]

const LOCAL_ADDR: &str = "0.0.0.0:8002";

/// The server address, normally the server runs on the same machine.
/// Override with the `MAKERPNP_SERVER_ADDR` environment variable, e.g. `MAKERPNP_SERVER_ADDR=192.168.18.60:8001`.
const DEFAULT_SERVER_ADDR: &str = "127.0.0.1:8001";
const SERVER_ADDR_ENV_VAR: &str = "MAKERPNP_SERVER_ADDR";

// TODO remove `TARGET_FPS` it's value should come from the per-camera FPS configuration on the
//      server via camera discovery
const TARGET_FPS: f32 = 30.0;
const SCHEDULED_FPS_MIN: f32 = 5.0;
const SCHEDULED_FPS_MAX: f32 = 60.0;

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
