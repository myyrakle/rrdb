use std::{error::Error, string::ToString};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeError {
    pub message: String,
}

impl TypeError {
    pub fn new<T: ToString>(message: T) -> Self {
        Self {
            message: message.to_string(),
        }
    }

    pub fn boxed<T: ToString>(message: T) -> Box<Self> {
        Box::new(Self::new(message))
    }

    pub fn dyn_boxed<T: ToString>(message: T) -> Box<dyn Error + Send> {
        Box::new(Self::new(message))
    }
}

impl std::error::Error for TypeError {}

impl std::fmt::Display for TypeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "parsing error: {}", self.message)
    }
}
