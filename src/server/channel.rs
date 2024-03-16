use std::error::Error;

use tokio::sync::oneshot::Sender;

use crate::{ast::SQLStatement, executor::result::ExecuteResult};

#[derive(Debug)]
pub struct ChannelRequest {
    pub statement: SQLStatement,
    pub response_sender: Sender<ChannelResponse>,
}

#[derive(Debug)]
pub struct ChannelResponse {
    pub result: Result<ExecuteResult, Box<dyn Error + Send>>,
}
