use super::RRDBError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntoError {
    pub message: String,
}

impl IntoError {
    pub fn new<T: ToString>(message: T) -> RRDBError {
        RRDBError::IntoError(Self {
            message: message.to_string(),
        })
    }
}

impl std::error::Error for IntoError {}

impl std::fmt::Display for IntoError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "parsing error(into error): {}", self.message)
    }
}
