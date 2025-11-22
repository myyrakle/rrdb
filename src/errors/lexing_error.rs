use super::{ErrorKind, Errors};

#[derive(Debug)]
pub struct LexingError {
    pub message: String,
    pub backtrace: std::backtrace::Backtrace,
}

impl PartialEq for LexingError {
    fn eq(&self, other: &Self) -> bool {
        self.message == other.message
    }
}

impl LexingError {
    pub fn wrap<T: ToString>(message: T) -> Errors {
        Errors::new(ErrorKind::LexingError(message.to_string()))
    }
}

impl std::error::Error for LexingError {}

impl std::fmt::Display for LexingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "lexing error: {}", self.message)
    }
}
