use std::sync::{Arc, Mutex};

use crate::lib::{ast::predule::SQLStatement, executor::result::ExecuteResult};

#[derive(Clone, Debug)]
pub struct ChannelRequest {
    pub statement: SQLStatement,
    pub execute_result: Arc<Mutex<Option<ExecuteResult>>>,
}

#[derive(Clone, Debug)]
pub struct ChannelResponse {}
