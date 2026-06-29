use serde::Deserialize;

use clap::Args;

/// Config options for initialization.
#[derive(Clone, Debug, Default, Deserialize, Args)]
pub struct ConfigOptions {
    /// RRDB 파일이 세팅될 기본 디렉터리
    #[clap(name = "base-path", long, short)]
    pub base_path: Option<String>,
}

#[derive(Clone, Debug, Args)]
#[clap(name = "init")]
pub struct Command {
    #[clap(flatten)]
    pub init: ConfigOptions,
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use crate::command::{Command, SubCommand};

    #[test]
    fn init_accepts_base_path() {
        let command = Command::parse_from(["rrdb", "init", "--base-path", "local-test"]);

        match command.action {
            SubCommand::Init(init) => {
                assert_eq!(init.init.base_path, Some("local-test".to_string()));
            }
            _ => panic!("expected init command"),
        }
    }

    #[test]
    fn init_rejects_config_path() {
        let result = Command::try_parse_from(["rrdb", "init", "--config-path", "local-test"]);

        assert!(result.is_err());
    }
}
