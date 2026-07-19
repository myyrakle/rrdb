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

/// Bytes buffered in the current segment since the last fsync. Once this
/// threshold is crossed, `write_entry` performs a group-commit fsync instead
/// of waiting for the next checkpoint or periodic sync. Keeping this well
/// below the default 16MB segment size bounds how much data a crash between
/// syncs can lose without forcing a disk flush on every single write.
const GROUP_COMMIT_THRESHOLD_BYTES: usize = 64 * 1024;

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
    /// Bytes written to the current segment file since the last fsync
    /// (group commit). Reset on every sync/checkpoint/rotation.
    unsynced_bytes: usize,
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
            unsynced_bytes: 0,
        }
    }

    /// Entries written to the current segment but not yet checkpointed.
    /// Used at startup to replay operations that may not have been applied
    /// before a crash.
    pub fn pending_entries(&self) -> &[WALEntry] {
        &self.buffers
    }

    /// The sequence number of the segment currently being written to.
    pub fn current_sequence(&self) -> usize {
        self.sequence
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
        let mut frame = Vec::new();
        frame.extend_from_slice(&0u32.to_le_bytes());
        self.encoder.encode_into(&mut frame, &entry)?;
        let frame_len = u32::try_from(frame.len() - size_of::<u32>())
            .map_err(|_| WALError::wrap("wal entry is too large".to_string()))?;
        frame[..size_of::<u32>()].copy_from_slice(&frame_len.to_le_bytes());

        self.rotate_if_needed(frame.len()).await?;
        self.append_frame_to_mmap(&frame).await?;

        self.unsynced_bytes += frame.len();
        if self.unsynced_bytes >= GROUP_COMMIT_THRESHOLD_BYTES {
            self.sync_current_file().await?;
            self.unsynced_bytes = 0;
        }

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
            self.unsynced_bytes = 0;
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
        self.unsynced_bytes = 0;
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

    /// Force an fsync of the current segment now, without writing a
    /// checkpoint entry or rotating. Intended for a periodic background task
    /// so group-commit durability doesn't depend solely on hitting the byte
    /// threshold or the next checkpoint. No-op if nothing is unsynced.
    pub async fn sync(&mut self) -> errors::Result<()> {
        if self.unsynced_bytes == 0 {
            return Ok(());
        }

        self.sync_current_file().await?;
        self.unsynced_bytes = 0;

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
    use crate::common::fs::{FileSystemEntry, MockFileSystem};
    use crate::config::launch_config::LaunchConfig;
    use crate::engine::wal::endec::WALDecoder;
    use crate::engine::wal::endec::implements::bincode::{BincodeDecoder, BincodeEncoder};
    use crate::engine::wal::manager::builder::WALBuilder;
    use crate::engine::wal::types::{EntryType, WALEntry};
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
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

    fn encode_entries(entries: &[WALEntry]) -> Vec<u8> {
        let encoder = BincodeEncoder::new();
        let mut content = Vec::new();

        for entry in entries {
            let encoded = encoder.encode(entry).unwrap();
            content.extend_from_slice(&u32::try_from(encoded.len()).unwrap().to_le_bytes());
            content.extend_from_slice(&encoded);
        }

        content
    }

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
    async fn test_build_reads_segments_through_injected_file_system() {
        let config = get_test_config(Path::new("/virtual/wal"));
        let segment_1 = PathBuf::from("/virtual/wal/00000001.waltest");
        let segment_2 = PathBuf::from("/virtual/wal/00000002.waltest");
        let content_1 = encode_entries(&[create_entry(EntryType::Insert, Some("first"))]);
        let content_2 = encode_entries(&[create_entry(EntryType::Delete, Some("second"))]);
        let mut file_system = MockFileSystem::new();

        let listed_segment_1 = segment_1.clone();
        let listed_segment_2 = segment_2.clone();
        file_system
            .expect_read_dir()
            .with(mockall::predicate::eq(config.wal_directory.clone()))
            .times(1)
            .return_once(move |_| {
                Ok(vec![
                    FileSystemEntry {
                        path: listed_segment_2,
                        is_file: true,
                    },
                    FileSystemEntry {
                        path: listed_segment_1,
                        is_file: true,
                    },
                ])
            });
        file_system.expect_read().times(2).returning(move |path| {
            if path.ends_with("00000001.waltest") {
                Ok(content_1.clone())
            } else {
                Ok(content_2.clone())
            }
        });

        let wal_manager = WALBuilder::with_file_system(&config, Arc::new(file_system))
            .build(BincodeDecoder::new(), BincodeEncoder::new())
            .await
            .unwrap();

        let payloads: Vec<_> = wal_manager
            .pending_entries()
            .iter()
            .map(|entry| entry.data.clone().unwrap())
            .collect();
        assert_eq!(payloads, vec![b"first".to_vec(), b"second".to_vec()]);
    }

    #[tokio::test]
    async fn test_build_propagates_injected_directory_read_error() {
        let config = get_test_config(Path::new("/virtual/wal"));
        let mut file_system = MockFileSystem::new();
        file_system
            .expect_read_dir()
            .times(1)
            .return_once(|_| Err(std::io::Error::other("directory unavailable")));

        let error = match WALBuilder::with_file_system(&config, Arc::new(file_system))
            .build(BincodeDecoder::new(), BincodeEncoder::new())
            .await
        {
            Ok(_) => panic!("directory read error was ignored"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("directory unavailable"));
    }

    #[tokio::test]
    async fn test_build_preserves_segment_path_for_injected_read_error() {
        let config = get_test_config(Path::new("/virtual/wal"));
        let segment = PathBuf::from("/virtual/wal/00000001.waltest");
        let mut file_system = MockFileSystem::new();
        file_system.expect_read_dir().times(1).return_once({
            let segment = segment.clone();
            move |_| {
                Ok(vec![FileSystemEntry {
                    path: segment,
                    is_file: true,
                }])
            }
        });
        file_system
            .expect_read()
            .times(1)
            .return_once(|_| Err(std::io::Error::other("segment unavailable")));

        let error = match WALBuilder::with_file_system(&config, Arc::new(file_system))
            .build(BincodeDecoder::new(), BincodeEncoder::new())
            .await
        {
            Ok(_) => panic!("segment read error was ignored"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("00000001.waltest"));
        assert!(error.to_string().contains("segment unavailable"));
    }

    #[tokio::test]
    async fn test_build_single_file_with_checkpoint() {
        let wal_dir = setup_test_wal_dir("single_file_checkpoint").await;
        let config = get_test_config(&wal_dir);

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
    async fn test_build_loads_pending_entries_from_all_segments() {
        let wal_dir = setup_test_wal_dir("multi_segment_pending").await;
        let config = get_test_config(&wal_dir);
        write_wal_file(
            &config,
            1,
            &vec![create_entry(EntryType::Insert, Some("segment-1"))],
        )
        .await;
        write_wal_file(
            &config,
            2,
            &vec![create_entry(EntryType::Delete, Some("segment-2"))],
        )
        .await;

        let wal_manager = WALBuilder::new(&config)
            .build(BincodeDecoder::new(), BincodeEncoder::new())
            .await
            .unwrap();

        let payloads: Vec<_> = wal_manager
            .pending_entries()
            .iter()
            .map(|entry| entry.data.clone().unwrap())
            .collect();
        assert_eq!(payloads, vec![b"segment-1".to_vec(), b"segment-2".to_vec()]);
        assert_eq!(wal_manager.current_sequence(), 2);
    }

    #[tokio::test]
    async fn test_build_keeps_only_entries_after_last_checkpoint_across_segments() {
        let wal_dir = setup_test_wal_dir("checkpoint_across_segments").await;
        let config = get_test_config(&wal_dir);
        write_wal_file(
            &config,
            1,
            &vec![
                create_entry(EntryType::Insert, Some("durable")),
                create_entry(EntryType::Checkpoint, None),
            ],
        )
        .await;
        write_wal_file(
            &config,
            2,
            &vec![create_entry(EntryType::Set, Some("pending"))],
        )
        .await;

        let wal_manager = WALBuilder::new(&config)
            .build(BincodeDecoder::new(), BincodeEncoder::new())
            .await
            .unwrap();

        assert_eq!(wal_manager.pending_entries().len(), 1);
        assert!(matches!(
            wal_manager.pending_entries()[0].entry_type,
            EntryType::Set
        ));
        assert_eq!(
            wal_manager.pending_entries()[0].data,
            Some(b"pending".to_vec())
        );
    }

    #[tokio::test]
    async fn test_build_rejects_corrupt_intermediate_segment() {
        let wal_dir = setup_test_wal_dir("corrupt_intermediate_segment").await;
        let config = get_test_config(&wal_dir);
        write_wal_file(
            &config,
            1,
            &vec![create_entry(EntryType::Insert, Some("first"))],
        )
        .await;
        tokio::fs::write(
            wal_dir.join(format!("00000002.{}", config.wal_extension)),
            [8, 0, 0, 0, 1, 2],
        )
        .await
        .unwrap();
        write_wal_file(
            &config,
            3,
            &vec![create_entry(EntryType::Delete, Some("last"))],
        )
        .await;

        let error = match WALBuilder::new(&config)
            .build(BincodeDecoder::new(), BincodeEncoder::new())
            .await
        {
            Ok(_) => panic!("corrupt intermediate WAL segment was accepted"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("00000002"));
        assert!(error.to_string().contains("truncated wal frame body"));
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
        assert_eq!(wal_manager.buffers.len(), 1);
        assert_eq!(
            wal_manager.current_offset,
            wal_manager.current_segment.as_ref().unwrap().offset
        );
    }
}
