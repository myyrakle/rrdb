use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use std::io::Error;

use crate::constants::{DEFAULT_CONFIG_BASEPATH, DEFAULT_CONFIG_FILENAME, DEFAULT_DATABASE_NAME};
use crate::engine::DBEngine;
use crate::engine::ast::ddl::create_database::CreateDatabaseQuery;
use crate::errors;
use crate::errors::execute_error::ExecuteError;

#[cfg(target_os = "macos")]
use crate::constants::LAUNCHD_PLIST_PATH;

impl DBEngine {
    pub async fn initialize(&self) -> errors::Result<()> {
        self.initialize_with_base_path(None).await
    }

    pub async fn initialize_with_base_path(
        &self,
        base_path: Option<PathBuf>,
    ) -> errors::Result<()> {
        self.init_config(base_path).await?;
        self.init_database().await?;

        Ok(())
    }

    // 기본 설정파일 세팅
    async fn init_config(&self, base_path: Option<PathBuf>) -> errors::Result<()> {
        let base_path = base_path.unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_BASEPATH));
        let config_path = base_path.join(DEFAULT_CONFIG_FILENAME);

        // 1. 루트 디렉터리 생성 (없다면)
        self.create_top_level_directory_if_not_exists(&base_path)
            .await?;

        // 2. 전역 설정파일 생성 (없다면)
        self.create_global_config_if_not_exists(&config_path)
            .await?;

        // 3. 데이터 디렉터리 생성 (없다면)
        self.create_data_directory_if_not_exists().await?;

        // 4. WAL 디렉터리 생성 (없다면)
        self.create_wal_directory_if_not_exists().await?;

        Ok(())
    }

    pub async fn install_daemon(&self) -> errors::Result<()> {
        self.create_daemon_config_if_not_exists().await?;
        self.start_daemon().await
    }

    async fn init_database(&self) -> errors::Result<()> {
        // 6. 기본 데이터베이스 생성 (rrdb)
        self.create_database(
            CreateDatabaseQuery::builder()
                .set_name(DEFAULT_DATABASE_NAME.into())
                .set_if_not_exists(true),
        )
        .await?;

        Ok(())
    }

    async fn create_top_level_directory_if_not_exists(
        &self,
        base_path: &Path,
    ) -> errors::Result<()> {
        let base_path = base_path.to_str().unwrap_or_default();

        if let Err(error) = self.file_system.create_dir(base_path).await
            && error.kind() != std::io::ErrorKind::AlreadyExists
        {
            println!("path {:?}", base_path);
            println!("error: {:?}", error.to_string());
            return Err(ExecuteError::wrap(error.to_string()));
        }

        Ok(())
    }

    async fn create_global_config_if_not_exists(&self, config_path: &Path) -> errors::Result<()> {
        let global_config = toml::to_string(self.config.as_ref()).unwrap();

        if let Err(error) = self
            .file_system
            .write_file(
                config_path.to_str().unwrap_or_default(),
                global_config.as_bytes(),
            )
            .await
        {
            return Err(ExecuteError::wrap(error.to_string()));
        }

        Ok(())
    }

    async fn create_data_directory_if_not_exists(&self) -> errors::Result<()> {
        let data_path = self.config.data_directory.clone();

        if let Err(error) = self.file_system.create_dir(&data_path).await
            && error.kind() != std::io::ErrorKind::AlreadyExists
        {
            return Err(ExecuteError::wrap(error.to_string()));
        }

        Ok(())
    }

    async fn create_wal_directory_if_not_exists(&self) -> errors::Result<()> {
        let wal_path = self.config.wal_directory.clone();

        if let Err(error) = self.file_system.create_dir(&wal_path).await
            && error.kind() != std::io::ErrorKind::AlreadyExists
        {
            return Err(ExecuteError::wrap(error.to_string()));
        }

        Ok(())
    }

    #[cfg(target_os = "linux")]
    async fn create_daemon_config_if_not_exists(&self) -> errors::Result<()> {
        use crate::constants::SYSTEMD_DAEMON_SCRIPT;

        let base_path = PathBuf::from("/etc/systemd/system/rrdb.service");

        self.write_and_check_err(base_path, SYSTEMD_DAEMON_SCRIPT)
            .await
    }

    #[cfg(target_os = "macos")]
    async fn create_daemon_config_if_not_exists(&self) -> errors::Result<()> {
        use crate::constants::LAUNCHD_DAEMON_SCRIPT;

        let base_path = PathBuf::from(LAUNCHD_PLIST_PATH);

        self.write_and_check_err(base_path, LAUNCHD_DAEMON_SCRIPT)
            .await
    }

    #[cfg(target_os = "windows")]
    async fn create_daemon_config_if_not_exists(&self) -> errors::Result<()> {
        Command::new("winget").args(["install", "--accept-package-agreements", "nssm"]);

        let output = Command::new("nssm.exe")
            .args([
                "install",
                "rrdb",
                "C:\\Program Files\\rrdb\\rrdb.exe",
                "run",
            ])
            .output();

        self.check_output_status(output)
    }

    #[allow(dead_code)]
    async fn write_and_check_err(&self, base_path: PathBuf, contents: &str) -> errors::Result<()> {
        if let Err(error) = self
            .file_system
            .write_file(base_path.to_str().unwrap_or_default(), contents.as_bytes())
            .await
            && error.kind() != std::io::ErrorKind::AlreadyExists
        {
            return Err(ExecuteError::wrap(error.to_string()));
        }
        Ok(())
    }

    async fn start_daemon(&self) -> errors::Result<()> {
        let (program, args) = self.get_daemon_start_command();
        let output = self.command_runner.run(Command::new(program).args(args));

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

    fn check_output_status(&self, output: Result<Output, Error>) -> errors::Result<()> {
        if output.is_err() {
            Err(ExecuteError::wrap("failed to start daemon".to_string()))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::launch_config::LaunchConfig;
    #[cfg(target_os = "linux")]
    use crate::constants::SYSTEMD_DAEMON_SCRIPT;

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn test_init_config() {
        use mockall::predicate::eq;

        use crate::{
            common::{
                command::{CommandRunner, MockCommandRunner},
                fs::{FileSystem, MockFileSystem},
            },
            constants::{DEFAULT_DATA_DIRNAME, DEFAULT_WAL_DIRNAME},
        };

        use super::*;
        use std::sync::Arc;

        const CONFIG: &[u8] = br##"port = 22208
host = "0.0.0.0"
data_directory = "/var/lib/rrdb/data"
wal_enabled = true
wal_directory = "/var/lib/rrdb/wal"
wal_segment_size = 16777216
wal_extension = "log"
"##;

        struct TestCase {
            name: &'static str,
            want_error: bool,
            mock_file_system: Box<dyn Fn() -> Arc<dyn FileSystem + Send + Sync>>,
            mock_command_runner: Box<dyn Fn() -> Arc<dyn CommandRunner + Send + Sync>>,
            mock_config: Box<dyn Fn() -> Arc<LaunchConfig>>,
        }

        let test_cases = vec![
            TestCase {
                name: "init 정상 동작 (linux)",
                want_error: false,
                mock_config: Box::new(|| {
                    let config = LaunchConfig::default();

                    Arc::new(config)
                }),
                mock_file_system: Box::new(move || {
                    let mut mock = MockFileSystem::new();

                    // 1. 최상위 디렉터리 생성
                    mock.expect_create_dir()
                        .times(1)
                        .with(eq(DEFAULT_CONFIG_BASEPATH))
                        .returning(|_| Ok(()));

                    // 2. 전역 설정파일 생성
                    mock.expect_write_file()
                        .times(1)
                        .with(
                            eq(DEFAULT_CONFIG_BASEPATH.to_owned() + "/" + DEFAULT_CONFIG_FILENAME),
                            eq(CONFIG),
                        )
                        .returning(|_, _| Ok(()));

                    // 3. 데이터 디렉터리 생성
                    mock.expect_create_dir()
                        .times(1)
                        .with(eq(DEFAULT_CONFIG_BASEPATH.to_owned()
                            + "/"
                            + DEFAULT_DATA_DIRNAME))
                        .returning(|_| Ok(()));

                    // 4. WAL 디렉터리 생성
                    mock.expect_create_dir()
                        .times(1)
                        .with(eq(DEFAULT_CONFIG_BASEPATH.to_owned()
                            + "/"
                            + DEFAULT_WAL_DIRNAME))
                        .returning(|_| Ok(()));

                    Arc::new(mock)
                }),
                mock_command_runner: Box::new(|| {
                    let mock = MockCommandRunner::new();

                    Arc::new(mock)
                }),
            },
            TestCase {
                name: "최종 Command 실행 실패",
                want_error: true,
                mock_config: Box::new(|| {
                    let config = LaunchConfig::default();

                    Arc::new(config)
                }),
                mock_file_system: Box::new(move || {
                    let mut mock = MockFileSystem::new();

                    // 1. 최상위 디렉터리 생성
                    mock.expect_create_dir()
                        .with(eq(DEFAULT_CONFIG_BASEPATH))
                        .returning(|_| Ok(()));

                    // 2. 전역 설정파일 생성
                    mock.expect_write_file()
                        .with(
                            eq(DEFAULT_CONFIG_BASEPATH.to_owned() + "/" + DEFAULT_CONFIG_FILENAME),
                            eq(CONFIG),
                        )
                        .returning(|_, _| Ok(()));

                    // 3. 데이터 디렉터리 생성
                    mock.expect_create_dir()
                        .with(eq(DEFAULT_CONFIG_BASEPATH.to_owned()
                            + "/"
                            + DEFAULT_DATA_DIRNAME))
                        .returning(|_| Ok(()));

                    // 4. WAL 디렉터리 생성
                    mock.expect_create_dir()
                        .with(eq(DEFAULT_CONFIG_BASEPATH.to_owned()
                            + "/"
                            + DEFAULT_WAL_DIRNAME))
                        .returning(|_| Ok(()));

                    // 5. 데몬 설정파일 생성
                    mock.expect_write_file()
                        .with(
                            eq("/etc/systemd/system/rrdb.service"),
                            eq(SYSTEMD_DAEMON_SCRIPT.as_bytes()),
                        )
                        .returning(|_, _| Ok(()));

                    Arc::new(mock)
                }),
                mock_command_runner: Box::new(|| {
                    let mut mock = MockCommandRunner::new();

                    mock.expect_run()
                        .returning(|_| Err(Error::from_raw_os_error(1)));

                    Arc::new(mock)
                }),
            },
            TestCase {
                name: "WAL 디렉터리 생성 실패",
                want_error: true,
                mock_config: Box::new(|| {
                    let config = LaunchConfig::default();

                    Arc::new(config)
                }),
                mock_file_system: Box::new(move || {
                    let mut mock = MockFileSystem::new();

                    // 1. 최상위 디렉터리 생성
                    mock.expect_create_dir()
                        .times(1)
                        .with(eq(DEFAULT_CONFIG_BASEPATH))
                        .returning(|_| Ok(()));

                    // 2. 전역 설정파일 생성
                    mock.expect_write_file()
                        .times(1)
                        .with(
                            eq(DEFAULT_CONFIG_BASEPATH.to_owned() + "/" + DEFAULT_CONFIG_FILENAME),
                            eq(CONFIG),
                        )
                        .returning(|_, _| Ok(()));

                    // 3. 데이터 디렉터리 생성
                    mock.expect_create_dir()
                        .times(1)
                        .with(eq(DEFAULT_CONFIG_BASEPATH.to_owned()
                            + "/"
                            + DEFAULT_DATA_DIRNAME))
                        .returning(|_| Ok(()));

                    // 4. WAL 디렉터리 생성
                    mock.expect_create_dir()
                        .times(1)
                        .with(eq(DEFAULT_CONFIG_BASEPATH.to_owned()
                            + "/"
                            + DEFAULT_WAL_DIRNAME))
                        .returning(|_| Err(Error::from_raw_os_error(1)));

                    Arc::new(mock)
                }),
                mock_command_runner: Box::new(|| {
                    let mock = MockCommandRunner::new();

                    Arc::new(mock)
                }),
            },
            TestCase {
                name: "데몬 설정파일 생성 실패",
                want_error: true,
                mock_config: Box::new(|| {
                    let config = LaunchConfig::default();

                    Arc::new(config)
                }),
                mock_file_system: Box::new(move || {
                    let mut mock = MockFileSystem::new();

                    // 1. 최상위 디렉터리 생성
                    mock.expect_create_dir()
                        .times(1)
                        .with(eq(DEFAULT_CONFIG_BASEPATH))
                        .returning(|_| Ok(()));

                    // 2. 전역 설정파일 생성
                    mock.expect_write_file()
                        .times(1)
                        .with(
                            eq(DEFAULT_CONFIG_BASEPATH.to_owned() + "/" + DEFAULT_CONFIG_FILENAME),
                            eq(CONFIG),
                        )
                        .returning(|_, _| Ok(()));

                    // 3. 데이터 디렉터리 생성
                    mock.expect_create_dir()
                        .times(1)
                        .with(eq(DEFAULT_CONFIG_BASEPATH.to_owned() + "/data"))
                        .returning(|_| Ok(()));

                    // 4. WAL 디렉터리 생성
                    mock.expect_create_dir()
                        .times(1)
                        .with(eq(DEFAULT_CONFIG_BASEPATH.to_owned()
                            + "/"
                            + DEFAULT_WAL_DIRNAME))
                        .returning(|_| Ok(()));

                    // 5. 데몬 설정파일 생성
                    mock.expect_write_file()
                        .times(1)
                        .with(
                            eq("/etc/systemd/system/rrdb.service"),
                            eq(SYSTEMD_DAEMON_SCRIPT.as_bytes()),
                        )
                        .returning(|_, _| Err(Error::from_raw_os_error(1)));

                    Arc::new(mock)
                }),
                mock_command_runner: Box::new(|| {
                    let mut mock = MockCommandRunner::new();

                    mock.expect_run()
                        .returning(|_| Err(Error::from_raw_os_error(1)));

                    Arc::new(mock)
                }),
            },
            TestCase {
                name: "데이터 디렉터리 생성 실패",
                want_error: true,
                mock_config: Box::new(|| {
                    let config = LaunchConfig::default();

                    Arc::new(config)
                }),
                mock_file_system: Box::new(move || {
                    let mut mock = MockFileSystem::new();

                    // 1. 최상위 디렉터리 생성
                    mock.expect_create_dir()
                        .times(1)
                        .with(eq(DEFAULT_CONFIG_BASEPATH))
                        .returning(|_| Ok(()));

                    // 2. 전역 설정파일 생성
                    mock.expect_write_file()
                        .times(1)
                        .with(
                            eq(DEFAULT_CONFIG_BASEPATH.to_owned() + "/" + DEFAULT_CONFIG_FILENAME),
                            eq(CONFIG),
                        )
                        .returning(|_, _| Ok(()));

                    // 3. 데이터 디렉터리 생성
                    mock.expect_create_dir()
                        .times(1)
                        .with(eq(DEFAULT_CONFIG_BASEPATH.to_owned()
                            + "/"
                            + DEFAULT_DATA_DIRNAME))
                        .returning(|_| Err(Error::from_raw_os_error(1)));

                    Arc::new(mock)
                }),
                mock_command_runner: Box::new(|| {
                    let mock = MockCommandRunner::new();

                    Arc::new(mock)
                }),
            },
            TestCase {
                name: "전역 설정파일 생성 실패",
                want_error: true,
                mock_config: Box::new(|| {
                    let config = LaunchConfig::default();

                    Arc::new(config)
                }),
                mock_file_system: Box::new(move || {
                    let mut mock = MockFileSystem::new();

                    // 1. 최상위 디렉터리 생성
                    mock.expect_create_dir()
                        .times(1)
                        .with(eq(DEFAULT_CONFIG_BASEPATH))
                        .returning(|_| Ok(()));

                    // 2. 전역 설정파일 생성
                    mock.expect_write_file()
                        .times(1)
                        .with(
                            eq(DEFAULT_CONFIG_BASEPATH.to_owned() + "/" + DEFAULT_CONFIG_FILENAME),
                            eq(CONFIG),
                        )
                        .returning(|_, _| Err(Error::from_raw_os_error(1)));

                    Arc::new(mock)
                }),
                mock_command_runner: Box::new(|| {
                    let mock = MockCommandRunner::new();

                    Arc::new(mock)
                }),
            },
            TestCase {
                name: "최상위 디렉터리 생성 실패",
                want_error: true,
                mock_config: Box::new(|| {
                    let config = LaunchConfig::default();

                    Arc::new(config)
                }),
                mock_file_system: Box::new(move || {
                    let mut mock = MockFileSystem::new();

                    // 1. 최상위 디렉터리 생성
                    mock.expect_create_dir()
                        .times(1)
                        .with(eq(DEFAULT_CONFIG_BASEPATH))
                        .returning(|_| Err(Error::from_raw_os_error(1)));

                    Arc::new(mock)
                }),
                mock_command_runner: Box::new(|| {
                    let mock = MockCommandRunner::new();

                    Arc::new(mock)
                }),
            },
        ];

        for index in [0, 2, 4, 5, 6] {
            let test_case = &test_cases[index];
            let executor = DBEngine {
                config: (test_case.mock_config)(),
                file_system: (test_case.mock_file_system)(),
                command_runner: (test_case.mock_command_runner)(),
            };

            let result = executor.init_config(None).await;

            assert_eq!(
                test_case.want_error,
                result.is_err(),
                "{} - wanr_eror = {}, error = {:?}",
                test_case.name,
                test_case.want_error,
                result.err(),
            );
        }
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn test_init_config_uses_custom_base_path() {
        use mockall::predicate::eq;

        use crate::common::{command::MockCommandRunner, fs::MockFileSystem};

        use super::*;
        use std::sync::Arc;

        let base_path = PathBuf::from("/tmp/rrdb");
        let config = LaunchConfig::default_for_base_path(&base_path);
        let config_bytes = toml::to_string(&config).unwrap().into_bytes();

        let mut file_system = MockFileSystem::new();
        file_system
            .expect_create_dir()
            .times(1)
            .with(eq("/tmp/rrdb"))
            .returning(|_| Ok(()));
        file_system
            .expect_write_file()
            .times(1)
            .with(eq("/tmp/rrdb/rrdb.config"), eq(config_bytes))
            .returning(|_, _| Ok(()));
        file_system
            .expect_create_dir()
            .times(1)
            .with(eq("/tmp/rrdb/data"))
            .returning(|_| Ok(()));
        file_system
            .expect_create_dir()
            .times(1)
            .with(eq("/tmp/rrdb/wal"))
            .returning(|_| Ok(()));

        let command_runner = MockCommandRunner::new();

        let executor = DBEngine {
            config: Arc::new(config),
            file_system: Arc::new(file_system),
            command_runner: Arc::new(command_runner),
        };

        let result = executor.init_config(Some(base_path)).await;

        assert!(result.is_ok(), "error = {:?}", result.err());
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn test_install_daemon_registers_and_starts_daemon() {
        use mockall::predicate::eq;

        use crate::{
            common::{command::MockCommandRunner, fs::MockFileSystem},
            constants::SYSTEMD_DAEMON_SCRIPT,
        };

        use super::*;
        use std::sync::Arc;

        let mut file_system = MockFileSystem::new();
        file_system
            .expect_write_file()
            .times(1)
            .with(
                eq("/etc/systemd/system/rrdb.service"),
                eq(SYSTEMD_DAEMON_SCRIPT.as_bytes()),
            )
            .returning(|_, _| Ok(()));

        let mut command_runner = MockCommandRunner::new();
        command_runner.expect_run().times(1).returning(|_| {
            Ok(Output {
                stderr: Vec::new(),
                stdout: Vec::new(),
                status: Default::default(),
            })
        });

        let executor = DBEngine {
            config: Arc::new(LaunchConfig::default()),
            file_system: Arc::new(file_system),
            command_runner: Arc::new(command_runner),
        };

        let result = executor.install_daemon().await;

        assert!(result.is_ok(), "error = {:?}", result.err());
    }
}
