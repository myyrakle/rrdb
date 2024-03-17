pub mod init;
pub mod run;

use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Command {
    #[clap(subcommand)]
    pub action: SubCommand,
}

#[derive(clap::Subcommand, Debug)]
pub enum SubCommand {
    /// 설정 파일 및 저장공간 초기화
    Init(init::Command),
    /// DB 서버 실행
    Run(run::Command),
    /// DB 클라이언트 실행
    Client,
}
