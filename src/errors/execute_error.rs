use super::RRDBError;

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
    pub fn wrap<T: ToString>(message: T) -> RRDBError {
        RRDBError::ExecuteError(Self {
            message: message.to_string(),
            backtrace: std::backtrace::Backtrace::capture(),
        })
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

    #[test]
    fn test_execute_error_eq() {
        let error1 = ExecuteError::wrap("test");
        let error2 = ExecuteError::wrap("test");
        assert_eq!(error1, error2);
    }

    #[test]
    fn test_execute_error_display() {
        let error = ExecuteError::wrap("test");

        assert!(error.to_string().contains("test"));
    }
}
