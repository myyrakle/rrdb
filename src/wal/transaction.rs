use super::format::RecordType;

#[derive(Clone, Copy)]
pub enum TransactionState {
    Active = 0,
    Committed,
    RolledBack,
}

pub struct TransactionLogRecord {
    pub record_length: u32,
    pub lsn: u64,
    pub record_type: RecordType,
    pub transaction_id: u64,
    pub transaction_state: TransactionState,
    pub timestamp: u64,
    pub database_name: String,
    pub table_name: String,
    pub column_info: Vec<u8>, // Simplified for this example
    pub row_info: Vec<u8>,    // Simplified for this example
    pub data_length: u32,
    pub data: Vec<u8>,
    pub checksum: u32,
}