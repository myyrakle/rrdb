use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::constants::{DEFAULT_CONFIG_BASEPATH, DEFAULT_CONFIG_FILENAME, DEFAULT_DATA_DIRNAME};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GlobalConfig {
    pub port: u32,
    pub host: String,
    pub data_directory: String,
}

#[allow(clippy::derivable_impls)]
impl std::default::Default for GlobalConfig {
    fn default() -> Self {
        let base_path = PathBuf::from(DEFAULT_CONFIG_BASEPATH);

        Self {
            port: 55555,
            host: "0.0.0.0".to_string(),
            data_directory: base_path
                .join(DEFAULT_DATA_DIRNAME)
                .to_str()
                .unwrap()
                .to_string(),
        }
    }
}

impl GlobalConfig {
    pub fn load_from_path(filepath: Option<String>) -> Self {
        let filepath = match filepath {
            Some(path) => PathBuf::from(path),
            None => {
                let path = PathBuf::new();
                let path = path.join(DEFAULT_CONFIG_BASEPATH);
                let path = path.join(DEFAULT_CONFIG_FILENAME);

                path
            }
        };

        let config = std::fs::read_to_string(filepath).unwrap();
        toml::from_str(&config).unwrap()
    }
}
