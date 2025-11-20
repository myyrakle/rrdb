use super::{ErrorKind, Errors};

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
    pub fn wrap<T: ToString>(message: T) -> Errors {
        Errors::new(ErrorKind::WALError(message.to_string()))
    }
}

impl std::error::Error for WALError {}

impl std::fmt::Display for WALError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "wal error: {}", self.message)
    }
}

/*
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wal_error_display() {
        let error = Errors::new(ErrorKind::WALError("test".to_string()));
        assert!(error.to_string().contains("wal error"));
    }
}
*/
