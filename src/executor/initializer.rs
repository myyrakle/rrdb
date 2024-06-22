use std::path::PathBuf;

use crate::ast::ddl::create_database::CreateDatabaseQuery;
use crate::constants::{
    DEFAULT_CONFIG_BASEPATH, DEFAULT_CONFIG_FILENAME, DEFAULT_DATABASE_NAME, DEFAULT_DATA_DIRNAME,
};
use crate::errors::execute_error::ExecuteError;
use crate::errors::RRDBError;

use super::config::global::GlobalConfig;
use super::predule::Executor;

impl Executor {
    // 기본 설정파일 세팅
    pub async fn init(&self) -> Result<(), RRDBError> {
        // 1. 루트 디렉터리 생성 (없다면)
        let base_path = PathBuf::from(DEFAULT_CONFIG_BASEPATH);
        if let Err(error) = tokio::fs::create_dir(base_path.clone()).await {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                // Do Nothing
            } else {
                println!("path {:?}", base_path.clone());
                println!("error: {:?}", error.to_string());
                return Err(ExecuteError::new(error.to_string()));
            }
        }

        // 2. 전역 설정파일 생성 (없다면)
        let mut global_path = base_path.clone();
        global_path.push(DEFAULT_CONFIG_FILENAME);

        if let Err(error) = tokio::fs::create_dir(global_path.parent().unwrap().to_path_buf()).await
        {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                // Do Nothing
            } else {
                return Err(ExecuteError::new(error.to_string()));
            }
        }

        let global_info = GlobalConfig::default();
        let global_config = toml::to_string(&global_info).unwrap();

        if let Err(error) = tokio::fs::write(global_path, global_config.as_bytes()).await {
            return Err(ExecuteError::new(error.to_string()));
        }

        // 3. 데이터 디렉터리 생성 (없다면)
        let mut data_path = base_path.clone();
        data_path.push(DEFAULT_DATA_DIRNAME);

        if let Err(error) = tokio::fs::create_dir(data_path).await {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                // Do Nothing
            } else {
                return Err(ExecuteError::new(error.to_string()));
            }
        }

        // 4. 기본 데이터베이스 생성 (rrdb)
        self.create_database(
            CreateDatabaseQuery::builder()
                .set_name(DEFAULT_DATABASE_NAME.into())
                .set_if_not_exists(true),
        )
        .await?;

        Ok(())
    }
}
