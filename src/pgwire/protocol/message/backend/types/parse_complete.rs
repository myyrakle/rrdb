use bytes::BytesMut;

use crate::pgwire::protocol::backend::BackendMessage;

#[derive(Debug)]
pub struct ParseComplete;

impl BackendMessage for ParseComplete {
    const TAG: u8 = b'1';

    fn encode(&self, _dst: &mut BytesMut) {}
}
