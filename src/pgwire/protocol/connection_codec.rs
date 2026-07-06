use bytes::{Buf, BufMut, BytesMut};
use std::collections::HashMap;
use tokio_util::codec::{Decoder, Encoder};

use crate::pgwire::protocol::ProtocolError;

use super::{
    FormatCode, MESSAGE_HEADER_SIZE, STARTUP_HEADER_SIZE,
    backend::BackendMessage,
    client::{Bind, BindFormat, ClientMessage, Close, Describe, Execute, Parse, Startup},
};

#[derive(Default, Debug)]
pub struct ConnectionCodec {
    // most state tracking is handled at a higher level
    // however, the actual wire format uses a different header for startup vs normal messages
    // so we need to be able to differentiate inside the decoder
    startup_received: bool,
}

impl ConnectionCodec {
    pub fn new() -> Self {
        Self {
            startup_received: false,
        }
    }

    fn read_u8(src: &mut BytesMut) -> Result<u8, ProtocolError> {
        if src.is_empty() {
            return Err(ProtocolError::ParserError);
        }

        Ok(src.get_u8())
    }

    fn read_i16(src: &mut BytesMut) -> Result<i16, ProtocolError> {
        if src.len() < size_of::<i16>() {
            return Err(ProtocolError::ParserError);
        }

        Ok(src.get_i16())
    }

    fn read_i32(src: &mut BytesMut) -> Result<i32, ProtocolError> {
        if src.len() < size_of::<i32>() {
            return Err(ProtocolError::ParserError);
        }

        Ok(src.get_i32())
    }

    fn read_u32(src: &mut BytesMut) -> Result<u32, ProtocolError> {
        if src.len() < size_of::<u32>() {
            return Err(ProtocolError::ParserError);
        }

        Ok(src.get_u32())
    }

    fn read_cstr(src: &mut BytesMut) -> Result<String, ProtocolError> {
        let next_null = src
            .iter()
            .position(|&b| b == 0)
            .ok_or(ProtocolError::ParserError)?;
        let bytes = src[..next_null].to_owned();
        src.advance(bytes.len() + 1);
        Ok(String::from_utf8(bytes)?)
    }
}

impl Decoder for ConnectionCodec {
    type Item = ClientMessage;
    type Error = ProtocolError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if !self.startup_received {
            if src.len() < STARTUP_HEADER_SIZE {
                return Ok(None);
            }

            let mut header_buf = src.clone();
            let message_len = header_buf.get_i32() as usize;
            let protocol_version_major = header_buf.get_i16();
            let protocol_version_minor = header_buf.get_i16();

            if protocol_version_major == 1234i16 && protocol_version_minor == 5679i16 {
                src.advance(STARTUP_HEADER_SIZE);
                return Ok(Some(ClientMessage::SSLRequest));
            }

            if protocol_version_major == 1234i16 && protocol_version_minor == 5680i16 {
                src.advance(STARTUP_HEADER_SIZE);
                return Ok(Some(ClientMessage::GSSENCRequest));
            }

            if src.len() < message_len {
                src.reserve(message_len - src.len());
                return Ok(None);
            }

            src.advance(STARTUP_HEADER_SIZE);

            let mut parameters = HashMap::new();

            let mut param_str_start_pos = 0;
            let mut current_key = None;

            for (i, &blah) in src.iter().enumerate() {
                if blah == 0 {
                    let string_value = String::from_utf8(src[param_str_start_pos..i].to_owned())?;

                    param_str_start_pos = i + 1;

                    current_key = match current_key {
                        Some(key) => {
                            parameters.insert(key, string_value);
                            None
                        }
                        None => Some(string_value),
                    }
                }
            }

            src.advance(message_len - STARTUP_HEADER_SIZE);

            self.startup_received = true;
            return Ok(Some(ClientMessage::Startup(Startup {
                requested_protocol_version: (protocol_version_major, protocol_version_minor),
                parameters,
            })));
        }

        if src.len() < MESSAGE_HEADER_SIZE {
            src.reserve(MESSAGE_HEADER_SIZE);
            return Ok(None);
        }

        let mut header_buf = src.clone();
        let message_tag = header_buf.get_u8();
        let message_len = header_buf.get_i32() as usize;

        if message_len < size_of::<i32>() {
            return Err(ProtocolError::ParserError);
        }

        let total_message_len = 1 + message_len;

        if src.len() < total_message_len {
            src.reserve(total_message_len - src.len());
            return Ok(None);
        }

        src.advance(MESSAGE_HEADER_SIZE);
        let mut body = src.split_to(message_len - size_of::<i32>());

