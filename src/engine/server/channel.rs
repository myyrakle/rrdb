use tokio::sync::oneshot::Sender;

use crate::engine::ast::SQLStatement;
use crate::engine::types::ExecuteResult;
use crate::errors::RRDBError;

#[derive(Debug)]
pub struct ChannelRequest {
    pub statement: SQLStatement,
    pub connection_id: String,
    pub response_sender: Sender<ChannelResponse>,
}

#[derive(Debug)]
pub struct ChannelResponse {
    pub result: Result<ExecuteResult, RRDBError>,
}
