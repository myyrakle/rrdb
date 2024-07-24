use super::RRDBError;

#[derive(Debug)]
pub struct ServerError {
    pub message: String,
    pub backtrace: std::backtrace::Backtrace,
}

impl PartialEq for ServerError {
    fn eq(&self, other: &Self) -> bool {
        self.message == other.message
    }
}

impl ServerError {
    pub fn new<T: ToString>(message: T) -> Self {
        Self {
            message: message.to_string(),
            backtrace: std::backtrace::Backtrace::capture(),
        }
    }

    pub fn boxed<T: ToString>(message: T) -> Box<Self> {
        Box::new(Self::new(message))
    }
}

impl ServerError {
    pub fn wrap<T: ToString>(message: T) -> RRDBError {
        RRDBError::ServerError(Self {
            message: message.to_string(),
            backtrace: std::backtrace::Backtrace::capture(),
        })
    }
}

impl std::error::Error for ServerError {}

impl std::fmt::Display for ServerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "server error: {}", self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_error_eq() {
        let error1 = ServerError::wrap("test");
        let error2 = ServerError::wrap("test");
        assert_eq!(error1, error2);
    }

    #[test]
    fn test_server_error_display() {
        let error = ServerError::wrap("test");

        assert!(error.to_string().contains("server error: test"));
    }

    #[test]
    fn test_server_error_new() {
        let error = ServerError::new("test");

        assert_eq!(error.message, "test");
    }

    #[test]
    fn test_server_error_boxed() {
        let error = ServerError::boxed("test");

        assert_eq!(error.message, "test");
    }
}
