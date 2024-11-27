use crate::errors::{predule::WALError, RRDBError};

use super::types::WALEntry;

pub trait WALEncoder<T>: Clone {
    fn encode(&self, entry: &T) -> Result<Vec<u8>, RRDBError>;
}

pub trait WALDecoder<T>: Clone {
    fn decode(&self, data: &[u8]) -> Result<T, RRDBError>;
}


#[derive(Clone)]
pub struct BitcodeEncoder {}
impl BitcodeEncoder {
    pub fn new() -> Self {
        Self {}
    }
}

impl WALEncoder<Vec<WALEntry>> for BitcodeEncoder {
    fn encode(&self, entry: &Vec<WALEntry>) -> Result<Vec<u8>, RRDBError> {
        Ok(bitcode::encode(entry))
    }
}

#[derive(Clone)]
pub struct BitcodeDecoder {}
impl BitcodeDecoder {
    pub fn new() -> Self {
        Self {}
    }
}

impl WALDecoder<Vec<WALEntry>> for BitcodeDecoder {
    fn decode(&self, data: &[u8]) -> Result<Vec<WALEntry>, RRDBError> {
        Ok(bitcode::decode(data).map_err(|e| WALError::wrap(e.to_string()))?)
    }
}
