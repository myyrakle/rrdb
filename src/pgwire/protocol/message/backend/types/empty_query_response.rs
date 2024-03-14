use bytes::BytesMut;

use crate::pgwire::protocol::BackendMessage;

#[derive(Debug)]
pub struct EmptyQueryResponse;

impl BackendMessage for EmptyQueryResponse {
    const TAG: u8 = b'I';

    fn encode(&self, _dst: &mut BytesMut) {}
}
