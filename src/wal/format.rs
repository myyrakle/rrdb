// Binary formatter for WAL implementation
// WAL의 바이너리 포매터입니다.

use std::{error::Error, io::{self, Write}};

use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader};

use super::record::LogRecord;

pub const MAGIC_NUMBER: u32 = 0x44DB;
pub const VERSION: u16 = 1;

pub struct LogFileHeader {
    pub magic_number: u32,
    pub version: u16,
    pub last_lsn: u64
}

pub struct BinaryFormatterImpl {}

impl BinaryFormatterImpl {
    pub fn new() -> Self {
        BinaryFormatterImpl {}
    }

    pub async fn write_log_file_header<W: AsyncWriteExt + Unpin>(
        &self,
        writer: &mut W,
        header: &LogFileHeader,
    ) -> io::Result<()> {
        writer.write_all(&header.magic_number.to_be_bytes()).await?;
        writer.write_all(&header.version.to_be_bytes()).await
    }

    pub async fn write_transaction_log_record<W: AsyncWriteExt + Unpin>(
        &self,
        writer: &mut W,
        record: &LogRecord,
    ) -> io::Result<()> {
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
        for column_info in &record.column_info {
            let name_bytes = column_info.name.as_bytes();
            writer.write_all(&(name_bytes.len() as u16).to_be_bytes()).await?;
            writer.write_all(name_bytes).await?;
            writer.write_all(&(column_info.column_type.type_code() as u8).to_be_bytes()).await?;
            writer.write_all(&column_info.length.unwrap_or(0).to_be_bytes()).await?;
        }

        writer.write_all(&(record.row_info.columns.len() as u32).to_be_bytes()).await?;
        for column_info in &record.row_info.columns {
            let name_bytes = column_info.name.as_bytes();
            writer.write_all(&(name_bytes.len() as u16).to_be_bytes()).await?;
            writer.write_all(name_bytes).await?;
            writer.write_all(&(column_info.column_type.type_code() as u8).to_be_bytes()).await?;
            writer.write_all(&column_info.length.unwrap_or(0).to_be_bytes()).await?;
        }

        writer.write_all(&(record.row_info.values.len() as u32).to_be_bytes()).await?;
        for values in &record.row_info.values {
            writer.write_all(&(values.len() as u32).to_be_bytes()).await?;
            writer.write_all(values).await?;
        }

        writer.write_all(&record.data_length.to_be_bytes()).await?;
        writer.write_all(&record.data).await?;
        Ok(())
    }
}

pub struct BinaryParser {
}

type Log = (LogFileHeader, Vec<LogRecord>);

impl BinaryParser {
    pub async fn read_log_from_file(path: &String) 
        -> Result<Log, Box<dyn Error>> {
        let data = tokio::fs::read(path).await?;

        let header = || -> Result<LogFileHeader, Box<dyn Error>> {
            let magic_number = u32::from_be_bytes(data[0..4].try_into()?);
            let version = u16::from_be_bytes(data[4..6].try_into()?);
            let last_lsn = u64::from_be_bytes(data[6..14].try_into()?);

            Ok(LogFileHeader {
                magic_number,
                version,
               last_lsn 
            })
        }()?;
        
        let value = (header, Vec::new());

        /// TODO
        Ok(value)
    }
}