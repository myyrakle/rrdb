use crate::ast::types::DataType;

#[derive(Clone, Copy)]
pub enum TransactionState {
    Active = 0,
    Committed,
    RolledBack,
}

#[derive(Clone, Copy)]
pub enum RecordType {
    Insert = 0,
    Update,
    Delete,
    Begin,
    Commit,
    Rollback,
}

#[derive(Clone, Debug)]
pub struct ColumnInfo {
    pub name: String,
    pub column_type: DataType,
    pub length: Option<u32>,
}

#[derive(Clone, Debug)]
pub struct RowData {
    pub columns: Vec<ColumnInfo>,
    pub values: Vec<Vec<u8>>, 
}

pub struct LogRecord {
    pub record_length: u32,
    pub lsn: u64,
    pub record_type: RecordType,
    pub transaction_id: u64,
    pub transaction_state: TransactionState,
    pub timestamp: u64,
    pub database_name: String,
    pub table_name: String,
    pub column_info: Vec<ColumnInfo>,
    pub row_info: RowData, 
    pub data_length: u32,
    pub data: Vec<u8>,
    pub checksum: u32,
}