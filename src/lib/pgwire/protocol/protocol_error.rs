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
