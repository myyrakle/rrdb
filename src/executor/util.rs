use std::path::PathBuf;

use crate::executor::predule::Executor;

impl Executor {
    // 데이터 저장 경로를 반환합니다..
    pub fn get_data_directory(&self) -> PathBuf {
        PathBuf::from(self.config.data_directory.clone())
    }
}
