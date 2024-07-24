use super::RRDBError;

#[derive(Debug)]
pub struct IntoError {
    pub message: String,
    pub backtrace: std::backtrace::Backtrace,
}

impl PartialEq for IntoError {
    fn eq(&self, other: &Self) -> bool {
        self.message == other.message
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_into_error_eq() {
        let error1 = IntoError::wrap("test");
        let error2 = IntoError::wrap("test");
        assert_eq!(error1, error2);
    }

    #[test]
    fn test_into_error_display() {
        let error = IntoError::wrap("test");

        assert!(error
            .to_string()
            .contains("parsing error(into error): test"));
    }
}
