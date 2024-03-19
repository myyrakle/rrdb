// Binary formatter for WAL implementation
// WAL의 바이너리 포매터입니다.

use std::io::{self, Write};

use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader};

use super::transaction::TransactionLogRecord;

pub struct BinaryFormatterImpl {}

const MAGIC_NUMBER: u32 = 0x1234ABCD;
const VERSION: u16 = 1;

pub struct LogFileHeader {
    pub magic_number: u32,
    pub version: u16,
    pub log_file_id: u64,
    pub checksum: u32,
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

impl BinaryFormatterImpl {
    pub fn new() -> Self {
        BinaryFormatterImpl {}
    }
        
    pub async fn write_log_file_header<W: AsyncWriteExt + Unpin>(&self, writer: &mut W, header: &LogFileHeader) -> io::Result<()> {
        writer.write_all(&header.magic_number.to_be_bytes()).await?;
        writer.write_all(&header.version.to_be_bytes()).await?;
        writer.write_all(&header.log_file_id.to_be_bytes()).await?;
        writer.write_all(&header.checksum.to_be_bytes()).await
    }

    pub async fn write_transaction_log_record<W: AsyncWriteExt + Unpin>(&self, writer: &mut W, record: &TransactionLogRecord) -> io::Result<()> {
        writer.write_all(&record.record_length.to_be_bytes()).await?;
        writer.write_all(&record.lsn.to_be_bytes()).await?;
        writer.write_all(&[record.record_type as u8]).await?;
        writer.write_all(&record.transaction_id.to_be_bytes()).await?;
        writer.write_all(&[record.transaction_state as u8]).await?;
        writer.write_all(&record.timestamp.to_be_bytes()).await?;

        let db_name_bytes = record.database_name.as_bytes();
        writer.write_all(&(db_name_bytes.len() as u16).to_be_bytes()).await?;
        writer.write_all(db_name_bytes).await?;

        let table_name_bytes = record.table_name.as_bytes();
        writer.write_all(&(table_name_bytes.len() as u16).to_be_bytes()).await?;
        writer.write_all(table_name_bytes).await?;

        writer.write_all(&(record.column_info.len() as u16).to_be_bytes()).await?;
        writer.write_all(&record.column_info).await?;

        writer.write_all(&(record.row_info.len() as u32).to_be_bytes()).await?;
        writer.write_all(&record.row_info).await?;

        writer.write_all(&record.data_length.to_be_bytes()).await?;
        writer.write_all(&record.data).await?;

        writer.write_all(&record.checksum.to_be_bytes()).await
    }
}

#[cfg(test)]
mod format_test {
    use crate::wal::transaction::TransactionState;

    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn test_write_log_file_header() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let mut buffer = vec![];
            let header = LogFileHeader {
                magic_number: MAGIC_NUMBER,
                version: VERSION,
                log_file_id: 12345678,
                checksum: 87654321,
            };

            let formatter = BinaryFormatterImpl::new();
            formatter.write_log_file_header(&mut buffer, &header).await.unwrap();

            assert_eq!(buffer.len(), 4 + 2 + 8 + 4);
            assert_eq!(&buffer[0..4], MAGIC_NUMBER.to_be_bytes().as_ref());
            assert_eq!(&buffer[4..6], VERSION.to_be_bytes().as_ref());
            assert_eq!(&buffer[6..14], header.log_file_id.to_be_bytes().as_ref());
            assert_eq!(&buffer[14..18], header.checksum.to_be_bytes().as_ref());

            println!("{:?}", &buffer)
        });
    }

    #[test]
    fn test_write_transaction_log_record() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let mut buffer = vec![];
            let record = TransactionLogRecord {
                record_length: 1024,
                lsn: 123456789012345,
                record_type: RecordType::Insert,
                transaction_id: 987654321,
                transaction_state: TransactionState::Active,
                timestamp: 98765432109876,
                database_name: "test_db".to_string(),
                table_name: "test_table".to_string(),
                column_info: vec![1, 2, 3],
                row_info: vec![4, 5, 6, 7],
                data_length: 4,
                data: vec![8, 9, 10, 11],
                checksum: 12345678,
            };

            let formatter = BinaryFormatterImpl::new();
            formatter.write_transaction_log_record(&mut buffer, &record).await.unwrap();

            assert!(buffer.len() > 0); 
            assert_eq!(&buffer[0..4], record.record_length.to_be_bytes().as_ref()); 

            println!("{:?}", buffer)
        });
    }
}
