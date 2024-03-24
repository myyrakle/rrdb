use super::RRDBError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LexingError {
    pub message: String,
}

impl LexingError {
    pub fn new<T: ToString>(message: T) -> RRDBError {
        RRDBError::LexingError(Self {
            message: message.to_string(),
        })
    }
}

impl std::error::Error for LexingError {}

impl std::fmt::Display for LexingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "lexing error: {}", self.message)
    }
}
