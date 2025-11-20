use super::{ErrorKind, Errors};

#[derive(Debug)]
pub struct ExecuteError {
    pub message: String,
    pub backtrace: std::backtrace::Backtrace,
}

impl PartialEq for ExecuteError {
    fn eq(&self, other: &Self) -> bool {
        self.message == other.message
    }
}

impl ExecuteError {
    pub fn wrap<T: ToString>(message: T) -> Errors {
        Errors::new(ErrorKind::ExecuteError(message.to_string()))
    }
}

impl std::error::Error for ExecuteError {}

impl std::fmt::Display for ExecuteError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::ErrorKind;

    #[test]
    fn test_execute_error_display() {
        let error = Errors::new(ErrorKind::ExecuteError("test".to_string()));

        assert!(error.to_string().contains("test"));
    }
}
