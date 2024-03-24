use super::RRDBError;

#[derive(Debug)]
pub struct ExecuteError {
    pub message: String,
    pub backtrace: std::backtrace::Backtrace,
}

impl ExecuteError {
    pub fn new<T: ToString>(message: T) -> RRDBError {
        RRDBError::ExecuteError(Self {
            message: message.to_string(),
            backtrace: std::backtrace::Backtrace::capture(),
        })
    }
}

impl std::error::Error for ExecuteError {}

impl std::fmt::Display for ExecuteError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "{}", self.message)
    }
}
