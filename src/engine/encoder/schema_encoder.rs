use serde::{Deserialize, Serialize};

pub struct StorageEncoder {}

#[allow(clippy::new_without_default)]
impl StorageEncoder {
    pub fn new() -> Self {
        StorageEncoder {}
    }

    pub fn encode(&self, data: impl Serialize) -> Vec<u8> {
        bincode::serialize(&data).unwrap()
    }

    pub fn encode_into(
        &self,
        writer: impl std::io::Write,
        data: impl Serialize,
    ) -> bincode::Result<()> {
        bincode::serialize_into(writer, &data)
    }

    pub fn decode<'a, T>(&self, data: &'a [u8]) -> bincode::Result<T>
    where
        T: Deserialize<'a>,
    {
        bincode::deserialize(data)
    }
}
