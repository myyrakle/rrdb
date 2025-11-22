use super::{ErrorKind, Errors};

#[derive(Debug)]
pub struct ExecuteError {
    pub message: String,
    pub backtrace: std::backtrace::Backtrace,
}

impl PartialEq for ExecuteError {
    fn eq(&self, other: &Self) -> bool {
        self.message == other.message
    }
}

impl ExecuteError {
    pub fn wrap<T: ToString>(message: T) -> Errors {
        Errors::new(ErrorKind::ExecuteError(message.to_string()))
    }
}

impl std::error::Error for ExecuteError {}

impl std::fmt::Display for ExecuteError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "{}", self.message)
    }
}
