use std::time::SystemTime;
#[allow(dead_code)]
#[allow(unused_variables)]
#[allow(unused_assignments)]
#[allow(unused_imports)]

use std::{fs, io::BufWriter, path::PathBuf};

use crate::{errors::{predule::WALError, RRDBError}, executor::config::global::GlobalConfig};

use super::{endec::{WALDecoder, WALEncoder}, types::{EntryType, WALEntry}};

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
// TODO: 단순히 이름을  wal{}. 형식으로 로깅하지 말고, 체계적인 파일 관리 구현
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
        let path = self.directory.join(format!("{}.{}", self.sequence, self.extension));

        let encoded = self.encoder.encode(&self.buffers)?;

        fs::write(&path, encoded).map_err(|e| WALError::wrap(e.to_string()))?;

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


pub struct WALBuilder<'a, T>
where
    T: WALDecoder<Vec<WALEntry>>,
{
    config: &'a GlobalConfig,
    decoder: T,
}

impl<'a, T> WALBuilder<'a, T>
where
    T: WALDecoder<Vec<WALEntry>>,
{
    pub fn new(config: &'a GlobalConfig, decoder: T) -> Self {
        Self {
            config,
            decoder,
        }
    }

    pub async fn build<D>(&self, encoder: D) -> Result<WALManager<D>, RRDBError>
    where
        D: WALEncoder<Vec<WALEntry>>,
    {
        let (sequence, entries) = self.load_data().await?;

        Ok(WALManager::new(
            sequence,
            entries,
            self.config.wal_segment_size as usize,
            PathBuf::from(self.config.wal_directory.clone()),
            self.config.wal_extension.to_string(),
            encoder,
        ))
    }

    async fn load_data(&self) -> Result<(usize, Vec<WALEntry>), RRDBError> {
        let mut sequence = 1;

        // get all log file entry
        let logs = std::fs::read_dir(&self.config.wal_directory)
            .map_err(|e| WALError::wrap(e.to_string()))?
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension() == Some(self.config.wal_extension.as_ref()))
            .collect::<Vec<_>>();

        let mut entries = Vec::new();

        if let Some(last_log) = logs.last() {
            sequence = logs.len();

            let content = std::fs::read(last_log.path())
                .map_err(|e| WALError::wrap(e.to_string()))?;
            let saved_entries: Vec<WALEntry> = self.decoder.decode(&content)?;

            match saved_entries.last() {
                Some(entry)
                    if matches!(entry.entry_type, EntryType::Checkpoint) => entries = saved_entries,
                _ => (),
            }
        }

        Ok((sequence, entries))
    }
}
