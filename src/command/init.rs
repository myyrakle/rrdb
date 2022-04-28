use serde::Deserialize;

use clap::Args;

/// Config options for the build system.
#[derive(Clone, Debug, Default, Deserialize, Args)]
pub struct ConfigOptionsInit {
    /// 파일이 세팅될 경로
    #[clap(long, short)]
    pub config_path: Option<String>,
}

#[derive(Clone, Debug, Args)]
#[clap(name = "init")]
pub struct InitCommand {
    #[clap(flatten)]
    pub init: ConfigOptionsInit,
}
