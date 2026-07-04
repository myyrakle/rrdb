pub mod ast;
pub mod encoder;
pub mod index;
pub mod lexer;
pub mod optimizer;
pub mod parser;
pub mod schema;
pub mod server;
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
use crate::engine::index::manager::IndexManager;
use crate::engine::optimizer::statistics::StatisticsManager;
use crate::engine::schema::table::TableSchema;
use crate::engine::types::ExecuteResult;
use crate::engine::wal::endec::implements::bincode::BincodeEncoder;
use crate::engine::wal::manager::WALManager;
use crate::errors;
use crate::errors::execute_error::ExecuteError;
use tokio::sync::{Mutex, RwLock};

pub type SharedWALManager = Arc<Mutex<WALManager<BincodeEncoder>>>;

pub struct DBEngine {
    pub(crate) config: Arc<LaunchConfig>,
    pub(crate) file_system: Arc<dyn FileSystem + Send + Sync>,
    pub(crate) command_runner: Arc<dyn CommandRunner + Send + Sync>,
    pub(crate) table_config_cache: Arc<RwLock<HashMap<TableName, TableSchema>>>,
    pub(crate) row_storage_lock: Arc<Mutex<()>>,
    pub(crate) index_manager: Arc<IndexManager>,
    pub(crate) statistics_manager: Arc<StatisticsManager>,
    /// 디스크의 인덱스 파일을 메모리로 적재했는지 여부 (최초 사용 시 1회 적재)
    pub(crate) indices_loaded: Arc<tokio::sync::OnceCell<()>>,
}

impl DBEngine {
    pub fn new(config: LaunchConfig) -> Self {
        let data_directory = PathBuf::from(config.data_directory.clone());

        Self {
            config: Arc::new(config),
            file_system: Arc::new(RealFileSystem {}),
            command_runner: Arc::new(RealCommandRunner {}),
            table_config_cache: Arc::new(RwLock::new(HashMap::new())),
            row_storage_lock: Arc::new(Mutex::new(())),
            index_manager: Arc::new(IndexManager::new(data_directory)),
            statistics_manager: Arc::new(StatisticsManager::new()),
            indices_loaded: Arc::new(tokio::sync::OnceCell::new()),
        }
    }

    // 쿼리 최적화 및 실행, 결과 반환
    pub async fn process_query(
        &self,
        statement: SQLStatement,
        wal_manager: SharedWALManager,
        _connection_id: String,
    ) -> errors::Result<ExecuteResult> {
        log::debug!("AST echo: {:?}", statement);

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
            SQLStatement::DDL(DDLStatement::CreateIndexQuery(query)) => {
                self.create_index(query).await
            }
            SQLStatement::DDL(DDLStatement::DropIndexQuery(query)) => self.drop_index(query).await,
            SQLStatement::DML(DMLStatement::InsertQuery(query)) => {
                self.insert(query, wal_manager.clone()).await
            }
            SQLStatement::DML(DMLStatement::SelectQuery(query)) => self.select(query).await,
            SQLStatement::DML(DMLStatement::UpdateQuery(query)) => {
                self.update(query, wal_manager.clone()).await
            }
            SQLStatement::DML(DMLStatement::DeleteQuery(query)) => {
                self.delete(query, wal_manager.clone()).await
            }
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
    pub(crate) async fn get_table_config_cached(
        &self,
        table_name: TableName,
    ) -> errors::Result<TableSchema> {
        if let Some(table_config) = self.table_config_cache.read().await.get(&table_name) {
            return Ok(table_config.clone());
        }

        let table_config = self.get_table_config(table_name.clone()).await?;

        self.table_config_cache
            .write()
            .await
            .insert(table_name, table_config.clone());

        Ok(table_config)
    }

    pub(crate) async fn cache_table_config(&self, table_config: TableSchema) {
        self.table_config_cache
            .write()
            .await
            .insert(table_config.table.clone(), table_config);
    }

    pub(crate) async fn invalidate_table_config_cache(&self, table_name: &TableName) {
        self.table_config_cache.write().await.remove(table_name);
    }

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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::config::launch_config::LaunchConfig;
    use crate::engine::DBEngine;
    use crate::engine::ast::types::{Column, DataType, TableName};
    use crate::engine::encoder::schema_encoder::StorageEncoder;
    use crate::engine::schema::table::TableSchema;

    #[tokio::test]
    async fn get_table_config_cached_reuses_loaded_schema() {
        let base_path = PathBuf::from("target/test_table_config_cache/reuses_loaded_schema");
        if base_path.exists() {
            tokio::fs::remove_dir_all(&base_path).await.unwrap();
        }

        let config = LaunchConfig::default_for_base_path(&base_path);
        let table_name = TableName::new(Some("rrdb".to_string()), "users".to_string());
        let table_path = PathBuf::from(&config.data_directory)
            .join("rrdb")
            .join("tables")
            .join("users");
        let config_path = table_path.join("table.config");

        tokio::fs::create_dir_all(&table_path).await.unwrap();

        let table_config = TableSchema {
            table: table_name.clone(),
            columns: vec![
                Column::builder()
                    .set_name("id".to_string())
                    .set_data_type(DataType::Int)
                    .set_primary_key(true)
                    .build(),
            ],
            primary_key: vec!["id".to_string()],
            foreign_keys: vec![],
            unique_keys: vec![],
        };

        let encoder = StorageEncoder::new();
        tokio::fs::write(&config_path, encoder.encode(table_config))
            .await
            .unwrap();

        let engine = DBEngine::new(config);
        let first = engine
            .get_table_config_cached(table_name.clone())
            .await
            .unwrap();
        tokio::fs::remove_file(&config_path).await.unwrap();
        let second = engine.get_table_config_cached(table_name).await.unwrap();

        assert_eq!(first.columns.len(), 1);
        assert_eq!(second.columns[0].name, "id");
    }
}
