use super::RRDBError;

#[derive(Debug)]
pub struct LexingError {
    pub message: String,
    pub backtrace: std::backtrace::Backtrace,
}

impl LexingError {
    pub fn new<T: ToString>(message: T) -> RRDBError {
        RRDBError::LexingError(Self {
            message: message.to_string(),
            backtrace: std::backtrace::Backtrace::capture(),
        })
    }
}

impl std::error::Error for LexingError {}

impl std::fmt::Display for LexingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "lexing error: {}", self.message)
    }
}
