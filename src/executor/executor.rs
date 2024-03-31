use std::path::PathBuf;

use crate::ast::ddl::create_database::CreateDatabaseQuery;
use crate::ast::{DDLStatement, DMLStatement, OtherStatement, SQLStatement};
use crate::constants::{
    DEFAULT_CONFIG_BASEPATH, DEFAULT_CONFIG_FILENAME, DEFAULT_DATABASE_NAME, DEFAULT_DATA_DIRNAME,
};
use crate::errors::execute_error::ExecuteError;
use crate::errors::RRDBError;
use crate::executor::predule::ExecuteResult;
use crate::logger::predule::Logger;

use super::config::global::GlobalConfig;

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

    // 쿼리 최적화 및 실행, 결과 반환
    pub async fn process_query(
        &self,
        statement: SQLStatement,
        _connection_id: String,
    ) -> Result<ExecuteResult, RRDBError> {
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
            Err(error) => Err(ExecuteError::new(error.to_string())),
        }
    }
}
