use super::{Errors, ErrorKind};

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

/*
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_error_display() {
        let error = Errors::new(ErrorKind::TypeError("test".to_string()));
        assert!(error.to_string().contains("type error"));
    }
}
*/
