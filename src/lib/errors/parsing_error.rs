#[derive(Debug, Clone)]
pub struct ParsingError {
    pub message: String,
}

impl ParsingError {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_owned(),
        }
    }

    pub fn boxed(message: &str) -> Box<Self> {
        Box::new(Self::new(message))
    }
}

impl std::error::Error for ParsingError {}

impl std::fmt::Display for ParsingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "parsing error: {}", self.message)
    }
}
