use super::RRDBError;

#[derive(Debug)]
pub struct ParsingError {
    pub message: String,
    pub backtrace: std::backtrace::Backtrace,
}

impl PartialEq for ParsingError {
    fn eq(&self, other: &Self) -> bool {
        self.message == other.message
    }
}

impl ParsingError {
    pub fn wrap<T: ToString>(message: T) -> RRDBError {
        RRDBError::ParsingError(Self {
            message: message.to_string(),
            backtrace: std::backtrace::Backtrace::capture(),
        })
    }
}

impl std::error::Error for ParsingError {}

impl std::fmt::Display for ParsingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "parsing error: {}", self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parsing_error_eq() {
        let error1 = ParsingError::wrap("test");
        let error2 = ParsingError::wrap("test");
        assert_eq!(error1, error2);
    }

    #[test]
    fn test_parsing_error_display() {
        let error = ParsingError::wrap("test");

        assert!(error.to_string().contains("parsing error: test"));
    }
}
