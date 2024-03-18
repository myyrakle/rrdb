use serde::Deserialize;

use clap::Args;

/// Config options for the build system.
#[derive(Clone, Debug, Default, Deserialize, Args)]
pub struct ConfigOptionsInit {}

#[derive(Clone, Debug, Args)]
#[clap(name = "init")]
pub struct InitCommand {
    #[clap(flatten)]
    pub init: ConfigOptionsInit,
}
