pub mod builder;

use std::time::SystemTime;
#[allow(dead_code)]
#[allow(unused_variables)]
#[allow(unused_assignments)]
#[allow(unused_imports)]
use std::{fs, io::BufWriter, path::PathBuf};

use crate::errors;
use crate::errors::wal_errors::WALError;

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

    pub fn append(&mut self, mut entry: WALEntry) -> errors::Result<()> {
        let entire_data_size = self.buffers.iter().map(|entry| entry.size()).sum::<usize>();

        // 원본 WAL 객체 정보
        let original_entry_type = entry.entry_type.clone();
        let original_timestamp = entry.timestamp;
        let original_transaction_id = entry.transaction_id;
        let original_entry_full_size = entry.size();
        let data_option = entry.data.take();

        // 데이터 분배 필요 여부 확인
        if entire_data_size + original_entry_full_size > self.page_size {
            if let Some(mut distributed_entry_data) = data_option {
                let mut first_chunk = true;

                // 데이터 분배
                while !distributed_entry_data.is_empty() {
                    let chunk_size = std::cmp::min(
                        distributed_entry_data.len(),
                        self.page_size - entire_data_size,
                    );

                    let chunk: Vec<u8> = distributed_entry_data.drain(..chunk_size).collect();

                    self.buffers.push(WALEntry {
                        entry_type: original_entry_type.clone(),
                        data: Some(chunk),
                        timestamp: original_timestamp,
                        transaction_id: original_transaction_id,
                        is_continuation: !first_chunk,
                    });

                    first_chunk = false;
                }
            } else {
                self.buffers.push(WALEntry {
                    entry_type: original_entry_type,
                    data: None,
                    timestamp: original_timestamp,
                    transaction_id: original_transaction_id,
                    is_continuation: false,
                });
            }
        } else {
            self.buffers.push(WALEntry {
                entry_type: original_entry_type,
                data: data_option,
                timestamp: original_timestamp,
                transaction_id: original_transaction_id,
                is_continuation: false,
            });
        }

        Ok(())
    }

    async fn save_to_file(&mut self) -> errors::Result<()> {
        let path = self
            .directory
            .join(format!("{:08X}.{}", self.sequence, self.extension));

        let encoded = self.encoder.encode(&self.buffers)?;

        tokio::fs::write(&path, encoded)
            .await
            .map_err(|e| WALError::wrap(e.to_string()))?;

        // fsync 디스크 동기화 보장
        let file = tokio::fs::OpenOptions::new()
            .write(true)
            .open(path)
            .await
            .map_err(|e| WALError::wrap(e.to_string()))?;
        file.sync_all()
            .await
            .map_err(|e| WALError::wrap(e.to_string()))?;

        Ok(())
    }

    async fn checkpoint(&mut self) -> errors::Result<()> {
        self.buffers.push(WALEntry {
            data: None,
            entry_type: EntryType::Checkpoint,
            timestamp: Self::get_current_secs()?,
            transaction_id: None,
            is_continuation: false,
        });
        self.save_to_file().await?;

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
    use crate::engine::wal::endec::implements::bitcode::{BitcodeDecoder, BitcodeEncoder};
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
        let wal_dir = PathBuf::from(format!("target/test_wal_data/{}", test_name));
        if wal_dir.exists() {
            tokio::fs::remove_dir_all(&wal_dir).await.unwrap_or_else(|e| {
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
        let encoder = BitcodeEncoder::new();
        let encoded_data = encoder.encode(entries).unwrap();
        let file_path = PathBuf::from(&config.wal_directory)
            .join(format!("{:08X}.{}", sequence, config.wal_extension));

        let mut file = tokio::fs::File::create(&file_path)
            .await
            .unwrap_or_else(|e| panic!("Failed to create wal file {:?}: {}", file_path, e));
        file.write_all(&encoded_data)
            .await
            .unwrap_or_else(|e| panic!("Failed to write to wal file {:?}: {}", file_path, e));
        file.sync_all()
            .await
            .unwrap_or_else(|e| panic!("Failed to sync wal file {:?}: {}", file_path, e));
    }

    #[tokio::test]
    async fn test_build_no_wal_files() {
        let wal_dir = setup_test_wal_dir("no_wal_files").await;
        let config = get_test_config(&wal_dir);

        let builder = WALBuilder::new(&config);
        let encoder = BitcodeEncoder::new();
        let decoder = BitcodeDecoder::new();

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
        let encoder = BitcodeEncoder::new();
        let decoder = BitcodeDecoder::new();

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
    async fn test_build_multiple_files() {
        let wal_dir = setup_test_wal_dir("multiple_files").await;

        // 일부러 페이지 사이즈를 작게 설정
        let mut config = get_test_config(&wal_dir);
        config.wal_segment_size = 20; // 20 바이트

        let builder = WALBuilder::new(&config);
        let encoder = BitcodeEncoder::new();
        let decoder = BitcodeDecoder::new();

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
