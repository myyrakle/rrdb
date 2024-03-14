use bytes::{BufMut, BytesMut};

use crate::lib::pgwire::protocol::BackendMessage;

#[derive(Debug)]
pub struct CommandComplete {
    pub command_tag: String,
}

impl BackendMessage for CommandComplete {
    const TAG: u8 = b'C';

    fn encode(&self, dst: &mut BytesMut) {
        dst.put_slice(self.command_tag.as_bytes());
        dst.put_u8(0);
    }
}
