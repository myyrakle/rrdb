use std::path::PathBuf;
use std::process::{Command, Output};

use std::io::Error;

use crate::ast::ddl::create_database::CreateDatabaseQuery;
use crate::constants::{DEFAULT_CONFIG_BASEPATH, DEFAULT_CONFIG_FILENAME, DEFAULT_DATABASE_NAME};
use crate::errors::execute_error::ExecuteError;
use crate::errors::RRDBError;

use super::config::global::GlobalConfig;
use super::predule::Executor;

#[cfg(target_os = "macos")]
use crate::constants::LAUNCHD_PLIST_PATH;

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
            if error.kind() != std::io::ErrorKind::AlreadyExists {
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

    #[cfg(target_os = "linux")]
    async fn create_daemon_config_if_not_exists(&self) -> Result<(), RRDBError> {
        let base_path = PathBuf::from("/etc/systemd/system/rrdb.service");
        let script = r#"[Unit]
Description=RRDB

[Service]
Type=simple
Restart=on-failure
ExecStart=/usr/bin/rrdb run
RemainAfterExit=on
User=root
StandardOutput=file:/var/log/rrdb.stdout.log
StandardError=file:/var/log/rrdb.stderr.log

[Install]
WantedBy=multi-user.target"#;

        self.write_and_check_err(base_path, script).await
    }

    #[cfg(target_os = "macos")]
    async fn create_daemon_config_if_not_exists(&self) -> Result<(), RRDBError> {
        let base_path = PathBuf::from(LAUNCHD_PLIST_PATH);
        let script = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
        <key>Label</key>
        <string>myyrakle.github.io.rrdb</string>
        <key>UserName</key>
        <string>root</string>
        <key>Program</key>
        <string>/usr/local/bin/rrdb</string>
        <key>ProgramArguments</key>
        <array>
            <string>run</string>
        </array>
        <key>RunAtLoad</key>
        <true/>
        <key>StandardOutPath</key>
        <string>/var/log/rrdb.stdout.log</string>
        <key>StandardErrorPath</key>
        <string>/var/log/rrdb.stderr.log</string>
</dict>
</plist>"#;

        self.write_and_check_err(base_path, script).await
    }

    #[cfg(target_os = "windows")]
    async fn create_daemon_config_if_not_exists(&self) -> Result<(), RRDBError> {
        Command::new("winget").args(["install", "--accept-package-agreements", "nssm"]);

        let output = Command::new("nssm.exe").args(["install", "rrdb", "C:\\Program Files\\rrdb\\rrdb.exe", "run"]).output();

        self.check_output_status(output)
    }

    async fn write_and_check_err(
        &self,
        base_path: PathBuf,
        contents: &str,
    ) -> Result<(), RRDBError> {
        if let Err(error) = tokio::fs::write(base_path, contents).await {
            if error.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(ExecuteError::wrap(error.to_string()));
            }
        }
        Ok(())
    }

    async fn start_daemon(&self) -> Result<(), RRDBError> {
        let (program, args) = self.get_daemon_start_command();
        let output = Command::new(program).args(args).output();

        self.check_output_status(output)
    }

    #[cfg(target_os = "linux")]
    fn get_daemon_start_command(&self) -> (&'static str, Vec<&'static str>) {
        ("systemctl", vec!["enable", "--now", "rrdb.service"])
    }

    #[cfg(target_os = "macos")]
    fn get_daemon_start_command(&self) -> (&'static str, Vec<&'static str>) {
        ("launchctl", vec!["load", LAUNCHD_PLIST_PATH])
    }

    #[cfg(target_os = "windows")]
    fn get_daemon_start_command(&self) -> (&'static str, Vec<&'static str>) {
        ("sc.exe", vec!["start", "rrdb"])
    }

    fn check_output_status(&self, output: Result<Output, Error>) -> Result<(), RRDBError> {
        if output.is_err() {
            Err(ExecuteError::wrap("failed to start daemon"))
        } else {
            Ok(())
        }
    }
}
