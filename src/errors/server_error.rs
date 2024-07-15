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

impl std::error::Error for ServerError {}

impl std::fmt::Display for ServerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "server error: {}", self.message)
    }
}
