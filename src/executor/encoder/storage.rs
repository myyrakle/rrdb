use serde::{Deserialize, Serialize};

pub struct StorageEncoder {}

#[allow(clippy::new_without_default)]
impl StorageEncoder {
    pub fn new() -> Self {
        StorageEncoder {}
    }

    pub fn encode(&self, data: impl Serialize) -> Vec<u8> {
        bson::to_vec(&data).unwrap()
    }

    pub fn decode<'a, T>(&self, data: &'a [u8]) -> Option<T>
    where
        T: Deserialize<'a>,
    {
        bson::from_slice(data).ok()
    }
}
