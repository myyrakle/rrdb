use std::time::SystemTime;
#[allow(dead_code)]
#[allow(unused_variables)]
#[allow(unused_assignments)]
#[allow(unused_imports)]
use std::{fs, io::BufWriter, path::PathBuf};

use crate::{
    errors::{predule::WALError, RRDBError},
    executor::config::global::GlobalConfig,
};

use super::{
    endec::{WALDecoder, WALEncoder},
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

    pub fn append(&mut self, entry: WALEntry) -> Result<(), RRDBError> {
        self.buffers.push(entry);

        self.check_and_mark()?;
        Ok(())
    }

    fn check_and_mark(&mut self) -> Result<(), RRDBError> {
        let size = self.buffers.iter().map(|entry| entry.size()).sum::<usize>();

        if size > self.page_size {
            self.checkpoint()?;

            self.buffers.clear();
            self.sequence += 1;
        }

        Ok(())
    }

    fn save_to_file(&mut self) -> Result<(), RRDBError> {
        let path = self
            .directory
            .join(format!("{:08X}.{}", self.sequence, self.extension));

        let encoded = self.encoder.encode(&self.buffers)?;

        fs::write(&path, encoded)
            .map_err(|e| WALError::wrap(e.to_string()))?;

        // fsync 디스크 동기화 보장
        let file = fs::OpenOptions::new()
            .write(true)
            .open(path)
            .map_err(|e| WALError::wrap(e.to_string()))?;
        file.sync_all().map_err(|e| WALError::wrap(e.to_string()))?;

        Ok(())
    }

    fn checkpoint(&mut self) -> Result<(), RRDBError> {
        self.buffers.push(WALEntry {
            data: None,
            entry_type: EntryType::Checkpoint,
            timestamp: Self::get_current_secs()?,
            transaction_id: None,
        });
        self.save_to_file()?;

        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), RRDBError> {
        self.checkpoint()?;
        Ok(())
    }

    fn get_current_secs() -> Result<u128, RRDBError> {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| WALError::wrap(e.to_string()))
            .map(|duration| duration.as_millis())
    }
}

pub struct WALBuilder<'a> {
    config: &'a GlobalConfig,
}

impl<'a> WALBuilder<'a> {
    pub fn new(config: &'a GlobalConfig) -> Self {
        Self { config }
    }

    pub async fn build<T, D>(&self, decoder: T, encoder: D) -> Result<WALManager<D>, RRDBError>
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

    async fn load_data<T>(&self, decoder: T) -> Result<(usize, Vec<WALEntry>), RRDBError>
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
            let wrapped_parsed_seq = path.extension()
                .filter(|ext_osstr| ext_osstr.to_str() == Some(self.config.wal_extension.as_str()))
                .and_then(|_| path.file_stem())
                .and_then(|stem_osstr| stem_osstr.to_str())
                .and_then(|stem_str| usize::from_str_radix(stem_str, 16).ok());

            if let Some(seq) = wrapped_parsed_seq {
                if seq > max_sequence {
                    max_sequence = seq;
                    last_log_path = Some(path);
                }
            }
        }

        let (current_sequence, entries) = last_log_path.map_or_else(
            // Case 1: WAL 파일이 하나도 없는 초기 상태
            || {
                // 첫 번째 WAL 파일이므로 시퀀스는 1로 시작하고, 복구할 엔트리는 없음
                Ok((1, Vec::new()))
            },

            // Case 2: 최신 WAL 파일이 존재하는 상태
            |log_path| {
                // 최신 WAL 파일의 내용을 읽음
                let content = std::fs::read(&log_path).map_err(|e|
                    WALError::wrap(format!(
                        "failed to read log file {:?}: {}",
                        log_path, e.to_string()
                    ))
                )?;

                // 파일 내용이 비어있는 경우 복구할 엔트리는 없음 
                if content.is_empty() {
                    return Ok((max_sequence + 1, Vec::new()))
                }

                let saved_entries: Vec<WALEntry> = decoder.decode(&content).map_err(|e|
                    WALError::wrap(format!(
                        "failed to decode log file {:?}: {}",
                        log_path,
                        e.to_string()
                    ))
                )?;

                let last_entry = match saved_entries.last() {
                    Some(entry) => entry,
                    None => return Ok((max_sequence + 1, Vec::new())),
                };

                let next_sequence = if matches!(last_entry.entry_type, EntryType::Checkpoint) {
                    max_sequence + 1
                } else {
                    // 마지막 엔트리가 체크포인트가 아니면 비정상 종료로 간주
                    max_sequence
                };

                // 로드된 엔트리들과 결정된 시퀀스 번호를 반환
                Ok((next_sequence, saved_entries))
            },
        )?;

        Ok((current_sequence, entries))
    }
}
