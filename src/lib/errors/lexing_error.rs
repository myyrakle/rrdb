use std::string::ToString;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LexingError {
    pub message: String,
}

impl LexingError {
    pub fn new<T: ToString>(message: T) -> Self {
        Self {
            message: message.to_string(),
        }
    }

    pub fn boxed<T: ToString>(message: T) -> Box<Self> {
        Box::new(Self::new(message))
    }
}

impl std::error::Error for LexingError {}

impl std::fmt::Display for LexingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "lexing error: {}", self.message)
    }
}
