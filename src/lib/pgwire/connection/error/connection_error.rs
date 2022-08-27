use crate::lib::pgwire::protocol::{ErrorResponse, ProtocolError};

/// Describes an error that may or may not result in the termination of a connection.
#[derive(thiserror::Error, Debug)]
pub enum ConnectionError {
    /// A protocol error was encountered, e.g. an invalid message for a connection's current state.
    #[error("protocol error: {0}")]
    Protocol(ProtocolError),
    /// A Postgres error containing a SqlState code and message occurred.
    /// May result in connection termination depending on the severity.
    #[error("error response: {0}")]
    ErrorResponse(ErrorResponse),
    /// The connection was closed.
    /// This always implies connection termination.
    #[error("connection closed")]
    ConnectionClosed,
}

impl From<ProtocolError> for ConnectionError {
    fn from(value: ProtocolError) -> ConnectionError {
        ConnectionError::Protocol(value)
    }
}

impl From<ErrorResponse> for ConnectionError {
    fn from(value: ErrorResponse) -> ConnectionError {
        ConnectionError::ErrorResponse(value)
    }
}
