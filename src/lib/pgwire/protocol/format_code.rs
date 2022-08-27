use std::convert::TryFrom;

use crate::lib::pgwire::protocol::ProtocolError;

/// Describes how to format a given value or set of values.
#[derive(Debug, Copy, Clone)]
pub enum FormatCode {
    /// Use the stable text representation.
    Text = 0,
    /// Use the less-stable binary representation.
    Binary = 1,
}

impl TryFrom<i16> for FormatCode {
    type Error = ProtocolError;

    fn try_from(value: i16) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(FormatCode::Text),
            1 => Ok(FormatCode::Binary),
            other => Err(ProtocolError::InvalidFormatCode(other)),
        }
    }
}
