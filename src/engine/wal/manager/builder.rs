use std::path::PathBuf;
use std::sync::Arc;

use crate::common::fs::{FileSystem, RealFileSystem};
use crate::config::launch_config::LaunchConfig;
use crate::engine::wal::endec::{WALDecoder, WALEncoder};
use crate::engine::wal::types::{EntryType, WALEntry};
use crate::errors;
use crate::errors::wal_errors::WALError;

use super::WALManager;

pub struct WALBuilder<'a> {
    config: &'a LaunchConfig,
    file_system: Arc<dyn FileSystem + Send + Sync>,
}

impl<'a> WALBuilder<'a> {
    pub fn new(config: &'a LaunchConfig) -> Self {
        Self::with_file_system(config, Arc::new(RealFileSystem))
    }

    pub fn with_file_system(
        config: &'a LaunchConfig,
        file_system: Arc<dyn FileSystem + Send + Sync>,
    ) -> Self {
        Self {
            config,
            file_system,
        }
    }

    pub async fn build<T, D>(&self, decoder: T, encoder: D) -> errors::Result<WALManager<D>>
    where
        T: WALDecoder<Vec<WALEntry>>,
        D: WALEncoder<WALEntry>,
    {
        let (sequence, entries, current_offset) = self.load_data(decoder).await?;

        Ok(WALManager::new(
            sequence,
            entries,
            current_offset,
            self.config.wal_segment_size as usize,
            PathBuf::from(self.config.wal_directory.clone()),
            self.config.wal_extension.to_string(),
            encoder,
        ))
    }

    async fn load_data<T>(&self, decoder: T) -> errors::Result<(usize, Vec<WALEntry>, usize)>
    where
        T: WALDecoder<Vec<WALEntry>>,
    {
        let mut segments = Vec::new();

        let dir_entries = self
            .file_system
            .read_dir(&self.config.wal_directory)
            .await
            .map_err(|e| WALError::wrap(e.to_string()))?;

        for entry in dir_entries {
            if !entry.is_file {
                continue;
            }
            let path = entry.path;

            // 파일 명 16진수로 변환
            let sequence = path
                .extension()
                .filter(|ext_osstr| ext_osstr.to_str() == Some(self.config.wal_extension.as_str()))
                .and_then(|_| path.file_stem())
                .and_then(|stem_osstr| stem_osstr.to_str())
                .and_then(|stem_str| usize::from_str_radix(stem_str, 16).ok());

            if let Some(sequence) = sequence {
                segments.push((sequence, path));
            }
        }

        segments.sort_by_key(|(sequence, _)| *sequence);

        let Some((max_sequence, _)) = segments.last() else {
            return Ok((1, Vec::new(), 0));
        };
        let max_sequence = *max_sequence;
        let mut pending_entries = Vec::new();
        let mut newest_ends_with_checkpoint = false;
        let mut newest_offset = 0;

        for (sequence, path) in segments {
            let content = self.file_system.read(&path).await.map_err(|e| {
                WALError::wrap(format!("failed to read log file {:?}: {}", path, e))
            })?;
            let used_bytes = used_wal_bytes(&content).map_err(|e| {
                WALError::wrap(format!("failed to inspect log file {:?}: {}", path, e))
            })?;
            let entries: Vec<WALEntry> = decoder.decode(&content).map_err(|e| {
                WALError::wrap(format!("failed to decode log file {:?}: {}", path, e))
            })?;

            if sequence == max_sequence {
                newest_offset = used_bytes;
                newest_ends_with_checkpoint = entries
                    .last()
                    .is_some_and(|entry| matches!(entry.entry_type, EntryType::Checkpoint));
            }

            for entry in entries {
                if matches!(entry.entry_type, EntryType::Checkpoint) {
                    pending_entries.clear();
                } else {
                    pending_entries.push(entry);
                }
            }
        }

        if newest_ends_with_checkpoint {
            Ok((max_sequence + 1, pending_entries, 0))
        } else {
            Ok((max_sequence, pending_entries, newest_offset))
        }
    }
}

fn used_wal_bytes(content: &[u8]) -> errors::Result<usize> {
    let mut offset = 0;

    while offset < content.len() {
        if content.len() - offset < size_of::<u32>() {
            if content[offset..].iter().all(|byte| *byte == 0) {
                return Ok(offset);
            }

            return Err(WALError::wrap("truncated wal frame header".to_string()));
        }

        let frame_len = u32::from_le_bytes(
            content[offset..offset + size_of::<u32>()]
                .try_into()
                .map_err(|e| WALError::wrap(format!("{:?}", e)))?,
        ) as usize;

        if frame_len == 0 {
            return Ok(offset);
        }

        offset += size_of::<u32>();

        if content.len() - offset < frame_len {
            return Err(WALError::wrap("truncated wal frame body".to_string()));
        }

        offset += frame_len;
    }

    Ok(offset)
}
