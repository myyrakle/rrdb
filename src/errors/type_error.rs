use super::RRDBError;

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
    pub fn wrap<T: ToString>(message: T) -> RRDBError {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_error_eq() {
        let error1 = TypeError::wrap("test");
        let error2 = TypeError::wrap("test");
        assert_eq!(error1, error2);
    }

    #[test]
    fn test_type_error_display() {
        let error = TypeError::wrap("test");

        assert!(error.to_string().contains("parsing error: test"));
    }
}
