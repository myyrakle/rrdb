use std::path::PathBuf;

use crate::executor::predule::Executor;

impl Executor {
    // 데이터 저장 경로를 지정합니다.
    pub fn get_base_path(&self) -> PathBuf {
        PathBuf::from(get_target_basepath())
    }
}
