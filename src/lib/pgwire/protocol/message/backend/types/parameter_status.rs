use bytes::BytesMut;

use crate::lib::pgwire::protocol::BackendMessage;

#[derive(Debug)]
pub struct ParameterStatus {
    name: String,
    value: String,
}

impl BackendMessage for ParameterStatus {
    const TAG: u8 = b'S';

    fn encode(&self, dst: &mut BytesMut) {
        dst.put_slice(self.name.as_bytes());
        dst.put_u8(0);
        dst.put_slice(self.value.as_bytes());
        dst.put_u8(0);
    }
}

impl ParameterStatus {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}
