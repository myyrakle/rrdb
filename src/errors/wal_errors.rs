use super::RRDBError;

#[derive(Debug)]
pub struct WALError {
    pub message: String,
    pub backtrace: std::backtrace::Backtrace,
}

impl PartialEq for WALError {
    fn eq(&self, other: &Self) -> bool {
        self.message == other.message
    }
}

impl WALError {
    pub fn wrap<T: ToString>(message: T) -> RRDBError {
        RRDBError::WALError(Self {
            message: message.to_string(),
            backtrace: std::backtrace::Backtrace::capture(),
        })
    }
}

impl std::error::Error for WALError {}

impl std::fmt::Display for WALError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "wal error: {}", self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wal_error_eq() {
        let error1 = WALError::wrap("test");
        let error2 = WALError::wrap("test");
        assert_eq!(error1, error2);
    }

    #[test]
    fn test_wal_error_display() {
        let error = WALError::wrap("test");

        assert_eq!(error.to_string(), "wal error: test");
    }

    #[test]
    fn test_wal_error_wrap() {
        let error = WALError::wrap("test");
        assert_eq!(error.to_string(), "wal error: test");
    }
}
