use super::RRDBError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsingError {
    pub message: String,
}

impl ParsingError {
    pub fn new<T: ToString>(message: T) -> RRDBError {
        RRDBError::ParsingError(Self {
            message: message.to_string(),
        })
    }
}

impl std::error::Error for ParsingError {}

impl std::fmt::Display for ParsingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "parsing error: {}", self.message)
    }
}
