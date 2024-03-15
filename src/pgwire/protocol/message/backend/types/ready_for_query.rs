use bytes::{BufMut, BytesMut};

use crate::pgwire::protocol::backend::BackendMessage;

#[derive(Debug)]
pub struct ReadyForQuery;

impl BackendMessage for ReadyForQuery {
    const TAG: u8 = b'Z';

    fn encode(&self, dst: &mut BytesMut) {
        dst.put_u8(b'I');
    }
}
