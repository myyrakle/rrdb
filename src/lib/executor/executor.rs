use path_absolutize::*;
use std::error::Error;
use std::path::PathBuf;

use crate::lib::ast::predule::{
    CreateDatabaseQuery, DDLStatement, DMLStatement, OtherStatement, SQLStatement,
};
use crate::lib::errors::execute_error::ExecuteError;
use crate::lib::executor::predule::{ExecuteResult, GlobalConfig};
use crate::lib::logger::predule::Logger;
use crate::lib::utils::predule::set_system_env;

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
    pub async fn init(&self, path: String) -> Result<(), Box<dyn Error + Send>> {
        let mut path_buf = PathBuf::new();
        path_buf.push(path);
        path_buf.push(".rrdb.config");

        #[allow(non_snake_case)]
        let RRDB_BASE_PATH = path_buf
            .absolutize()
            .map_err(|e| ExecuteError::dyn_boxed(e.to_string()))?
            .to_str()
            .unwrap()
            .to_string();
        set_system_env("RRDB_BASE_PATH", &RRDB_BASE_PATH);

        // 루트 디렉터리 생성
        let base_path = path_buf.clone();
        if let Err(error) = tokio::fs::create_dir(base_path.clone()).await {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                // Do Nothing
            } else {
                return Err(ExecuteError::boxed(error.to_string()));
            }
        }

        // 전역 설정파일 생성
        let mut global_path = base_path.clone();
        global_path.push("global.config");
        let global_info = GlobalConfig::default();
        let global_config = toml::to_string(&global_info).unwrap();

        if let Err(error) = tokio::fs::write(global_path, global_config.as_bytes()).await {
            return Err(ExecuteError::boxed(error.to_string()));
        }

        self.create_database(CreateDatabaseQuery::builder().set_name("rrdb".into()))
            .await?;

        Ok(())
    }

    // 쿼리 최적화 및 실행, 결과 반환
    pub async fn process_query(
        &self,
        statement: SQLStatement,
    ) -> Result<ExecuteResult, Box<dyn Error + Send>> {
        Logger::info(format!("AST echo: {:?}", statement));

        // 쿼리 실행
        let result = match statement {
            SQLStatement::DDL(DDLStatement::CreateDatabaseQuery(query)) => {
                self.create_database(query).await
            }
            SQLStatement::DDL(DDLStatement::AlterDatabase(query)) => {
                self.alter_database(query).await
            }
            SQLStatement::DDL(DDLStatement::DropDatabaseQuery(query)) => {
                self.drop_database(query).await
            }
            SQLStatement::DDL(DDLStatement::CreateTableQuery(query)) => {
                self.create_table(query).await
            }
            SQLStatement::DDL(DDLStatement::AlterTableQuery(query)) => {
                self.alter_table(query).await
            }
            SQLStatement::DDL(DDLStatement::DropTableQuery(query)) => self.drop_table(query).await,
            SQLStatement::DML(DMLStatement::InsertQuery(query)) => self.insert(query).await,
            SQLStatement::DML(DMLStatement::SelectQuery(query)) => self.select(query).await,
            SQLStatement::DML(DMLStatement::UpdateQuery(query)) => self.update(query).await,
            SQLStatement::DML(DMLStatement::DeleteQuery(query)) => self.delete(query).await,
            SQLStatement::Other(OtherStatement::ShowDatabases(query)) => {
                self.show_databases(query).await
            }
            SQLStatement::Other(OtherStatement::UseDatabase(query)) => {
                self.use_databases(query).await
            }
            SQLStatement::Other(OtherStatement::ShowTables(query)) => self.show_tables(query).await,
            SQLStatement::Other(OtherStatement::DescTable(query)) => self.desc_table(query).await,
            _ => unimplemented!("no execute implementation"),
        };

        match result {
            Ok(result) => Ok(result),
            Err(error) => Err(ExecuteError::boxed(error.to_string())),
        }
    }
}
