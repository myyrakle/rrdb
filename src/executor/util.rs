use std::path::PathBuf;

use crate::{executor::predule::Executor, utils::env::get_system_env};

impl Executor {
    pub fn get_base_path(&self) -> PathBuf {
        PathBuf::from(get_system_env("RRDB_BASE_PATH"))
    }
}
