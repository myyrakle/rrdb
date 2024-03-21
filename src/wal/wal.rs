use tokio::fs;

use crate::utils::path::get_target_basepath;

use super::{format::{BinaryFormatterImpl, LogFileHeader, MAGIC_NUMBER, VERSION}, record::LogRecord};

const WAL_METADATA: &str = "current_wal_index";

#[derive(Clone, Debug, Default)]
pub struct WalManager {
    logs: Vec<LogRecord>,
    last_file_index: u64,
    last_lsn: u64
}

impl WalManager {
    pub async fn new() -> Self {
        // wal 메타데이터 파일을 읽어서 index와 lsn을 결정함 
        let mut wal = WalManager::default();

        let mut wal_metadata = get_target_basepath();
        wal_metadata.push(WAL_METADATA);

        match wal_metadata.exists() {
            true => {
                let lsn_data = fs::read(wal_metadata).await.expect("Cannot read wal metadata file");
                wal.last_file_index = u64::from_be_bytes(lsn_data[0..8].try_into().expect("Metadata file is incorrect"));
            },
            false => {
                wal.last_file_index = 1;
                fs::write(wal_metadata, wal.last_lsn.to_be_bytes()).await.expect("Cannot write wal metadata file");
            }
        }

        todo!()
    }

    pub fn insert(&mut self, record: LogRecord) {
        self.logs.push(record);
    }

    pub async fn flush() -> Result<(), std::io::Error> {
        todo!()
        // let log: Vec<u8> = Vec::new();
        // let formatter = BinaryFormatterImpl::new();
        

        // let header = LogFileHeader {
        //    magic_number: MAGIC_NUMBER,
        //    version: VERSION,
        // };

        // // fs::metadata

        // // formatter.write_log_file_header(&mut log, )
        // // fs::write(path, contents).await?
        // Ok(())
    }
}