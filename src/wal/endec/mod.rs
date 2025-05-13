pub mod implements;

use crate::errors::RRDBError;

pub trait WALEncoder<T>: Clone {
    fn encode(&self, entry: &T) -> Result<Vec<u8>, RRDBError>;
}

pub trait WALDecoder<T>: Clone {
    fn decode(&self, data: &[u8]) -> Result<T, RRDBError>;
}
