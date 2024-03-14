use bytes::{BufMut, BytesMut};

use crate::pgwire::protocol::BackendMessage;

#[derive(Debug)]
pub struct AuthenticationOk;

impl BackendMessage for AuthenticationOk {
    const TAG: u8 = b'R';

    fn encode(&self, dst: &mut BytesMut) {
        dst.put_i32(0);
    }
}
