use crate::engine::wal::endec::{WALDecoder, WALEncoder};
use crate::engine::wal::types::WALEntry;
use crate::errors::Errors;
use crate::errors::wal_errors::WALError;

#[derive(Clone)]
pub struct BitcodeEncoder {}
impl Default for BitcodeEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl BitcodeEncoder {
    pub fn new() -> Self {
        Self {}
    }
}

impl WALEncoder<Vec<WALEntry>> for BitcodeEncoder {
    fn encode(&self, entry: &Vec<WALEntry>) -> Result<Vec<u8>, Errors> {
        Ok(bitcode::encode(entry))
    }
}

#[derive(Clone)]
pub struct BitcodeDecoder {}
impl Default for BitcodeDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl BitcodeDecoder {
    pub fn new() -> Self {
        Self {}
    }
}

impl WALDecoder<Vec<WALEntry>> for BitcodeDecoder {
    fn decode(&self, data: &[u8]) -> Result<Vec<WALEntry>, Errors> {
        bitcode::decode(data).map_err(|e| WALError::wrap(e.to_string()))
    }
}
