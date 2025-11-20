pub mod implements;

use crate::errors::Errors;

pub trait WALEncoder<T>: Clone {
    fn encode(&self, entry: &T) -> Result<Vec<u8>, Errors>;
}

pub trait WALDecoder<T>: Clone {
    fn decode(&self, data: &[u8]) -> Result<T, Errors>;
}
