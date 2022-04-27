use serde::Deserialize;

use clap::Args;

/// Config options for the build system.
#[derive(Clone, Debug, Default, Deserialize, Args)]
pub struct ConfigOptionsRun {
    /// 파일이 세팅될 경로
    #[clap(long, short)]
    pub port: Option<u32>,
}

#[derive(Clone, Debug, Args)]
#[clap(name = "run")]
pub struct RunCommand {
    #[clap(flatten)]
    pub value: ConfigOptionsRun,
}
