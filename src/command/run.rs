use serde::Deserialize;

use clap::Args;

/// Config options for the build system.
#[derive(Clone, Debug, Default, Deserialize, Args)]
pub struct ConfigOptions {
    #[clap(name = "config", long, help = "config file path")]
    pub config: Option<String>,
}

#[derive(Clone, Debug, Args)]
#[clap(name = "run")]
pub struct Command {
    #[clap(flatten)]
    pub value: ConfigOptions,
}
