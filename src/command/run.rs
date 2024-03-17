use serde::Deserialize;

use clap::Args;

/// Config options for the build system.
#[derive(Clone, Debug, Default, Deserialize, Args)]
pub struct ConfigOptions {
    /// 포트
    #[clap(name = "port", long)]
    pub port: Option<u32>,

    /// 호스트
    #[clap(name = "host", long)]
    pub host: Option<String>,
}

#[derive(Clone, Debug, Args)]
#[clap(name = "run")]
pub struct Command {
    #[clap(flatten)]
    pub value: ConfigOptions,
}
