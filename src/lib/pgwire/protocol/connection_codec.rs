use std::collections::HashMap;

use bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder};

use crate::lib::pgwire::connection::ProtocolError;

use super::{
    BackendMessage, Bind, BindFormat, ClientMessage, Describe, Execute, FormatCode, Parse, Startup,
    MESSAGE_HEADER_SIZE, STARTUP_HEADER_SIZE,
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

        if src.len() < message_len {
            src.reserve(message_len - src.len());
            return Ok(None);
        }

        src.advance(MESSAGE_HEADER_SIZE);

        let read_cstr = |src: &mut BytesMut| -> Result<String, ProtocolError> {
            let next_null = src
                .iter()
                .position(|&b| b == 0)
                .ok_or(ProtocolError::ParserError)?;
            let bytes = src[..next_null].to_owned();
            src.advance(bytes.len() + 1);
            Ok(String::from_utf8(bytes)?)
        };

        let message = match message_tag {
            b'P' => {
                let prepared_statement_name = read_cstr(src)?;
                let query = read_cstr(src)?;
                let num_params = src.get_i16();
                let _params: Vec<_> = (0..num_params).into_iter().map(|_| src.get_u32()).collect();

                ClientMessage::Parse(Parse {
                    prepared_statement_name,
                    query,
                    parameter_types: Vec::new(),
                })
            }
            b'D' => {
                let target_type = src.get_u8();
                let name = read_cstr(src)?;

                ClientMessage::Describe(match target_type {
                    b'P' => Describe::Portal(name),
                    b'S' => Describe::PreparedStatement(name),
                    _ => return Err(ProtocolError::ParserError),
                })
            }
            b'S' => ClientMessage::Sync,
            b'B' => {
                let portal = read_cstr(src)?;
                let prepared_statement_name = read_cstr(src)?;

                let num_param_format_codes = src.get_i16();
                for _ in 0..num_param_format_codes {
                    let _format_code = src.get_i16();
                }

                let num_params = src.get_i16();
                for _ in 0..num_params {
                    let param_len = src.get_i32() as usize;
                    let _bytes = &src[0..param_len];
                    src.advance(param_len);
                }

                let result_format = match src.get_i16() {
                    0 => BindFormat::All(FormatCode::Text),
                    1 => BindFormat::All(src.get_i16().try_into()?),
                    n => {
                        let mut result_format_codes = Vec::new();
                        for _ in 0..n {
                            result_format_codes.push(src.get_i16().try_into()?);
                        }
                        BindFormat::PerColumn(result_format_codes)
                    }
                };

                ClientMessage::Bind(Bind {
                    portal,
                    prepared_statement_name,
                    result_format,
                })
            }
            b'E' => {
                let portal = read_cstr(src)?;
                let max_rows = match src.get_i32() {
                    0 => None,
                    other => Some(other),
                };

                ClientMessage::Execute(Execute { portal, max_rows })
            }
            b'Q' => {
                let query = read_cstr(src)?;
                ClientMessage::Query(query)
            }
            b'X' => ClientMessage::Terminate,
            other => return Err(ProtocolError::InvalidMessageType(other)),
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
