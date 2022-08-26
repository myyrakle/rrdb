use bytes::BytesMut;

use crate::lib::pgwire::protocol::BackendMessage;

#[derive(Debug)]
pub struct BindComplete;

impl BackendMessage for BindComplete {
    const TAG: u8 = b'2';

    fn encode(&self, _dst: &mut BytesMut) {}
}
