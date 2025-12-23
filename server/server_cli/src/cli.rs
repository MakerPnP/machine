use std::path::PathBuf;
use clap::Parser;

/// Example app that requires a config file
#[derive(Parser, Debug)]
#[command(
    name = "server_cli",
    version,
    about = "MakerPnP - Server"
)]
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
}