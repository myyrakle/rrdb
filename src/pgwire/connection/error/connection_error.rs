use crate::pgwire::protocol::{backend::ErrorResponse, ProtocolError};

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
