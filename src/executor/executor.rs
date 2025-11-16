use std::sync::Arc;

use crate::ast::{DDLStatement, DMLStatement, OtherStatement, SQLStatement};
use crate::errors::RRDBError;
use crate::errors::execute_error::ExecuteError;
use crate::executor::predule::ExecuteResult;
use crate::logger::predule::Logger;
use crate::wal::endec::implements::bitcode::BitcodeEncoder;
use crate::wal::manager::WALManager;

use super::config::global::GlobalConfig;
use super::mocking::{CommandRunner, FileSystem, RealCommandRunner, RealFileSystem};

pub struct Executor {
    pub(crate) config: Arc<GlobalConfig>,
    pub(crate) file_system: Arc<dyn FileSystem + Send + Sync>,
    pub(crate) command_runner: Arc<dyn CommandRunner + Send + Sync>,
}

impl Executor {
    pub fn new(config: Arc<GlobalConfig>) -> Self {
        Self {
            config,
            file_system: Arc::new(RealFileSystem {}),
            command_runner: Arc::new(RealCommandRunner {}),
        }
    }

    // 쿼리 최적화 및 실행, 결과 반환
    pub async fn process_query(
        &self,
        statement: SQLStatement,
        _wal_manager: Arc<WALManager<BitcodeEncoder>>,
        _connection_id: String,
    ) -> Result<ExecuteResult, RRDBError> {
        Logger::info(format!("AST echo: {:?}", statement));

        // TODO: WAL 로깅 추가

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
