use std::path::PathBuf;

use crate::{executor::predule::Executor, utils::path::get_target_basepath};

impl Executor {
    pub fn get_base_path(&self) -> PathBuf {
        PathBuf::from(get_target_basepath())
    }
}
