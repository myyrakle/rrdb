use serde::Deserialize;

use clap::Args;

/// Config options for the build system.
#[derive(Clone, Debug, Default, Deserialize, Args)]
pub struct ConfigOptions {
    #[clap(
        name = "port",
        default_value = "55555",
        long,
        short,
        help = "Port to listen on"
    )]
    pub port: u32,

    #[clap(
        name = "host",
        default_value = "0.0.0.0",
        long,
        help = "Hostname to listen on (IP or domain)"
    )]
    pub host: String,
}

#[derive(Clone, Debug, Args)]
#[clap(name = "run")]
pub struct Command {
    #[clap(flatten)]
    pub value: ConfigOptions,
}
