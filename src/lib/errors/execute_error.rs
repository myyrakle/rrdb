use std::string::ToString;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecuteError {
    pub message: String,
}

impl ExecuteError {
    pub fn new<T: ToString>(message: T) -> Self {
        Self {
            message: message.to_string(),
        }
    }

    pub fn boxed<T: ToString>(message: T) -> Box<Self> {
        Box::new(Self::new(message))
    }
}

impl std::error::Error for ExecuteError {}

impl std::fmt::Display for ExecuteError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "parsing error: {}", self.message)
    }
}
