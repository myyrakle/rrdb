use std::fmt::Display;

use bytes::{BufMut, BytesMut};

use crate::pgwire::protocol::{BackendMessage, Severity, SqlState};

#[derive(thiserror::Error, Debug, Clone)]
pub struct ErrorResponse {
    pub sql_state: SqlState,
    pub severity: Severity,
    pub message: String,
}

impl ErrorResponse {
    pub fn new(sql_state: SqlState, severity: Severity, message: impl Into<String>) -> Self {
        ErrorResponse {
            sql_state,
            severity,
            message: message.into(),
        }
    }

    pub fn error(sql_state: SqlState, message: impl Into<String>) -> Self {
        Self::new(sql_state, Severity::ERROR, message)
    }

    pub fn fatal(sql_state: SqlState, message: impl Into<String>) -> Self {
        Self::new(sql_state, Severity::FATAL, message)
    }
}

impl Display for ErrorResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "error")
    }
}

impl From<Box<dyn std::error::Error>> for ErrorResponse {
    fn from(value: Box<dyn std::error::Error>) -> ErrorResponse {
        ErrorResponse {
            sql_state: SqlState::SYNTAX_ERROR,
            severity: Severity::ERROR,
            message: value.to_string(),
        }
    }
}

impl BackendMessage for ErrorResponse {
    const TAG: u8 = b'E';

    fn encode(&self, dst: &mut BytesMut) {
        dst.put_u8(b'C');
        dst.put_slice(self.sql_state.0.as_bytes());
        dst.put_u8(0);
        dst.put_u8(b'S');
        dst.put_slice(self.severity.0.as_bytes());
        dst.put_u8(0);
        dst.put_u8(b'M');
        dst.put_slice(self.message.as_bytes());
        dst.put_u8(0);

        dst.put_u8(0); // tag
    }
}
