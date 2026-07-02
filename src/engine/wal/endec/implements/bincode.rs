use crate::engine::wal::endec::{WALDecoder, WALEncoder};
use crate::engine::wal::types::WALEntry;
use crate::errors;
use crate::errors::wal_errors::WALError;

#[derive(Clone)]
pub struct BincodeEncoder {}

impl Default for BincodeEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl BincodeEncoder {
    pub fn new() -> Self {
        Self {}
    }
}

impl WALEncoder<Vec<WALEntry>> for BincodeEncoder {
    fn encode(&self, entry: &Vec<WALEntry>) -> errors::Result<Vec<u8>> {
        bincode::serialize(entry).map_err(|e| WALError::wrap(e.to_string()))
    }
}

#[derive(Clone)]
pub struct BincodeDecoder {}

impl Default for BincodeDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl BincodeDecoder {
    pub fn new() -> Self {
        Self {}
    }
}

impl WALDecoder<Vec<WALEntry>> for BincodeDecoder {
    fn decode(&self, data: &[u8]) -> errors::Result<Vec<WALEntry>> {
        let mut entries = Vec::new();
        let mut offset = 0;

        while offset < data.len() {
            if data.len() - offset < size_of::<u32>() {
                return Err(WALError::wrap("truncated wal frame header".to_string()));
            }

            let frame_len = u32::from_le_bytes(
                data[offset..offset + size_of::<u32>()]
                    .try_into()
                    .map_err(|e| WALError::wrap(format!("{:?}", e)))?,
            ) as usize;
            offset += size_of::<u32>();

            if data.len() - offset < frame_len {
                return Err(WALError::wrap("truncated wal frame body".to_string()));
            }

            let mut frame_entries: Vec<WALEntry> =
                bincode::deserialize(&data[offset..offset + frame_len])
                    .map_err(|e| WALError::wrap(e.to_string()))?;
            entries.append(&mut frame_entries);
            offset += frame_len;
        }

        Ok(entries)
    }
}
