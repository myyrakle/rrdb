use std::backtrace::Backtrace;

pub mod execute_error;
// pub mod into_error;
pub mod lexing_error;
pub mod parsing_error;
// pub mod predule;
// pub mod server_error;
pub mod type_error;
pub mod wal_errors;

pub struct Errors {
    pub kind: ErrorKind,
    pub backtrace: Backtrace,
    pub message: Option<String>,
}

impl Errors {
    pub fn new(kind: ErrorKind) -> Self {
        Errors {
            kind,
            backtrace: Backtrace::capture(),
            message: None,
        }
    }

    pub fn with_message(mut self, message: String) -> Self {
        self.message = Some(message);
        self
    }
}

#[derive(Debug, PartialEq)]
pub enum ErrorKind {
    LexingError(String),
    TypeError(String),
    ExecuteError(String),
    IntoError(String),
    ParsingError(String),
    ServerError(String),
    WALError(String),
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ErrorKind::ExecuteError(msg) => write!(formatter, "{}", msg),
            ErrorKind::IntoError(msg) => write!(formatter, "parsing error(into error): {}", msg),
            ErrorKind::LexingError(msg) => write!(formatter, "lexing error: {}", msg),
            ErrorKind::ParsingError(msg) => write!(formatter, "parsing error: {}", msg),
            ErrorKind::ServerError(msg) => write!(formatter, "server error: {}", msg),
            ErrorKind::TypeError(msg) => write!(formatter, "type error: {}", msg),
            ErrorKind::WALError(msg) => write!(formatter, "wal error: {}", msg),
        }
    }
}

impl std::fmt::Display for Errors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(msg) = &self.message {
            write!(f, "{}: {}", self.kind, msg)
        } else {
            write!(f, "{}", self.kind)
        }
    }
}

impl std::fmt::Debug for Errors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(msg) = &self.message {
            write!(f, "{:?} = {}\n{}", self.kind, msg, self.backtrace)
        } else {
            write!(f, "{:?}\n{}", self.kind, self.backtrace)
        }
    }
}

impl std::error::Error for ErrorKind {}
impl std::error::Error for Errors {}

impl From<ErrorKind> for Errors {
    fn from(error_code: ErrorKind) -> Self {
        Errors::new(error_code)
    }
}

pub type Result<T> = std::result::Result<T, Errors>;
