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
            let declared_len = header_buf.get_i32();
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

            // 길이는 클라이언트가 보낸 값이므로 신뢰할 수 없습니다. 헤더보다 짧으면
            // 아래 `message_len - STARTUP_HEADER_SIZE`가 언더플로해 패닉하고,
            // 음수면 usize 캐스팅 후 거대한 값이 되어 `reserve`가 할당에 실패합니다.
            // 둘 다 인증 이전 단계라 누구나 서버를 죽일 수 있습니다.
            if declared_len < STARTUP_HEADER_SIZE as i32 {
                return Err(ProtocolError::ParserError);
            }

            let message_len = declared_len as usize;

            if src.len() < message_len {
                src.reserve(message_len - src.len());
                return Ok(None);
            }

            src.advance(STARTUP_HEADER_SIZE);

            let mut parameters = HashMap::new();

            let mut param_str_start_pos = 0;
            let mut current_key = None;

            // 파라미터 영역은 이 메시지 안으로 한정해야 합니다. `src`는 아직
            // 소비되지 않은 스트림 전체라, 뒤에 파이프라이닝된 다음 메시지가
            // 붙어 있으면 그 바이트까지 파라미터로 읽힙니다.
            let parameters_len = message_len - STARTUP_HEADER_SIZE;
            let parameters_buf = &src[..parameters_len];

            for (i, &byte) in parameters_buf.iter().enumerate() {
                if byte == 0 {
                    let string_value =
                        String::from_utf8(parameters_buf[param_str_start_pos..i].to_owned())?;

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

            src.advance(parameters_len);

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
        let declared_len = header_buf.get_i32();

        // 음수 길이는 usize 캐스팅 시 거대한 값이 되어 `reserve`가 할당에
        // 실패하며 패닉합니다. 캐스팅 전에 걸러냅니다.
        if declared_len < size_of::<i32>() as i32 {
            return Err(ProtocolError::ParserError);
        }

        let message_len = declared_len as usize;

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
    use super::STARTUP_HEADER_SIZE;

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

    /// 길이 필드는 클라이언트가 보낸 값이라 신뢰할 수 없습니다. 헤더 크기보다
    /// 작거나 음수인 길이는 `usize` 캐스팅 전에 걸러야 합니다. 그러지 않으면
    /// `message_len - STARTUP_HEADER_SIZE`가 언더플로해 패닉하거나, 음수가
    /// 거대한 `usize`가 되어 `reserve`가 할당에 실패합니다.
    ///
    /// startup 메시지는 인증 이전 단계라 아무나 보낼 수 있어, 이 패닉은 원격에서
    /// 연결 태스크를 죽이는 데 쓰일 수 있었습니다.
    #[test]
    fn malformed_startup_length_is_rejected_without_panicking() {
        for declared_len in [0i32, 1, 4, 7, -1, i32::MIN] {
            let mut codec = ConnectionCodec::new();
            let mut message = BytesMut::new();
            message.put_i32(declared_len);
            message.put_i16(3);
            message.put_i16(0);
            message.put_slice(b"user\0me\0\0");

            assert!(
                codec.decode(&mut message).is_err(),
                "startup length {declared_len} should be rejected"
            );
        }
    }

    /// 정상적인 startup 메시지는 그대로 처리되어야 합니다.
    #[test]
    fn well_formed_startup_message_still_decodes() {
        let mut codec = ConnectionCodec::new();
        let mut body = BytesMut::new();
        body.put_i16(3);
        body.put_i16(0);
        body.put_slice(b"user\0me\0\0");

        let mut message = BytesMut::new();
        message.put_i32((4 + body.len()) as i32);
        message.extend_from_slice(&body);

        let decoded = codec.decode(&mut message).unwrap().unwrap();

        match decoded {
            ClientMessage::Startup(startup) => {
                assert_eq!(startup.requested_protocol_version, (3, 0));
                assert_eq!(
                    startup.parameters.get("user").map(String::as_str),
                    Some("me")
                );
            }
            other => panic!("expected startup, got {other:?}"),
        }
        assert!(codec.startup_received);
    }

    /// 일반 메시지 경로도 같은 이유로 음수 길이를 캐스팅 전에 걸러야 합니다.
    #[test]
    fn negative_regular_message_length_is_rejected_without_panicking() {
        for declared_len in [-1i32, i32::MIN] {
            let mut codec = ConnectionCodec {
                startup_received: true,
            };
            let mut message = BytesMut::new();
            message.put_u8(b'Q');
            message.put_i32(declared_len);
            message.put_slice(b"select 1\0");

            assert!(
                codec.decode(&mut message).is_err(),
                "message length {declared_len} should be rejected"
            );
        }
    }

    /// startup 파라미터 파싱은 이 메시지의 길이 안에서만 이뤄져야 합니다.
    ///
    /// 이전 구현은 아직 소비되지 않은 스트림 전체(`src`)를 순회해서, 클라이언트가
    /// startup 뒤에 다음 메시지를 곧바로 이어 보내면(파이프라이닝) 그 바이트까지
    /// 파라미터로 읽었습니다. 결과적으로 보내지도 않은 항목이 파라미터 맵에
    /// 들어가고, 이어지는 메시지도 일부 소비되어 프레이밍이 어긋납니다.
    #[test]
    fn startup_parameters_stop_at_message_boundary() {
        let mut codec = ConnectionCodec::new();

        let mut body = BytesMut::new();
        body.put_i16(3);
        body.put_i16(0);
        body.put_slice(b"user\0me\0\0");

        let mut message = BytesMut::new();
        message.put_i32((4 + body.len()) as i32);
        message.extend_from_slice(&body);

        // startup 직후에 이어지는 바이트. 파라미터로 섞여 들어가면 안 됩니다.
        message.put_slice(b"INJECTED\0VALUE\0");

        let decoded = codec.decode(&mut message).unwrap().unwrap();

        match decoded {
            ClientMessage::Startup(startup) => {
                assert_eq!(
                    startup.parameters.get("user").map(String::as_str),
                    Some("me")
                );
                assert_eq!(
                    startup.parameters.len(),
                    1,
                    "bytes after the startup message must not become parameters, got {:?}",
                    startup.parameters
                );
            }
            other => panic!("expected startup, got {other:?}"),
        }

        // 뒤따르던 바이트는 그대로 남아 다음 디코딩에 쓰여야 합니다.
        assert_eq!(&message[..], b"INJECTED\0VALUE\0");
    }

    /// 결정적 xorshift. 실패하면 seed 하나로 그대로 재현됩니다.
    struct FuzzRng(u64);

    impl FuzzRng {
        fn next(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.0 = x;
            x
        }
    }

    /// 구조를 갖춘 퍼징: 헤더(tag + length)는 항상 올바르게 만들어서
    /// 길이 검사를 통과시킨 뒤 본문 파서까지 도달시킵니다. 순수 랜덤
    /// 바이트는 유효한 헤더를 거의 만들지 못해 이 경로를 못 건드립니다.
    #[test]
    fn decoder_never_panics_on_structured_garbage() {
        let tags: [u8; 8] = [b'B', b'Q', b'P', b'D', b'E', b'C', b'X', b'F'];

        for seed in 1u64..20000 {
            let mut rng = FuzzRng(seed);
            let tag = tags[(rng.next() % 8) as usize];
            let body_len = (rng.next() % 40) as usize;

            let mut body = Vec::with_capacity(body_len);
            for _ in 0..body_len {
                let r = rng.next();
                body.push(match r % 5 {
                    0 => 0u8,
                    1 => 0xff,
                    2 => 0x80,
                    3 => (r >> 16) as u8,
                    _ => b'a',
                });
            }

            // 길이 필드 자체도 퍼징 대상입니다. 여기를 항상 올바르게 계산하면
            // 조작된 길이로 인한 언더플로/거대 할당 경로를 영영 못 건드립니다.
            let honest = (size_of::<i32>() + body.len()) as i32;
            let declared = match rng.next() % 8 {
                0 => 0,
                1 => -1,
                2 => i32::MIN,
                3 => i32::MAX,
                4 => honest.wrapping_neg(),
                5 => (rng.next() >> 32) as i32,
                6 => honest.wrapping_sub((rng.next() % 16) as i32),
                _ => honest,
            };
            let mut buf = BytesMut::new();
            buf.put_u8(tag);
            buf.put_i32(declared);
            buf.put_slice(&body);

            let mut codec = ConnectionCodec::new();
            codec.startup_received = true;
            // 패닉하지 않는 것이 조건입니다. Err는 정상적인 거부입니다.
            let _ = codec.decode(&mut buf);
        }
    }

    /// 인증 이전 startup 경로도 같은 방식으로 훑습니다.
    #[test]
    fn startup_decoder_never_panics_on_structured_garbage() {
        for seed in 1u64..20000 {
            let mut rng = FuzzRng(seed);
            let body_len = (rng.next() % 40) as usize;

            let mut body = Vec::with_capacity(body_len);
            for _ in 0..body_len {
                let r = rng.next();
                body.push(match r % 4 {
                    0 => 0u8,
                    1 => 0xff,
                    2 => (r >> 16) as u8,
                    _ => b'k',
                });
            }

            let honest = (STARTUP_HEADER_SIZE + body.len()) as i32;
            let declared = match rng.next() % 8 {
                0 => 0,
                1 => -1,
                2 => i32::MIN,
                3 => i32::MAX,
                4 => honest.wrapping_neg(),
                5 => (rng.next() >> 32) as i32,
                6 => honest.wrapping_sub((rng.next() % 16) as i32),
                _ => honest,
            };
            let major = (rng.next() % 4) as i16;
            let minor = (rng.next() % 4) as i16;

            let mut buf = BytesMut::new();
            buf.put_i32(declared);
            buf.put_i16(major);
            buf.put_i16(minor);
            buf.put_slice(&body);

            let mut codec = ConnectionCodec::new();
            let _ = codec.decode(&mut buf);
        }
    }
}
