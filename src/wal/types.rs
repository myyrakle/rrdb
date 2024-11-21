use bitcode::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct WALEntry {
    pub entry_type: EntryType,
    pub data: Option<Vec<u8>>,
    pub timestamp: f64,
    pub transaction_id: Option<u64>,
}

impl WALEntry {
    pub fn size(&self) -> usize {
        let data_size = self.data.as_ref().map_or(0, |data| data.len());
        size_of::<EntryType>() + size_of::<f64>() + size_of::<u64>() + data_size
    }
}

#[derive(Clone, Debug, Encode, Decode)]
pub enum EntryType {
    Insert,
    Set,
    Delete,
    Checkpoint,

    TransactionBegin,
    TransactionCommit,
}
