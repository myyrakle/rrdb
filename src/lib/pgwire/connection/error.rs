/// Describes an error that may or may not result in the termination of a connection.
#[derive(thiserror::Error, Debug)]
pub enum ConnectionError {
    /// A protocol error was encountered, e.g. an invalid message for a connection's current state.
    #[error("protocol error: {0}")]
    Protocol(#[from] ProtocolError),
    /// A Postgres error containing a SqlState code and message occurred.
    /// May result in connection termination depending on the severity.
    #[error("error response: {0}")]
    ErrorResponse(#[from] ErrorResponse),
    /// The connection was closed.
    /// This always implies connection termination.
    #[error("connection closed")]
    ConnectionClosed,
}

#[derive(thiserror::Error, Debug)]
pub enum ProtocolError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("utf8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("parsing error")]
    ParserError,
    #[error("invalid message type: {0}")]
    InvalidMessageType(u8),
    #[error("invalid format code: {0}")]
    InvalidFormatCode(i16),
}

#[derive(thiserror::Error, Debug, Clone)]
pub struct ErrorResponse {
    pub sql_state: SqlState,
    pub severity: Severity,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqlState(pub &'static str);

#[derive(thiserror::Error, Debug)]
pub enum ProtocolError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("utf8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("parsing error")]
    ParserError,
    #[error("invalid message type: {0}")]
    InvalidMessageType(u8),
    #[error("invalid format code: {0}")]
    InvalidFormatCode(i16),
}