        let message = match message_tag {
            b'P' => {
                let prepared_statement_name = Self::read_cstr(&mut body)?;
                let query = Self::read_cstr(&mut body)?;
                let num_params = Self::read_i16(&mut body)?;

                if num_params < 0 {
                    return Err(ProtocolError::ParserError);
                }

                for _ in 0..num_params {
                    let _param_type = Self::read_u32(&mut body)?;
                }

                ClientMessage::Parse(Parse {
                    prepared_statement_name,
                    query,
                    parameter_types: Vec::new(),
                })
            }
            b'D' => {
                let target_type = Self::read_u8(&mut body)?;
                let name = Self::read_cstr(&mut body)?;

                ClientMessage::Describe(match target_type {
                    b'P' => Describe::Portal(name),
                    b'S' => Describe::PreparedStatement(name),
                    _ => return Err(ProtocolError::ParserError),
                })
            }
            b'C' => {
                let target_type = Self::read_u8(&mut body)?;
                let name = Self::read_cstr(&mut body)?;

                ClientMessage::Close(match target_type {
                    b'P' => Close::Portal(name),
                    b'S' => Close::PreparedStatement(name),
                    _ => return Err(ProtocolError::ParserError),
                })
            }
            b'H' => ClientMessage::Flush,
            b'S' => ClientMessage::Sync,
            b'B' => {
                let portal = Self::read_cstr(&mut body)?;
                let prepared_statement_name = Self::read_cstr(&mut body)?;

                let num_param_format_codes = Self::read_i16(&mut body)?;
                if num_param_format_codes < 0 {
                    return Err(ProtocolError::ParserError);
                }
                for _ in 0..num_param_format_codes {
                    let _format_code = Self::read_i16(&mut body)?;
                }

                let num_params = Self::read_i16(&mut body)?;
                if num_params < 0 {
                    return Err(ProtocolError::ParserError);
                }
                let mut parameters = Vec::with_capacity(num_params as usize);
                for _ in 0..num_params {
                    let param_len = Self::read_i32(&mut body)?;
                    if param_len == -1 {
                        parameters.push(None);
                        continue;
                    }
                    if param_len < -1 {
                        return Err(ProtocolError::ParserError);
                    }

                    let param_len = param_len as usize;
                    if body.len() < param_len {
                        return Err(ProtocolError::ParserError);
                    }
                    let param = String::from_utf8(body[..param_len].to_vec())?;
                    parameters.push(Some(param));
                    body.advance(param_len);
                }

                let result_format = match Self::read_i16(&mut body)? {
                    0 => BindFormat::All(FormatCode::Text),
                    1 => BindFormat::All(Self::read_i16(&mut body)?.try_into()?),
                    n => {
                        if n < 0 {
                            return Err(ProtocolError::ParserError);
                        }

                        let mut result_format_codes = Vec::new();
                        for _ in 0..n {
                            result_format_codes.push(Self::read_i16(&mut body)?.try_into()?);
                        }
                        BindFormat::PerColumn(result_format_codes)
                    }
                };

                ClientMessage::Bind(Bind {
                    portal,
                    prepared_statement_name,
                    parameters,
                    result_format,
                })
            }
            b'E' => {
                let portal = Self::read_cstr(&mut body)?;
                let max_rows = match Self::read_i32(&mut body)? {
                    0 => None,
                    other => Some(other),
                };

                ClientMessage::Execute(Execute { portal, max_rows })
            }
            b'Q' => {
                let query = Self::read_cstr(&mut body)?;
                ClientMessage::Query(query)
            }
            b'X' => ClientMessage::Terminate,
            other => {
                return Err(ProtocolError::InvalidMessageType(other));
            }
        };

        Ok(Some(message))
    }
}

impl<T: BackendMessage> Encoder<T> for ConnectionCodec {
    type Error = ProtocolError;

    fn encode(&mut self, item: T, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let mut body = BytesMut::new();
        item.encode(&mut body);

        dst.put_u8(T::TAG);
        dst.put_i32((body.len() + 4) as i32);
        dst.put_slice(&body);
        Ok(())
    }
}

impl Encoder<char> for ConnectionCodec {
    type Error = ProtocolError;

