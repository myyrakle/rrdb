use bytes::BytesMut;

use crate::lib::pgwire::protocol::BackendMessage;

#[derive(Debug)]
pub struct ParseComplete;

impl BackendMessage for ParseComplete {
    const TAG: u8 = b'1';

    fn encode(&self, _dst: &mut BytesMut) {}
}
