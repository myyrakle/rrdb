use crate::lib::config::GlobalConfig;
use std::path::PathBuf;

pub struct Executor {}

impl Executor {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn init(&self, path: String) -> Result<(), Box<dyn std::error::Error>> {
        let mut path_buf = PathBuf::new();
        path_buf.push(path);

        path_buf.push(".rrdb.config");
        (match tokio::fs::create_dir(path_buf.clone()).await {
            Ok(_) => Ok(()),
            Err(error) => {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    Ok(())
                } else {
                    Err(error)
                }
            }
        })?;

        path_buf.push(".global.config");
        let global_info = GlobalConfig::default();
        let global_config = toml::to_string(&global_info).unwrap();
        tokio::fs::write(path_buf, global_config.as_bytes()).await?;

        Ok(())
    }
}
