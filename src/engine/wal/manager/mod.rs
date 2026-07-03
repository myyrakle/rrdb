pub mod builder;

use std::io::ErrorKind;
use std::path::PathBuf;
use std::time::SystemTime;

use crate::errors;
use crate::errors::wal_errors::WALError;
use tokio::io::AsyncWriteExt;

use super::{
    endec::WALEncoder,
    types::{EntryType, WALEntry},
};

#[derive(Default, Debug, Clone)]
pub struct WALManager<T>
where
    T: WALEncoder<Vec<WALEntry>>,
{
    /// The sequence number of the WAL file
    sequence: usize,
    /// The buffer of the WAL file
    buffers: Vec<WALEntry>,
    /// The page size of the WAL file
    page_size: usize,
    /// The directory of the WAL file
    directory: PathBuf,
    /// The extension of the WAL file
    extension: String,
    encoder: T,
}

// TODO: gz 압축 구현
// TODO: 대용량 페이지 파일 XLOG_CONTINUATION 처리 구현
impl<T> WALManager<T>
where
    T: WALEncoder<Vec<WALEntry>>,
{
    fn new(
        sequence: usize,
        entries: Vec<WALEntry>,
        page_size: usize,
        directory: PathBuf,
        extension: String,
        encoder: T,
    ) -> Self {
        Self {
            sequence,
            buffers: entries,
            page_size,
            directory,
            extension,
            encoder,
        }
    }

    pub async fn append(&mut self, entry: WALEntry) -> errors::Result<()> {
        self.write_entry(entry).await
    }

    pub async fn append_record(
        &mut self,
        entry_type: EntryType,
        data: Option<Vec<u8>>,
        transaction_id: Option<u64>,
    ) -> errors::Result<()> {
        self.append(WALEntry {
            data,
            entry_type,
            timestamp: Self::get_current_secs()?,
            transaction_id,
            is_continuation: false,
        })
        .await
    }

    async fn write_entry(&mut self, entry: WALEntry) -> errors::Result<()> {
        self.rotate_if_needed(entry.size()).await?;

        let encoded = self.encoder.encode(&vec![entry.clone()])?;
        let frame_len = u32::try_from(encoded.len())
            .map_err(|_| WALError::wrap("wal entry is too large".to_string()))?;

        let mut frame = Vec::with_capacity(size_of::<u32>() + encoded.len());
        frame.extend_from_slice(&frame_len.to_le_bytes());
        frame.extend_from_slice(&encoded);

        let path = self.current_path();

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await
            .map_err(|e| WALError::wrap(e.to_string()))?;

        file.write_all(&frame)
            .await
            .map_err(|e| WALError::wrap(e.to_string()))?;
        file.flush()
            .await
            .map_err(|e| WALError::wrap(e.to_string()))?;

        self.buffers.push(entry);

        Ok(())
    }

    async fn rotate_if_needed(&mut self, incoming_size: usize) -> errors::Result<()> {
        let current_size = self.buffers.iter().map(|entry| entry.size()).sum::<usize>();

        if !self.buffers.is_empty() && current_size + incoming_size > self.page_size {
            self.sync_current_file().await?;
            self.sequence += 1;
            self.buffers.clear();
        }

        Ok(())
    }

    fn current_path(&self) -> PathBuf {
        self.directory
            .join(format!("{:08X}.{}", self.sequence, self.extension))
    }

    async fn sync_current_file(&self) -> errors::Result<()> {
        let path = self.current_path();

        match tokio::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .await
        {
            Ok(file) => file
                .sync_data()
                .await
                .map_err(|e| WALError::wrap(e.to_string()))?,
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => return Err(WALError::wrap(error.to_string())),
        }

        Ok(())
    }

    async fn checkpoint(&mut self) -> errors::Result<()> {
        self.append_record(EntryType::Checkpoint, None, None)
            .await?;
        self.sync_current_file().await?;
        self.sequence += 1;
        self.buffers.clear();

        Ok(())
    }

    pub async fn flush(&mut self) -> errors::Result<()> {
        self.checkpoint().await?;
        Ok(())
    }

    fn get_current_secs() -> errors::Result<u128> {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| WALError::wrap(e.to_string()))
            .map(|duration| duration.as_millis())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::launch_config::LaunchConfig;
    use crate::engine::wal::endec::WALDecoder;
    use crate::engine::wal::endec::implements::bincode::{BincodeDecoder, BincodeEncoder};
    use crate::engine::wal::manager::builder::WALBuilder;
    use crate::engine::wal::types::{EntryType, WALEntry};
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};
    use tokio::io::AsyncWriteExt;

    fn get_test_config(wal_dir_path: &Path) -> LaunchConfig {
        LaunchConfig {
            port: 22208,
            host: "127.0.0.1".to_string(),
            data_directory: "./test_db_data".to_string(),
            wal_enabled: true,
            wal_directory: wal_dir_path.to_str().unwrap().to_string(),
            wal_segment_size: 1024,
            wal_extension: "waltest".to_string(),
        }
    }

    async fn setup_test_wal_dir(test_name: &str) -> PathBuf {
        let test_binary = std::env::current_exe()
            .ok()
            .and_then(|path| path.file_stem().map(|stem| stem.to_os_string()))
            .and_then(|stem| stem.into_string().ok())
            .unwrap_or_else(|| "unknown".to_string());
        let wal_dir = PathBuf::from("target")
            .join("test_wal_data")
            .join(test_binary)
            .join(test_name);
        if wal_dir.exists() {
            tokio::fs::remove_dir_all(&wal_dir)
                .await
                .unwrap_or_else(|e| {
                    panic!("Failed to remove old test WAL dir {:?}: {}", wal_dir, e)
                });
        }
        tokio::fs::create_dir_all(&wal_dir)
            .await
            .unwrap_or_else(|e| panic!("Failed to create test WAL dir {:?}: {}", wal_dir, e));
        wal_dir
    }

    fn create_entry(entry_type: EntryType, data_str: Option<&str>) -> WALEntry {
        WALEntry {
            entry_type,
            data: data_str.map(|s| s.as_bytes().to_vec()),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            transaction_id: None,
            is_continuation: false,
        }
    }

    // WAL 파일에 엔트리들을 기록하는 헬퍼 함수
    async fn write_wal_file(config: &LaunchConfig, sequence: usize, entries: &Vec<WALEntry>) {
        let encoder = BincodeEncoder::new();
        let file_path = PathBuf::from(&config.wal_directory)
            .join(format!("{:08X}.{}", sequence, config.wal_extension));

        let mut file = tokio::fs::File::create(&file_path)
            .await
            .unwrap_or_else(|e| panic!("Failed to create wal file {:?}: {}", file_path, e));

        for entry in entries {
            let encoded_data = encoder.encode(&vec![entry.clone()]).unwrap();
            file.write_all(&(encoded_data.len() as u32).to_le_bytes())
                .await
                .unwrap_or_else(|e| panic!("Failed to write to wal file {:?}: {}", file_path, e));
            file.write_all(&encoded_data)
                .await
                .unwrap_or_else(|e| panic!("Failed to write to wal file {:?}: {}", file_path, e));
        }

        file.sync_all()
            .await
            .unwrap_or_else(|e| panic!("Failed to sync wal file {:?}: {}", file_path, e));
    }

    #[tokio::test]
    async fn test_build_no_wal_files() {
        let wal_dir = setup_test_wal_dir("no_wal_files").await;
        let config = get_test_config(&wal_dir);

        let builder = WALBuilder::new(&config);
        let encoder = BincodeEncoder::new();
        let decoder = BincodeDecoder::new();

        let wal_manager = builder.build(decoder, encoder).await.unwrap();

        assert_eq!(
            wal_manager.sequence, 1,
            "Sequence should be 1 when no WAL files exist"
        );
        assert!(
            wal_manager.buffers.is_empty(),
            "Buffers should be empty when no WAL files exist"
        );
    }

    #[tokio::test]
    async fn test_build_single_file_with_checkpoint() {
        let wal_dir = setup_test_wal_dir("single_file_checkpoint").await;
        let config = get_test_config(&wal_dir);

        // 테스트용 WAL 파일 생성 (시퀀스 1, 마지막은 체크포인트)
        let entries_seq1 = vec![
            create_entry(EntryType::Insert, Some("data1")),
            create_entry(EntryType::Set, Some("data2")),
            create_entry(EntryType::Checkpoint, None),
        ];
        write_wal_file(&config, 1, &entries_seq1).await;

        let builder = WALBuilder::new(&config);
        let encoder = BincodeEncoder::new();
        let decoder = BincodeDecoder::new();

        let wal_manager = builder.build(decoder, encoder).await.unwrap();

        // 시퀀스는 2여야 하고, 버퍼는 비어있어야 함 (체크포인트 완료)
        assert_eq!(
            wal_manager.sequence, 2,
            "Sequence should be 2 after a checkpointed file"
        );
        assert!(
            wal_manager.buffers.is_empty(),
            "Buffers should be empty after a checkpointed file"
        );
    }

    #[tokio::test]
    async fn test_append_record_writes_framed_bincode_entries() {
        let wal_dir = setup_test_wal_dir("append_record_framed_bincode").await;
        let config = get_test_config(&wal_dir);

        let builder = WALBuilder::new(&config);
        let encoder = BincodeEncoder::new();
        let decoder = BincodeDecoder::new();
        let mut wal_manager = builder.build(decoder.clone(), encoder).await.unwrap();

        wal_manager
            .append_record(EntryType::Insert, Some(b"row-1".to_vec()), Some(1))
            .await
            .unwrap();
        wal_manager
            .append_record(EntryType::Delete, Some(b"row-2".to_vec()), Some(2))
            .await
            .unwrap();

        let wal_path = wal_dir.join(format!("00000001.{}", config.wal_extension));
        let content = tokio::fs::read(wal_path).await.unwrap();
        let entries = decoder.decode(&content).unwrap();

        assert_eq!(entries.len(), 2);
        assert!(matches!(entries[0].entry_type, EntryType::Insert));
        assert_eq!(entries[0].transaction_id, Some(1));
        assert_eq!(entries[0].data, Some(b"row-1".to_vec()));
        assert!(matches!(entries[1].entry_type, EntryType::Delete));
        assert_eq!(entries[1].transaction_id, Some(2));
    }

    #[tokio::test]
    async fn test_build_multiple_files() {
        let wal_dir = setup_test_wal_dir("multiple_files").await;

        // 일부러 페이지 사이즈를 작게 설정
        let mut config = get_test_config(&wal_dir);
        config.wal_segment_size = 20; // 20 바이트

        let builder = WALBuilder::new(&config);
        let encoder = BincodeEncoder::new();
        let decoder = BincodeDecoder::new();

        let wal_manager = builder
            .build(decoder, encoder)
            .await
            .expect("Failed to build WAL manager");

        assert_eq!(wal_manager.sequence, 1, "Sequence should be 1");

        // 여러개로 분산 처리 되는지 확인
        let entries_seq1 = vec![
            create_entry(EntryType::Insert, Some("helloworld")), // 10바이트
            create_entry(EntryType::Set, Some("data2")),         // 5바이트
        ];
        write_wal_file(&config, 1, &entries_seq1).await;

        // 여기서 기본 페이지 사이즈보다 크게
        let entries_seq2 = vec![
            create_entry(EntryType::Insert, Some("helloworld")), // 10바이트
            create_entry(EntryType::Set, Some("data2")),         // 5바이트
        ];
        write_wal_file(&config, 2, &entries_seq2).await;
    }
}
