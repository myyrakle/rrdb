use super::RRDBError;

#[derive(Debug)]
pub struct LexingError {
    pub message: String,
    pub backtrace: std::backtrace::Backtrace,
}

impl PartialEq for LexingError {
    fn eq(&self, other: &Self) -> bool {
        self.message == other.message
    }
}

impl LexingError {
    pub fn wrap<T: ToString>(message: T) -> RRDBError {
        RRDBError::LexingError(Self {
            message: message.to_string(),
            backtrace: std::backtrace::Backtrace::capture(),
        })
    }
}

impl std::error::Error for LexingError {}

impl std::fmt::Display for LexingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "lexing error: {}", self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexing_error_eq() {
        let error1 = LexingError::wrap("test");
        let error2 = LexingError::wrap("test");
        assert_eq!(error1, error2);
    }

    #[test]
    fn test_lexing_error_display() {
        let error = LexingError::wrap("test");

        assert!(error.to_string().contains("lexing error: test"));
    }
}
