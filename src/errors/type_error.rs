use super::RRDBError;

#[derive(Debug)]
pub struct TypeError {
    pub message: String,
    pub backtrace: std::backtrace::Backtrace,
}

impl TypeError {
    pub fn new<T: ToString>(message: T) -> RRDBError {
        RRDBError::TypeError(Self {
            message: message.to_string(),
            backtrace: std::backtrace::Backtrace::capture(),
        })
    }
}

impl std::error::Error for TypeError {}

impl std::fmt::Display for TypeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "parsing error: {}", self.message)
    }
}
