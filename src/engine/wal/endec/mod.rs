pub mod implements;

use crate::errors;

pub trait WALEncoder<T>: Clone {
    fn encode(&self, entry: &T) -> errors::Result<Vec<u8>>;
    fn encode_into(&self, writer: impl std::io::Write, entry: &T) -> errors::Result<()>;
}

pub trait WALDecoder<T>: Clone {
    fn decode(&self, data: &[u8]) -> errors::Result<T>;
}
