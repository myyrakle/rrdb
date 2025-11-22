use super::{ErrorKind, Errors};

#[derive(Debug)]
pub struct ParsingError {
    pub message: String,
    pub backtrace: std::backtrace::Backtrace,
}

impl PartialEq for ParsingError {
    fn eq(&self, other: &Self) -> bool {
        self.message == other.message
    }
}

impl ParsingError {
    pub fn wrap<T: ToString>(message: T) -> Errors {
        Errors::new(ErrorKind::ParsingError(message.to_string()))
    }
}

impl std::error::Error for ParsingError {}

impl std::fmt::Display for ParsingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "parsing error: {}", self.message)
    }
}
