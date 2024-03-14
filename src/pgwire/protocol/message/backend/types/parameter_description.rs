use bytes::{BufMut, BytesMut};

use crate::pgwire::protocol::BackendMessage;

#[derive(Debug)]
pub struct ParameterDescription {}

impl BackendMessage for ParameterDescription {
    const TAG: u8 = b't';

    fn encode(&self, dst: &mut BytesMut) {
        dst.put_i16(0);
    }
}
