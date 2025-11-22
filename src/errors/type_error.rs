use super::{ErrorKind, Errors};

#[derive(Debug)]
pub struct TypeError {
    pub message: String,
    pub backtrace: std::backtrace::Backtrace,
}

impl PartialEq for TypeError {
    fn eq(&self, other: &Self) -> bool {
        self.message == other.message
    }
}

impl TypeError {
    pub fn wrap<T: ToString>(message: T) -> Errors {
        Errors::new(ErrorKind::TypeError(message.to_string()))
    }
}

impl std::error::Error for TypeError {}

impl std::fmt::Display for TypeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "type error: {}", self.message)
    }
}
