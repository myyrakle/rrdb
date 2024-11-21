use std::time::SystemTime;
#[allow(dead_code)]
#[allow(unused_variables)]
#[allow(unused_assignments)]
#[allow(unused_imports)]

use std::{fs, io::BufWriter, path::PathBuf};

use crate::errors::{predule::WALError, RRDBError};

use super::types::{EntryType, WALEntry};

pub struct WALManager {
    sequence: usize,
    buffers: Vec<WALEntry>,
    page_size: usize,
    directory: PathBuf,
}

// TODO: gz 압축 구현
// TODO: 대용량 페이지 파일 TOAST 등 처리 방법 고려
impl WALManager {
    fn new(sequence: usize, entries: Vec<WALEntry>, page_size: usize, directory: PathBuf) -> Self {
        Self {
            sequence,
            buffers: entries,
            page_size,
            directory,
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
        }

        Ok(())
    }

    fn save_to_file(&mut self) -> Result<(), RRDBError> {
        let path = self.directory.join(format!("{}.log", self.sequence));

        let encoded = bitcode::encode(&self.buffers);

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
            timestamp: WALManager::get_current_secs()?,
            transaction_id: None,
        });
        self.save_to_file()?;

        self.buffers.clear();
        self.sequence += 1;

        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), WALError> {
        todo!()
    }

    fn get_current_secs() -> Result<f64, RRDBError> {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| WALError::wrap(e.to_string()))
            .map(|duration| duration.as_secs_f64())
    }
}


pub struct WALBuilder {
    page_size: usize,
    directory: PathBuf,
}

impl Default for WALBuilder {
    fn default() -> Self {
        Self {
            page_size: 4096,
            directory: PathBuf::from("./wal"),
        }
    }
}

impl WALBuilder {
    pub fn build(&self) -> Result<WALManager, RRDBError> {
        let (sequence, entries) = self.load_data()?;

        Ok(WALManager::new(sequence, entries, self.page_size, self.directory.clone()))
    }

    pub fn set_page_size(mut self, page_size: usize) -> Self {
        self.page_size = page_size;
        self
    }

    pub fn set_directory(mut self, directory: PathBuf) -> Self {
        self.directory = directory;
        self
    }

    fn load_data(&self) -> Result<(usize, Vec<WALEntry>), RRDBError> {
        let mut sequence = 1;

        // get all log file entry
        let logs = std::fs::read_dir(&self.directory)
            .map_err(|e| WALError::wrap(e.to_string()))?
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension() == Some("log".as_ref()))
            .collect::<Vec<_>>();

        let mut entries = Vec::new();

        if let Some(last_log) = logs.last() {
            sequence = logs.len();

            let content = std::fs::read(last_log.path())
                .map_err(|e| WALError::wrap(e.to_string()))?;
            let saved_entries: Vec<WALEntry> = bitcode::decode(&content)
                .map_err(|e| WALError::wrap(e.to_string()))?;

            match saved_entries.last() {
                Some(entry)
                    if matches!(entry.entry_type, EntryType::Checkpoint) => entries = saved_entries,
                _ => (),
            }
        }

        Ok((sequence, entries))
    }
}
