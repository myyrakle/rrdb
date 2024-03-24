use super::RRDBError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeError {
    pub message: String,
}

impl TypeError {
    pub fn new<T: ToString>(message: T) -> RRDBError {
        RRDBError::TypeError(Self {
            message: message.to_string(),
        })
    }
}

impl std::error::Error for TypeError {}

impl std::fmt::Display for TypeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "parsing error: {}", self.message)
    }
}
