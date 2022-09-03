use crate::lib::ast::ddl::{DDLStatement, SQLStatement};
use crate::lib::errors::execute_error::ExecuteError;
use crate::lib::executor::predule::{ExecuteResult, GlobalConfig};
use crate::lib::optimizer::predule::Optimizer;
use crate::lib::utils::predule::set_system_env;
use path_absolutize::*;
use std::error::Error;
use std::path::PathBuf;

pub struct Executor {}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

impl Executor {
    pub fn new() -> Self {
        Self {}
    }

    // 기본 설정파일 세팅
    pub async fn init(&self, path: String) -> Result<(), Box<dyn Error>> {
        let mut path_buf = PathBuf::new();
        path_buf.push(path);
        path_buf.push(".rrdb.config");

        #[allow(non_snake_case)]
        let RRDB_BASE_PATH = path_buf.absolutize()?.to_str().unwrap().to_string();
        set_system_env("RRDB_BASE_PATH", &RRDB_BASE_PATH);

        // 루트 디렉터리 생성
        let base_path = path_buf.clone();
        (match tokio::fs::create_dir(base_path.clone()).await {
            Ok(_) => Ok(()),
            Err(error) => {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    Ok(())
                } else {
                    Err(error)
                }
            }
        })?;

        // 전역 설정파일 생성
        let mut global_path = base_path.clone();
        global_path.push("global.config");
        let global_info = GlobalConfig::default();
        let global_config = toml::to_string(&global_info).unwrap();
        tokio::fs::write(global_path, global_config.as_bytes()).await?;

        Ok(())
    }

    pub async fn process_query(
        &self,
        mut statement: SQLStatement,
    ) -> Result<ExecuteResult, Box<dyn Error>> {
        // 최적화 작업
        let optimizer = Optimizer::new();
        optimizer.optimize(&mut statement);

        // 쿼리 실행
        match statement {
            SQLStatement::DDL(DDLStatement::CreateDatabaseQuery(query)) => {
                self.create_database(query).await
            }
            _ => Err(ExecuteError::boxed("test")),
        }
    }
}
