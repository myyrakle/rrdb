use serde::{Deserialize, Serialize};

pub struct StorageEncoder {}

impl StorageEncoder {
    pub fn encode(data: impl Serialize) -> Vec<u8> {
        bson::to_vec(&data).unwrap()
    }

    pub fn decode<'a, T>(data: &'a [u8]) -> Option<T>
    where
        T: Deserialize<'a>,
    {
        match bson::from_slice(data) {
            Ok(data) => Some(data),
            _ => None,
        }
    }
}
