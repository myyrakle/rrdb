use bytes::BytesMut;

use crate::pgwire::protocol::backend::BackendMessage;

#[derive(Debug)]
pub struct CloseComplete;

impl BackendMessage for CloseComplete {
    const TAG: u8 = b'3';

    fn encode(&self, _dst: &mut BytesMut) {}
}
