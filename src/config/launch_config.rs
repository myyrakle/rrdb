use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::constants::{
    DEFAULT_CONFIG_BASEPATH, DEFAULT_CONFIG_FILENAME, DEFAULT_DATA_DIRNAME, DEFAULT_WAL_DIRNAME,
    DEFAULT_WAL_EXTENSION,
};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LaunchConfig {
    pub port: u32,
    pub host: String,
    pub data_directory: String,

    pub wal_enabled: bool,
    pub wal_directory: String,
    pub wal_segment_size: u32,
    pub wal_extension: String,
}

#[allow(clippy::derivable_impls)]
impl std::default::Default for LaunchConfig {
    fn default() -> Self {
        let base_path = PathBuf::from(DEFAULT_CONFIG_BASEPATH);

        Self {
            port: 22208,
            host: "0.0.0.0".to_string(),
            data_directory: base_path
                .join(DEFAULT_DATA_DIRNAME)
                .to_str()
                .unwrap()
                .to_string(),
            wal_enabled: true,
            wal_directory: base_path
                .join(DEFAULT_WAL_DIRNAME)
                .to_str()
                .unwrap()
                .to_string(),
            wal_segment_size: 1024 * 1024 * 16, // 16MB 세그먼트 사이즈
            wal_extension: DEFAULT_WAL_EXTENSION.to_string(),
        }
    }
}

impl LaunchConfig {
    pub fn default_for_base_path(base_path: impl Into<PathBuf>) -> Self {
        Self::default().with_base_path(base_path)
    }

    pub fn with_base_path(mut self, base_path: impl Into<PathBuf>) -> Self {
        let base_path = absolute_path(base_path.into());

        self.data_directory = base_path
            .join(DEFAULT_DATA_DIRNAME)
            .to_str()
            .unwrap()
            .to_string();
        self.wal_directory = base_path
            .join(DEFAULT_WAL_DIRNAME)
            .to_str()
            .unwrap()
            .to_string();

        self
    }

    pub fn default_config_path() -> PathBuf {
        let base_path = PathBuf::from(DEFAULT_CONFIG_BASEPATH);
        base_path.join(DEFAULT_CONFIG_FILENAME)
    }

    pub async fn load_from_path(filepath: Option<String>) -> anyhow::Result<Self> {
        let filepath = match filepath {
            Some(path) => PathBuf::from(path),
            None => Self::default_config_path(),
        };

        let config = tokio::fs::read_to_string(filepath).await?;
        let decoded = toml::from_str(&config)?;

        Ok(decoded)
    }
}

fn absolute_path(path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .map(|current_dir| current_dir.join(&path))
            .unwrap_or(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_for_base_path_uses_base_path_for_storage_directories() {
        let config = LaunchConfig::default_for_base_path("/tmp/rrdb");

        assert_eq!(
            config.data_directory,
            PathBuf::from("/tmp/rrdb")
                .join(DEFAULT_DATA_DIRNAME)
                .to_string_lossy()
                .to_string()
        );
        assert_eq!(
            config.wal_directory,
            PathBuf::from("/tmp/rrdb")
                .join(DEFAULT_WAL_DIRNAME)
                .to_string_lossy()
                .to_string()
        );
    }

    #[test]
    fn with_base_path_overrides_loaded_storage_directories() {
        let mut config = LaunchConfig::default();
        config.data_directory = "/var/lib/rrdb/data".to_string();
        config.wal_directory = "/var/lib/rrdb/wal".to_string();

        let config = config.with_base_path("/tmp/rrdb");

        assert_eq!(
            config.data_directory,
            PathBuf::from("/tmp/rrdb")
                .join(DEFAULT_DATA_DIRNAME)
                .to_string_lossy()
                .to_string()
        );
        assert_eq!(
            config.wal_directory,
            PathBuf::from("/tmp/rrdb")
                .join(DEFAULT_WAL_DIRNAME)
                .to_string_lossy()
                .to_string()
        );
    }

    #[test]
    fn with_base_path_converts_relative_path_to_absolute_storage_directories() {
        let config = LaunchConfig::default_for_base_path("local-test");
        let base_path = std::env::current_dir().unwrap().join("local-test");

        assert_eq!(
            config.data_directory,
            base_path
                .join(DEFAULT_DATA_DIRNAME)
                .to_string_lossy()
                .to_string()
        );
        assert_eq!(
            config.wal_directory,
            base_path
                .join(DEFAULT_WAL_DIRNAME)
                .to_string_lossy()
                .to_string()
        );
    }
}
