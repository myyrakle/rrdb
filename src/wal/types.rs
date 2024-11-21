use bitcode::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct WALEntry {
    pub entry_type: EntryType,
    pub data: Option<Vec<u8>>,
    pub timestamp: f64,
    pub transaction_id: Option<u64>,
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
