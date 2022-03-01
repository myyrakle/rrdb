use crate::command::init::InitCommand;

#[derive(clap::Subcommand, Debug)]
pub enum SubCommands {
    /// 설정 파일 및 저장공간 초기화
    Init(InitCommand),
    /// DB 서버 실행
    Run,
    /// DB 클라이언트 실행
    Client,
}
