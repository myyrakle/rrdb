use bytes::BytesMut;

use crate::pgwire::protocol::backend::BackendMessage;

#[derive(Debug)]
pub struct NoData;

impl BackendMessage for NoData {
    const TAG: u8 = b'n';

    fn encode(&self, _dst: &mut BytesMut) {}
}
