pub mod builder;

use std::fs::OpenOptions;
use std::path::PathBuf;
use std::time::SystemTime;

use crate::errors;
use crate::errors::wal_errors::WALError;
use memmap2::{MmapMut, MmapOptions};
use tokio::task;

use super::{
    endec::WALEncoder,
    types::{EntryType, WALEntry},
};

#[derive(Debug)]
pub struct WALManager<T>
where
    T: WALEncoder<WALEntry>,
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
    current_segment: Option<WALSegmentWriter>,
    current_offset: usize,
}

#[derive(Debug)]
struct WALSegmentWriter {
    file: std::fs::File,
    mmap: MmapMut,
    offset: usize,
}

// TODO: gz 압축 구현
// TODO: 대용량 페이지 파일 XLOG_CONTINUATION 처리 구현
impl<T> WALManager<T>
where
    T: WALEncoder<WALEntry>,
{
    fn new(
        sequence: usize,
        entries: Vec<WALEntry>,
        current_offset: usize,
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
            current_segment: None,
            current_offset,
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

    pub(crate) fn pending_entries(&self) -> &[WALEntry] {
        &self.buffers
    }

    async fn write_entry(&mut self, entry: WALEntry) -> errors::Result<()> {
        let mut frame = Vec::new();
        frame.extend_from_slice(&0u32.to_le_bytes());
        self.encoder.encode_into(&mut frame, &entry)?;
        let frame_len = u32::try_from(frame.len() - size_of::<u32>())
            .map_err(|_| WALError::wrap("wal entry is too large".to_string()))?;
        frame[..size_of::<u32>()].copy_from_slice(&frame_len.to_le_bytes());

        self.rotate_if_needed(frame.len()).await?;
        self.append_frame_to_mmap(&frame).await?;

        self.buffers.push(entry);

        Ok(())
    }

    async fn rotate_if_needed(&mut self, incoming_size: usize) -> errors::Result<()> {
        if incoming_size > self.page_size {
            return Err(WALError::wrap(format!(
                "wal frame is larger than segment size: {} > {}",
                incoming_size, self.page_size
            )));
        }

        if self.current_offset > 0 && self.current_offset + incoming_size > self.page_size {
            self.sync_current_file().await?;
            self.current_segment = None;
            self.sequence += 1;
            self.buffers.clear();
            self.current_offset = 0;
        }

        Ok(())
    }

    fn current_path(&self) -> PathBuf {
        self.directory
            .join(format!("{:08X}.{}", self.sequence, self.extension))
    }

    async fn append_frame_to_mmap(&mut self, frame: &[u8]) -> errors::Result<()> {
        self.open_current_segment_if_needed().await?;

        let segment = self
            .current_segment
            .as_mut()
            .ok_or_else(|| WALError::wrap("wal segment is not open".to_string()))?;
        let end_offset = segment.offset + frame.len();

        if end_offset > self.page_size {
            return Err(WALError::wrap("wal segment overflow".to_string()));
        }

        segment.mmap[segment.offset..end_offset].copy_from_slice(frame);
        segment.offset = end_offset;
        self.current_offset = end_offset;

        Ok(())
    }

    async fn open_current_segment_if_needed(&mut self) -> errors::Result<()> {
        if self.current_segment.is_some() {
            return Ok(());
        }

        let path = self.current_path();
        let page_size = self.page_size;
        let offset = self.current_offset;
        let segment = task::spawn_blocking(move || -> errors::Result<WALSegmentWriter> {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| WALError::wrap(e.to_string()))?;
            }

            let file = OpenOptions::new()
                .create(true)
                .read(true)
                .write(true)
                .truncate(false)
                .open(&path)
                .map_err(|e| WALError::wrap(e.to_string()))?;
            file.set_len(page_size as u64)
                .map_err(|e| WALError::wrap(e.to_string()))?;
            let mmap = unsafe {
                MmapOptions::new()
                    .len(page_size)
                    .map_mut(&file)
                    .map_err(|e| WALError::wrap(e.to_string()))?
            };

            Ok(WALSegmentWriter { file, mmap, offset })
        })
        .await
        .map_err(|e| WALError::wrap(e.to_string()))??;

        self.current_segment = Some(segment);
        Ok(())
    }

    pub(crate) async fn sync_current_file(&mut self) -> errors::Result<()> {
        let Some(segment) = self.current_segment.take() else {
            return Ok(());
        };

        let (segment, sync_error) = task::spawn_blocking(move || {
            let sync_error = segment
                .mmap
                .flush()
                .err()
                .or_else(|| segment.file.sync_data().err())
                .map(|error| error.to_string());

            (segment, sync_error)
        })
        .await
        .map_err(|e| WALError::wrap(e.to_string()))?;

        self.current_segment = Some(segment);
        if let Some(error) = sync_error {
            return Err(WALError::wrap(error));
        }

        Ok(())
    }

    async fn checkpoint(&mut self) -> errors::Result<()> {
        self.append_record(EntryType::Checkpoint, None, None)
            .await?;
        self.sync_current_file().await?;
        self.current_segment = None;
        self.sequence += 1;
        self.buffers.clear();
        self.current_offset = 0;

        Ok(())
    }

    pub async fn flush(&mut self) -> errors::Result<()> {
        if self.buffers.is_empty() && self.current_segment.is_none() {
            return Ok(());
        }

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
            let encoded_data = encoder.encode(entry).unwrap();
            file.write_all(&u32::try_from(encoded_data.len()).unwrap().to_le_bytes())
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
        wal_manager.sync_current_file().await.unwrap();

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
    async fn test_append_record_writes_single_entry_frames() {
        let wal_dir = setup_test_wal_dir("append_record_single_entry_frames").await;
        let config = get_test_config(&wal_dir);

        let builder = WALBuilder::new(&config);
        let encoder = BincodeEncoder::new();
        let decoder = BincodeDecoder::new();
        let mut wal_manager = builder.build(decoder, encoder).await.unwrap();

        wal_manager
            .append_record(EntryType::Insert, Some(b"row-1".to_vec()), Some(1))
            .await
            .unwrap();
        wal_manager.sync_current_file().await.unwrap();

        let wal_path = wal_dir.join(format!("00000001.{}", config.wal_extension));
        let content = tokio::fs::read(wal_path).await.unwrap();
        let frame_len =
            u32::from_le_bytes(content[..size_of::<u32>()].try_into().unwrap()) as usize;
        let entry: WALEntry =
            bincode::deserialize(&content[size_of::<u32>()..size_of::<u32>() + frame_len]).unwrap();

        assert!(matches!(entry.entry_type, EntryType::Insert));
        assert_eq!(entry.transaction_id, Some(1));
        assert_eq!(entry.data, Some(b"row-1".to_vec()));
    }

    #[tokio::test]
    async fn test_append_record_appends_to_mmap_segment_without_pending_write_buffer() {
        let wal_dir = setup_test_wal_dir("append_record_mmap_segment").await;
        let config = get_test_config(&wal_dir);

        let builder = WALBuilder::new(&config);
        let encoder = BincodeEncoder::new();
        let decoder = BincodeDecoder::new();
        let mut wal_manager = builder.build(decoder, encoder).await.unwrap();

        wal_manager
            .append_record(EntryType::Insert, Some(b"row-1".to_vec()), Some(1))
            .await
            .unwrap();

        assert!(wal_manager.current_segment.is_some());
        assert!(wal_manager.current_offset > 0);

        let wal_path = wal_dir.join(format!("00000001.{}", config.wal_extension));
        let metadata = tokio::fs::metadata(wal_path).await.unwrap();
        assert_eq!(metadata.len(), config.wal_segment_size as u64);
    }

    #[tokio::test]
    async fn test_build_restores_append_offset_from_mmap_segment() {
        let wal_dir = setup_test_wal_dir("build_restores_mmap_offset").await;
        let config = get_test_config(&wal_dir);

        let decoder = BincodeDecoder::new();
        let encoder = BincodeEncoder::new();
        let mut wal_manager = WALBuilder::new(&config)
            .build(decoder.clone(), encoder)
            .await
            .unwrap();

        wal_manager
            .append_record(EntryType::Insert, Some(b"row-1".to_vec()), Some(1))
            .await
            .unwrap();
        wal_manager.sync_current_file().await.unwrap();
        let first_offset = wal_manager.current_offset;
        drop(wal_manager);

        let mut rebuilt = WALBuilder::new(&config)
            .build(decoder.clone(), BincodeEncoder::new())
            .await
            .unwrap();

        assert_eq!(rebuilt.sequence, 1);
        assert_eq!(rebuilt.current_offset, first_offset);

        rebuilt
            .append_record(EntryType::Delete, Some(b"row-2".to_vec()), Some(2))
            .await
            .unwrap();
        rebuilt.sync_current_file().await.unwrap();

        let wal_path = wal_dir.join(format!("00000001.{}", config.wal_extension));
        let content = tokio::fs::read(wal_path).await.unwrap();
        let entries = decoder.decode(&content).unwrap();

        assert_eq!(entries.len(), 2);
        assert!(matches!(entries[0].entry_type, EntryType::Insert));
        assert!(matches!(entries[1].entry_type, EntryType::Delete));
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
