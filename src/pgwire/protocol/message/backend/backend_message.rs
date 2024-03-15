use bytes::BytesMut;

pub trait BackendMessage: std::fmt::Debug {
    const TAG: u8;

    fn encode(&self, dst: &mut BytesMut);
}
