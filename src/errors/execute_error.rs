use std::error::Error;

use super::RRDBError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecuteError {
    pub message: String,
}

impl ExecuteError {
    pub fn new<T: ToString>(message: T) -> RRDBError {
        RRDBError::ExecuteError(Self {
            message: message.to_string(),
        })
    }
}

impl std::error::Error for ExecuteError {}

impl std::fmt::Display for ExecuteError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "{}", self.message)
    }
}
