#[allow(dead_code)]
use std::{fs, io::BufWriter, path::PathBuf};

use crate::errors::predule::WALError;

use super::types::WALEntry;

pub struct WALManager {
    sequence: usize,
    page_size: usize,
    current_file: Option<BufWriter<fs::File>>,
    current_size: usize,

    directory: PathBuf,
}

// TODO: gz 압축 구현
// TODO: 대용량 파일 TOAST 등 처리 방법 고려
impl WALManager {
    pub fn new(path: String) -> Self {
        Self {
            sequence: 0,
            page_size: 4096,
            current_file: None,
            current_size: 0,
            directory: PathBuf::from(path),
        }
    }

    pub fn append(&mut self, entry: WALEntry) -> Result<(), WALError> {
        todo!()
    }

    pub fn flush(&mut self) -> Result<(), WALError> {
        todo!()
    }

    fn read_entries<F>(&self, path: &PathBuf, mut callback: F) -> Result<(), WALError>
    where
        F: FnMut(&WALEntry) -> Result<(), WALError>,
    {
        todo!()
    }
}
