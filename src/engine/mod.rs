pub mod ast;
pub mod encoder;
pub mod lexer;
pub mod optimizer;
pub mod parser;
pub mod schema;
pub mod server;
pub mod storage;
pub mod wal;

// DB Engine implementations
pub mod actions;
pub mod expression;
pub mod initialize;
pub mod types;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::common::command::{CommandRunner, RealCommandRunner};
use crate::common::fs::{FileSystem, RealFileSystem};
use crate::config::launch_config::LaunchConfig;
use crate::engine::ast::types::TableName;
use crate::engine::ast::{DDLStatement, DMLStatement, OtherStatement, SQLStatement};
use crate::engine::encoder::schema_encoder::StorageEncoder;
use crate::engine::schema::table::TableSchema;
use crate::engine::storage::TableHeap;
use crate::engine::types::ExecuteResult;
use crate::engine::wal::endec::implements::bitcode::BitcodeEncoder;
use crate::engine::wal::manager::WALManager;
use crate::errors;
use crate::errors::execute_error::ExecuteError;
use tokio::sync::RwLock;

pub struct DBEngine {
    pub(crate) config: Arc<LaunchConfig>,
    pub(crate) file_system: Arc<dyn FileSystem + Send + Sync>,
    pub(crate) command_runner: Arc<dyn CommandRunner + Send + Sync>,
    pub(crate) table_heaps: Arc<RwLock<HashMap<TableName, TableHeap>>>,
}

impl DBEngine {
    pub fn new(config: LaunchConfig) -> Self {
        Self {
            config: Arc::new(config),
            file_system: Arc::new(RealFileSystem {}),
            command_runner: Arc::new(RealCommandRunner {}),
            table_heaps: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // 쿼리 최적화 및 실행, 결과 반환
    pub async fn process_query(
        &self,
        statement: SQLStatement,
        _wal_manager: Arc<WALManager<BitcodeEncoder>>,
        _connection_id: String,
    ) -> errors::Result<ExecuteResult> {
        log::info!("AST echo: {:?}", statement);

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
            Err(error) => Err(ExecuteError::wrap(error.to_string())),
        }
    }
}

impl DBEngine {
    pub async fn get_table_config(&self, table_name: TableName) -> errors::Result<TableSchema> {
        let encoder = StorageEncoder::new();

        let base_path = self.get_data_directory();

        let TableName {
            database_name,
            table_name,
        } = table_name;

        let database_name = database_name.unwrap();

        let database_path = base_path.clone().join(&database_name);
        let table_path = database_path.clone().join("tables").join(&table_name);

        // config data 파일 내용 변경
        let config_path = table_path.clone().join("table.config");

        match tokio::fs::read(&config_path).await {
            Ok(data) => {
                let table_config: Option<TableSchema> = encoder.decode(data.as_slice());

                match table_config {
                    Some(table_config) => Ok(table_config),
                    None => Err(ExecuteError::wrap("invalid config data".to_string())),
                }
            }
            Err(error) => match error.kind() {
                std::io::ErrorKind::NotFound => {
                    Err(ExecuteError::wrap("table not found".to_string()))
                }
                _ => Err(ExecuteError::wrap(format!("{:?}", error))),
            },
        }
    }

    // 데이터 저장 경로를 반환합니다..
    pub fn get_data_directory(&self) -> PathBuf {
        PathBuf::from(self.config.data_directory.clone())
    }
}
