use super::RRDBError;

#[derive(Debug)]
pub struct IntoError {
    pub message: String,
    pub backtrace: std::backtrace::Backtrace,
}

impl IntoError {
    pub fn wrap<T: ToString>(message: T) -> RRDBError {
        RRDBError::IntoError(Self {
            message: message.to_string(),
            backtrace: std::backtrace::Backtrace::capture(),
        })
    }
}

impl std::error::Error for IntoError {}

impl std::fmt::Display for IntoError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "parsing error(into error): {}", self.message)
    }
}
