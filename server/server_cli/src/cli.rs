use std::path::PathBuf;

use clap::Parser;

/// Example app that requires a config file
#[derive(Parser, Debug)]
#[command(name = "server_cli", version, about = "MakerPnP - Server")]
pub struct Args {
    /// Path to the config file
    #[arg(short = 'c', long = "config", value_name = "PATH", default_value_os = "config.ron")]
    pub config: PathBuf,

    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(
        short = 'v',
        long = "verbose",
        action = clap::ArgAction::Count
    )]
    pub verbosity_level: u8,

    /// Log to the console instead of showing the terminal ui.  The terminal ui is only shown when stdin and stdout are
    /// terminals.
    #[arg(long = "no-tui")]
    pub no_tui: bool,

    /// The number of log entries kept by the terminal ui, can be changed in the terminal ui.
    #[arg(long = "log-capacity", value_name = "ENTRIES", default_value_t = 10_000)]
    pub log_capacity: usize,
}
