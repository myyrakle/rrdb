use std::path::PathBuf;
use std::process::Output;

use crate::ast::ddl::create_database::CreateDatabaseQuery;
use crate::constants::{
    DEFAULT_CONFIG_BASEPATH, DEFAULT_CONFIG_FILENAME, DEFAULT_DATABASE_NAME, LAUNCHD_PLIST_PATH,
};
use crate::errors::execute_error::ExecuteError;
use crate::errors::RRDBError;

use super::config::global::GlobalConfig;
use super::predule::Executor;

impl Executor {
    // 기본 설정파일 세팅
    pub async fn init(&self) -> Result<(), RRDBError> {
        // 1. 루트 디렉터리 생성 (없다면)
        self.create_top_level_directory_if_not_exists().await?;

        // 2. 전역 설정파일 생성 (없다면)
        self.create_global_config_if_not_exists().await?;

        // 3. 데이터 디렉터리 생성 (없다면)
        self.create_data_directory_if_not_exists().await?;

        // 4. 데몬 설정파일 생성 (없다면)
        self.create_daemon_config_if_not_exists().await?;

        // 5. 기본 데이터베이스 생성 (rrdb)
        self.create_database(
            CreateDatabaseQuery::builder()
                .set_name(DEFAULT_DATABASE_NAME.into())
                .set_if_not_exists(true),
        )
        .await?;

        // 6. 데몬 실행
        self.start_daemon().await?;

        Ok(())
    }

    async fn create_top_level_directory_if_not_exists(&self) -> Result<(), RRDBError> {
        let base_path = PathBuf::from(DEFAULT_CONFIG_BASEPATH);

        if let Err(error) = tokio::fs::create_dir(base_path.clone()).await {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                println!("path {:?}", base_path.clone());
                println!("error: {:?}", error.to_string());
                return Err(ExecuteError::wrap(error.to_string()));
            }
        }

        Ok(())
    }

    async fn create_global_config_if_not_exists(&self) -> Result<(), RRDBError> {
        let base_path = PathBuf::from(DEFAULT_CONFIG_BASEPATH);

        let mut global_path = base_path.clone();
        global_path.push(DEFAULT_CONFIG_FILENAME);

        if let Err(error) = tokio::fs::create_dir(global_path.parent().unwrap().to_path_buf()).await
        {
            if error.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(ExecuteError::wrap(error.to_string()));
            }
        }

        let global_info = GlobalConfig::default();
        let global_config = toml::to_string(&global_info).unwrap();

        if let Err(error) = tokio::fs::write(global_path, global_config.as_bytes()).await {
            return Err(ExecuteError::wrap(error.to_string()));
        }

        Ok(())
    }

    async fn create_data_directory_if_not_exists(&self) -> Result<(), RRDBError> {
        let data_path = self.config.data_directory.clone();

        if let Err(error) = tokio::fs::create_dir(data_path).await {
            if error.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(ExecuteError::wrap(error.to_string()));
            }
        }

        Ok(())
    }

    async fn create_daemon_config_if_not_exists(&self) -> Result<(), RRDBError> {
        if cfg!(target_os = "linux") {
            let base_path = PathBuf::from("/etc/systemd/system/rrdb.service");
            let script = r#"[Unit]
Description=RRDB

[Service]
Type=simple
Restart=on-failure
ExecStart=/usr/bin/rrdb run
RemainAfterExit=on
User=root

[Install]
WantedBy=multi-user.target"#;

            if let Err(error) = tokio::fs::write(base_path, script).await {
                if error.kind() != std::io::ErrorKind::AlreadyExists {
                    return Err(ExecuteError::wrap(error.to_string()));
                }
            }
        }
        if cfg!(target_os = "macos") {
            let base_path = PathBuf::from(LAUNCHD_PLIST_PATH);
            let script = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
        <key>Label</key>
        <string>myyrakle.github.io.rrdb</string>
        <key>Program</key>
        <string>/usr/local/bin/rrdb</string>
        <key>RunAtLoad</key>
        <true/>
        <key>StandardOutPath</key>
        <string>/var/log/rrdb.log</string>
</dict>
</plist>"#;

            if let Err(error) = tokio::fs::write(base_path, script).await {
                if error.kind() != std::io::ErrorKind::AlreadyExists {
                    return Err(ExecuteError::wrap(error.to_string()));
                }
            }
        }
        Ok(())
    }

    async fn start_daemon(&self) -> Result<(), RRDBError> {
        let mut output: Option<Output> = None;
        if cfg!(target_os = "linux") {
            output = Some(
                std::process::Command::new("systemctl")
                    .arg("enable")
                    .arg("--now")
                    .arg("rrdb.service")
                    .output()
                    .expect("failed to start daemon"),
            );
        }
        if cfg!(target_os = "macos") {
            output = Some(
                std::process::Command::new("launchctl")
                    .arg("load")
                    .arg(LAUNCHD_PLIST_PATH)
                    .output()
                    .expect("failed to start daemon"),
            );
        }
        if output.is_some() && output.unwrap().status.success() {
            return Err(ExecuteError::wrap("failed to start daemon"));
        }
        Ok(())
    }
}
