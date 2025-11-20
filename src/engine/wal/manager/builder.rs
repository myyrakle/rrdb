use std::path::PathBuf;

use crate::config::launch_config::LaunchConfig;
use crate::engine::wal::endec::{WALDecoder, WALEncoder};
use crate::engine::wal::types::{EntryType, WALEntry};
use crate::errors::Errors;
use crate::errors::wal_errors::WALError;

use super::WALManager;

pub struct WALBuilder<'a> {
    config: &'a LaunchConfig,
}

impl<'a> WALBuilder<'a> {
    pub fn new(config: &'a LaunchConfig) -> Self {
        Self { config }
    }

    pub async fn build<T, D>(&self, decoder: T, encoder: D) -> Result<WALManager<D>, Errors>
    where
        T: WALDecoder<Vec<WALEntry>>,
        D: WALEncoder<Vec<WALEntry>>,
    {
        let (sequence, entries) = self.load_data(decoder).await?;

        Ok(WALManager::new(
            sequence,
            entries,
            self.config.wal_segment_size as usize,
            PathBuf::from(self.config.wal_directory.clone()),
            self.config.wal_extension.to_string(),
            encoder,
        ))
    }

    async fn load_data<T>(&self, decoder: T) -> Result<(usize, Vec<WALEntry>), Errors>
    where
        T: WALDecoder<Vec<WALEntry>>,
    {
        let mut max_sequence = 0;
        let mut last_log_path: Option<PathBuf> = None;

        let dir_entries = std::fs::read_dir(&self.config.wal_directory)
            .map_err(|e| WALError::wrap(e.to_string()))?;

        for entry_result in dir_entries {
            let path = match entry_result {
                Ok(entry) => entry.path(),
                Err(e) => return Err(WALError::wrap(e.to_string())),
            };

            if !path.is_file() {
                continue;
            }

            // 파일 명 16진수로 변환
            let wrapped_parsed_seq = path
                .extension()
                .filter(|ext_osstr| ext_osstr.to_str() == Some(self.config.wal_extension.as_str()))
                .and_then(|_| path.file_stem())
                .and_then(|stem_osstr| stem_osstr.to_str())
                .and_then(|stem_str| usize::from_str_radix(stem_str, 16).ok());

            if let Some(seq) = wrapped_parsed_seq
                && seq > max_sequence
            {
                max_sequence = seq;
                last_log_path = Some(path);
            }
        }

        let (current_sequence, entries) = last_log_path.map_or_else(
            // Case 1: WAL 파일이 하나도 없는 초기 상태
            || {
                // 첫 번째 WAL 파일이므로 시퀀스는 1로 시작하고, 복구할 엔트리는 없음
                Ok::<(usize, Vec<WALEntry>), Errors>((1, Vec::new()))
            },
            // Case 2: 최신 WAL 파일이 존재하는 상태
            |log_path| {
                // 최신 WAL 파일의 내용을 읽음
                let content = std::fs::read(&log_path).map_err(|e| {
                    WALError::wrap(format!("failed to read log file {:?}: {}", log_path, e))
                })?;

                // 파일 내용이 비어있는 경우 복구할 엔트리는 없음
                if content.is_empty() {
                    return Ok((max_sequence + 1, Vec::new()));
                }

                let saved_entries: Vec<WALEntry> = decoder.decode(&content).map_err(|e| {
                    WALError::wrap(format!(
                        "failed to decode log file {:?}: {}",
                        log_path,
                        e.to_string()
                    ))
                })?;

                let last_entry = match saved_entries.last() {
                    Some(entry) => entry,
                    None => return Ok((max_sequence + 1, Vec::new())),
                };

                if matches!(last_entry.entry_type, EntryType::Checkpoint) {
                    Ok((max_sequence + 1, Vec::new()))
                } else {
                    // 마지막 엔트리가 체크포인트가 아니면 비정상 종료로 간주
                    Ok((max_sequence, saved_entries))
                }
            },
        )?;

        Ok((current_sequence, entries))
    }
}
