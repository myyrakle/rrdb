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
    pub async fn init_config(&self) -> Result<(), RRDBError> {
        // 1. 루트 디렉터리 생성 (없다면)
        self.create_top_level_directory_if_not_exists().await?;

        // 2. 전역 설정파일 생성 (없다면)
        self.create_global_config_if_not_exists().await?;

        // 3. 데이터 디렉터리 생성 (없다면)
        self.create_data_directory_if_not_exists().await?;

        // 4. WAL 디렉터리 생성 (없다면)
        self.create_wal_directory_if_not_exists().await?;

        // 5. 데몬 설정파일 생성 (없다면)
        self.create_daemon_config_if_not_exists().await?;

        // 6. 데몬 실행
        self.start_daemon().await?;

        Ok(())
    }

    pub async fn init_database(&self) -> Result<(), RRDBError> {
        // 6. 기본 데이터베이스 생성 (rrdb)
        self.create_database(
            CreateDatabaseQuery::builder()
                .set_name(DEFAULT_DATABASE_NAME.into())
                .set_if_not_exists(true),
        )
        .await?;

        Ok(())
    }

    async fn create_top_level_directory_if_not_exists(&self) -> Result<(), RRDBError> {
        let base_path = DEFAULT_CONFIG_BASEPATH;

        if let Err(error) = self.file_system.create_dir(base_path).await
            && error.kind() != std::io::ErrorKind::AlreadyExists {
                println!("path {:?}", base_path);
                println!("error: {:?}", error.to_string());
                return Err(ExecuteError::wrap(error.to_string()));
            }

        Ok(())
    }

    async fn create_global_config_if_not_exists(&self) -> Result<(), RRDBError> {
        let base_path = PathBuf::from(DEFAULT_CONFIG_BASEPATH);

        let mut global_path = base_path;
        global_path.push(DEFAULT_CONFIG_FILENAME);

        let global_info = GlobalConfig::default();
        let global_config = toml::to_string(&global_info).unwrap();

        if let Err(error) = self
            .file_system
            .write_file(
                global_path.to_str().unwrap_or_default(),
                global_config.as_bytes(),
            )
            .await
        {
            return Err(ExecuteError::wrap(error.to_string()));
        }

        Ok(())
    }

    async fn create_data_directory_if_not_exists(&self) -> Result<(), RRDBError> {
        let data_path = self.config.data_directory.clone();

        if let Err(error) = self.file_system.create_dir(&data_path).await
            && error.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(ExecuteError::wrap(error.to_string()));
            }

        Ok(())
    }

    async fn create_wal_directory_if_not_exists(&self) -> Result<(), RRDBError> {
        let wal_path = self.config.wal_directory.clone();

        if let Err(error) = self.file_system.create_dir(&wal_path).await
            && error.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(ExecuteError::wrap(error.to_string()));
            }

        Ok(())
    }

    #[cfg(target_os = "linux")]
    async fn create_daemon_config_if_not_exists(&self) -> Result<(), RRDBError> {
        use crate::constants::SYSTEMD_DAEMON_SCRIPT;

        let base_path = PathBuf::from("/etc/systemd/system/rrdb.service");

        self.write_and_check_err(base_path, SYSTEMD_DAEMON_SCRIPT)
            .await
    }

    #[cfg(target_os = "macos")]
    async fn create_daemon_config_if_not_exists(&self) -> Result<(), RRDBError> {
        use crate::constants::LAUNCHD_DAEMON_SCRIPT;

        let base_path = PathBuf::from(LAUNCHD_PLIST_PATH);

        self.write_and_check_err(base_path, LAUNCHD_DAEMON_SCRIPT)
            .await
    }

    #[cfg(target_os = "windows")]
    async fn create_daemon_config_if_not_exists(&self) -> Result<(), RRDBError> {
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
    async fn write_and_check_err(
        &self,
        base_path: PathBuf,
        contents: &str,
    ) -> Result<(), RRDBError> {
        if let Err(error) = self
            .file_system
            .write_file(base_path.to_str().unwrap_or_default(), contents.as_bytes())
            .await
            && error.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(ExecuteError::wrap(error.to_string()));
            }
        Ok(())
    }

    async fn start_daemon(&self) -> Result<(), RRDBError> {
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

    fn check_output_status(&self, output: Result<Output, Error>) -> Result<(), RRDBError> {
        if output.is_err() {
            Err(ExecuteError::wrap("failed to start daemon"))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn test_init_config() {
        use mockall::predicate::eq;

        use crate::{
            constants::{DEFAULT_DATA_DIRNAME, DEFAULT_WAL_DIRNAME},
            executor::mocking::{CommandRunner, FileSystem, MockCommandRunner, MockFileSystem},
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

        use crate::constants::SYSTEMD_DAEMON_SCRIPT;

        struct TestCase {
            name: &'static str,
            want_error: bool,
            mock_file_system: Box<dyn Fn() -> Arc<dyn FileSystem + Send + Sync>>,
            mock_command_runner: Box<dyn Fn() -> Arc<dyn CommandRunner + Send + Sync>>,
            mock_config: Box<dyn Fn() -> Arc<GlobalConfig>>,
        }

        let test_cases = vec![
            TestCase {
                name: "init 정상 동작 (linux)",
                want_error: false,
                mock_config: Box::new(|| {
                    let config = GlobalConfig::default();

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

                    // 5. 데몬 설정파일 생성
                    mock.expect_write_file()
                        .times(1)
                        .with(
                            eq("/etc/systemd/system/rrdb.service"),
                            eq(SYSTEMD_DAEMON_SCRIPT.as_bytes()),
                        )
                        .returning(|_, _| Ok(()));

                    Arc::new(mock)
                }),
                mock_command_runner: Box::new(|| {
                    let mut mock = MockCommandRunner::new();

                    mock.expect_run().returning(|_| {
                        Ok(Output {
                            stderr: Vec::new(),
                            stdout: Vec::new(),
                            status: Default::default(),
                        })
                    });

                    Arc::new(mock)
                }),
            },
            TestCase {
                name: "최종 Command 실행 실패",
                want_error: true,
                mock_config: Box::new(|| {
                    let config = GlobalConfig::default();

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
                    let config = GlobalConfig::default();

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
                    let config = GlobalConfig::default();

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
                    let config = GlobalConfig::default();

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
                    let config = GlobalConfig::default();

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
                    let config = GlobalConfig::default();

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

        for test_case in test_cases[..5].iter() {
            let executor = Executor {
                config: (test_case.mock_config)(),
                file_system: (test_case.mock_file_system)(),
                command_runner: (test_case.mock_command_runner)(),
            };

            let result = executor.init_config().await;

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
}