    fn encode(&mut self, item: char, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.put_u8(item as u8);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use bytes::{BufMut, BytesMut};
    use tokio_util::codec::Decoder;

    use crate::pgwire::protocol::client::{ClientMessage, Close};

    use super::ConnectionCodec;

    fn close_message(target_type: u8, name: &str) -> BytesMut {
        let mut message = BytesMut::new();
        message.put_u8(b'C');
        message.put_i32((4 + 1 + name.len() + 1) as i32);
        message.put_u8(target_type);
        message.put_slice(name.as_bytes());
        message.put_u8(0);
        message
    }

    fn tag_only_message(tag: u8) -> BytesMut {
        let mut message = BytesMut::new();
        message.put_u8(tag);
        message.put_i32(4);
        message
    }

    fn startup_negotiation_message(minor: i16) -> BytesMut {
        let mut message = BytesMut::new();
        message.put_i32(8);
        message.put_i16(1234);
        message.put_i16(minor);
        message
    }

    fn execute_message(portal: &str, max_rows: i32) -> BytesMut {
        let mut message = BytesMut::new();
        message.put_u8(b'E');
        message.put_i32((4 + portal.len() + 1 + 4) as i32);
        message.put_slice(portal.as_bytes());
        message.put_u8(0);
        message.put_i32(max_rows);
        message
    }

    fn bind_message(statement: &str, params: &[Option<&str>]) -> BytesMut {
        let mut body = BytesMut::new();
        body.put_u8(0);
        body.put_slice(statement.as_bytes());
        body.put_u8(0);
        body.put_i16(0);
        body.put_i16(params.len() as i16);

        for param in params {
            match param {
                Some(value) => {
                    body.put_i32(value.len() as i32);
                    body.put_slice(value.as_bytes());
                }
                None => body.put_i32(-1),
            }
        }

        body.put_i16(0);

        let mut message = BytesMut::new();
        message.put_u8(b'B');
        message.put_i32((4 + body.len()) as i32);
        message.extend_from_slice(&body);
        message
    }

    #[test]
    fn decodes_bind_text_parameters() {
        let mut codec = ConnectionCodec {
            startup_received: true,
        };
        let mut message = bind_message("sqlx_s_1", &[Some("alpha"), None, Some("42")]);

        let decoded = codec.decode(&mut message).unwrap().unwrap();

        match decoded {
            ClientMessage::Bind(bind) => {
                assert_eq!(bind.prepared_statement_name, "sqlx_s_1");
                assert_eq!(
                    bind.parameters,
                    vec![Some("alpha".to_string()), None, Some("42".to_string())]
                );
            }
            other => panic!("expected bind, got {other:?}"),
        }
    }

    #[test]
    fn decodes_close_prepared_statement_message() {
        let mut codec = ConnectionCodec {
            startup_received: true,
        };
        let mut message = close_message(b'S', "sqlx_s_1");

        let decoded = codec.decode(&mut message).unwrap().unwrap();

        match decoded {
            ClientMessage::Close(Close::PreparedStatement(name)) => {
                assert_eq!(name, "sqlx_s_1");
            }
            other => panic!("expected prepared statement close, got {other:?}"),
        }
    }

    #[test]
    fn decodes_close_portal_message() {
        let mut codec = ConnectionCodec {
            startup_received: true,
        };
        let mut message = close_message(b'P', "sqlx_p_1");

        let decoded = codec.decode(&mut message).unwrap().unwrap();

        match decoded {
            ClientMessage::Close(Close::Portal(name)) => {
                assert_eq!(name, "sqlx_p_1");
            }
            other => panic!("expected portal close, got {other:?}"),
        }
    }

    #[test]
    fn decodes_flush_message() {
        let mut codec = ConnectionCodec {
            startup_received: true,
        };
        let mut message = tag_only_message(b'H');

        let decoded = codec.decode(&mut message).unwrap().unwrap();

        assert!(matches!(decoded, ClientMessage::Flush));
    }

    #[test]
    fn decodes_gss_encryption_request_without_consuming_startup_state() {
        let mut codec = ConnectionCodec::new();
        let mut message = startup_negotiation_message(5680);

        let decoded = codec.decode(&mut message).unwrap().unwrap();

        assert!(matches!(decoded, ClientMessage::GSSENCRequest));
        assert!(!codec.startup_received);
    }

    #[test]
    fn waits_for_complete_message_body_before_decoding() {
        let mut codec = ConnectionCodec {
            startup_received: true,
        };
        let mut message = execute_message("", 0);
        message.truncate(message.len() - 1);

        let decoded = codec.decode(&mut message).unwrap();

        assert!(decoded.is_none());
    }

    #[test]
    fn malformed_execute_message_returns_parser_error_without_panicking() {
        let mut codec = ConnectionCodec {
            startup_received: true,
        };
        let mut message = BytesMut::new();
        message.put_u8(b'E');
        message.put_i32(5);
        message.put_u8(0);

        let decoded = codec.decode(&mut message);

        assert!(decoded.is_err());
    }
}
