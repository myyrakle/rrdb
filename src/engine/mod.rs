pub mod ast;
pub mod encoder;
pub mod index;
pub mod lexer;
pub mod optimizer;
pub mod parser;
pub mod path_identifier;
pub mod query_memory;
pub mod row_buffer;
#[cfg(test)]
mod row_offset_model_tests;
mod row_offsets;
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
use std::ptr::NonNull;
use std::sync::Arc;

use crate::common::command::{CommandRunner, RealCommandRunner};
use crate::common::fs::{FileSystem, RealFileSystem};
use crate::config::launch_config::LaunchConfig;
use crate::engine::ast::ddl::create_index::CreateIndexQuery;
use crate::engine::ast::ddl::drop_index::DropIndexQuery;
use crate::engine::ast::dml::delete::DeleteQuery;
use crate::engine::ast::dml::insert::InsertQuery;
use crate::engine::ast::dml::update::UpdateQuery;
use crate::engine::ast::types::TableName;
use crate::engine::ast::{DDLStatement, DMLStatement, OtherStatement, SQLStatement};
use crate::engine::encoder::schema_encoder::StorageEncoder;
use crate::engine::index::manager::IndexManager;
use crate::engine::optimizer::statistics::StatisticsManager;
use crate::engine::query_memory::{QueryMemoryTracker, QueryMemoryTrackerRef};
use crate::engine::row_buffer::RowBufferPool;
use crate::engine::schema::table::TableSchema;
use crate::engine::types::ExecuteResult;
use crate::engine::wal::endec::implements::bincode::BincodeEncoder;
use crate::engine::wal::manager::WALManager;
use crate::engine::wal::types::{EntryType, InsertWALPayload, WALEntry};
use crate::errors;
use crate::errors::execute_error::ExecuteError;
use tokio::sync::{Mutex, RwLock};

/// 쿼리 수준 메모리 예산 추적기 (#265).
///
/// 서버는 연결마다 task를 spawn하므로, 동시에 여러
/// `process_query`가 실행될 수 있습니다. 공유된 슬롯(RwLock<Option<..>>)에
/// 저장하면 서로의 tracker를 덮어쓰거나 클리어하는 race가
/// 발생합니다. 특히 한 쿼리가 스캔을 마치기 전에 다른
/// 쿼리가 슬롯을 클리어하면 후자는 예산 없이 실행됩니다.
///
/// 해결: `tokio::task_local!`로 task 범위에 저장합니다.
/// (`tokio::task_local!`은 `Copy` 타입만 지원하므로
/// `QueryMemoryTrackerRef`(NonNull 래퍼)를 저장합니다.)
/// - task 마다 각자의 tracker를 가짐 → 동시 쿼리 간 race 없음
/// - task 종료 시 자동 정리 → cleanup race 없음
/// - 시그니처 변경 없이 내부 코드에서 `query_memory()` 호출 가능
tokio::task_local! {
    static QUERY_MEMORY_TRACKER: QueryMemoryTrackerRef;
}

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
    pub(crate) row_buffer_pool: Arc<Mutex<RowBufferPool>>,
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
            row_buffer_pool: Arc::new(Mutex::new(RowBufferPool::default())),
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

        // 쿼리 수준 메모리 예산 설정 (#265). DML(SELECT/UPDATE/DELETE)만 적용합니다.
        // INSERT는 행을 추가할 뿐 대량 메모리를 모으지 않고, DDL은 스키마 변경이라
        // 예산 대상이 아닙니다. CREATE INDEX의 backfill은 full_scan을 사용하므로
        // 예산 적용 대상이 맞지만, DDL 경로에 넣으면 인덱스 생성이 실패할 수 있어
        // 일단 DML만 적용합니다 (추후 필요 시 확장).
        //
        // 공유 슬롯이 아닌 task-local로 전달하므로, 동시 실행되는 여러 쿼리가
        // 서로의 tracker를 덮어쓰거나 클리어하는 race가 없습니다 (CodeRabbit #1).
        let tracker =
            Self::query_memory_tracker_for(&statement, self.config.max_query_memory_bytes);
        // Copy 핸들: scope 동안 `tracker`(Arc)를 클로저에 move하여 값이 살아있게 보장.
        let tracker_ref =
            QueryMemoryTrackerRef(tracker.as_ref().map(|arc| NonNull::from(arc.as_ref())));

        // 쿼리 실행 (task 범위로 tracker 설정; scope 종료 시 자동 정리)
        QUERY_MEMORY_TRACKER
            .scope(tracker_ref, async move {
                // tracker(Arc)를 scope 동안 유지 — tracker_ref가 가리키는 값의 수명 보장
                let _keep_alive = tracker;
                let result = self.execute_statement(statement, wal_manager).await;
                match result {
                    Ok(result) => Ok(result),
                    Err(error) => Err(ExecuteError::wrap(error.to_string())),
                }
            })
            .await
    }

    /// 스테이트먼트를 실행합니다. `process_query`가
    /// 메모리 예산 task-local을 설정한 후 호출합니다.
    async fn execute_statement(
        &self,
        statement: SQLStatement,
        wal_manager: SharedWALManager,
    ) -> errors::Result<ExecuteResult> {
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
                self.create_index(query, wal_manager.clone()).await
            }
            SQLStatement::DDL(DDLStatement::DropIndexQuery(query)) => {
                self.drop_index(query, wal_manager.clone()).await
            }
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

        result
    }

    /// 쿼리 메모리 예산 추적기를 생성합니다 (#265).
    /// DML(SELECT/UPDATE/DELETE)에만 예산을 설정하고, 나머지는 `None`을
    /// 반환하여 예산 없이 실행합니다.
    fn query_memory_tracker_for(
        statement: &SQLStatement,
        limit_bytes: u64,
    ) -> Option<Arc<QueryMemoryTracker>> {
        let is_dml_read_heavy = matches!(
            statement,
            SQLStatement::DML(DMLStatement::SelectQuery(_))
                | SQLStatement::DML(DMLStatement::UpdateQuery(_))
                | SQLStatement::DML(DMLStatement::DeleteQuery(_))
        );

        if !is_dml_read_heavy || limit_bytes == 0 {
            return None;
        }

        Some(Arc::new(QueryMemoryTracker::new(limit_bytes)))
    }

    /// 현재 실행 중인 쿼리의 메모리 추적기를 반환합니다.
    /// 예산 비활성 상태이면 `None`을 반환합니다.
    pub(crate) async fn query_memory(&self) -> Option<Arc<QueryMemoryTracker>> {
        // `try_with`를 사용해 task-local이 설정되지 않은 컨텍스트(테스트에서
        // `process_query`를 거치지 않고 full_scan 등을 직접 호출)에서도
        // panic 대신 `None`을 반환합니다.
        QUERY_MEMORY_TRACKER
            .try_with(|tracker_ref| match tracker_ref.0 {
                Some(ptr) => {
                    // 안전성: scope 동안 원본 Arc(`_keep_alive`)가 살아 있으므로
                    // 포인터는 항상 유효. refcount를 +1 하고 from_raw로
                    // 재구성하여 반환되는 Arc의 수명을 보장합니다.
                    //
                    // # Safety
                    // - `ptr`은 `NonNull::from(arc.as_ref())`로 만들어졌으므로 항상
                    //   유효한 `QueryMemoryTracker`를 가리킵니다.
                    // - scope 동안 `_keep_alive`가 refcount를 보유하므로
                    //   `increment_strong_count` 시점에 사용 후방된 메모리가 아닙니다.
                    unsafe {
                        std::sync::Arc::increment_strong_count(ptr.as_ptr());
                        Some(std::sync::Arc::from_raw(ptr.as_ptr()))
                    }
                }
                None => None,
            })
            .unwrap_or(None)
    }

    /// Replays WAL entries written but not yet checkpointed before a crash,
    /// re-executing each recorded operation so its data/index mutation is
    /// guaranteed to have taken effect (WAL-ahead recovery, mirroring
    /// PostgreSQL's redo phase). A single entry failing to reapply (e.g. a
    /// unique-constraint violation, which -- since WAL is written before the
    /// mutation -- would also have failed the first time around and so never
    /// actually took effect) aborts recovery so that the WAL is preserved for
    /// diagnosis and a later retry.
    pub async fn replay_wal(&self, entries: &[WALEntry]) -> errors::Result<()> {
        for (index, entry) in entries.iter().enumerate() {
            let data = entry.data.as_deref();

            let result: errors::Result<()> = async {
                match entry.entry_type {
                    EntryType::Insert => {
                        // 새 형식은 start_row_index를 포함합니다. 이전 형식(InsertQuery
                        // 단독)으로 기록된 WAL도 그대로 읽을 수 있어야 하므로,
                        // 실패 시 이전 형식으로 폴백합니다 (#236).
                        match Self::decode_wal_payload::<InsertWALPayload>(data) {
                            Ok(payload) => {
                                self.insert_replay_with_payload(payload).await.map(|_| ())
                            }
                            Err(_) => {
                                let query = Self::decode_wal_payload::<InsertQuery>(data)?;
                                self.insert_replay(query).await.map(|_| ())
                            }
                        }
                    }
                    EntryType::Set => {
                        let query = Self::decode_wal_payload::<UpdateQuery>(data)?;
                        self.update_replay(query).await.map(|_| ())
                    }
                    EntryType::Delete => {
                        let query = Self::decode_wal_payload::<DeleteQuery>(data)?;
                        self.delete_replay(query).await.map(|_| ())
                    }
                    EntryType::CreateIndex => {
                        let query = Self::decode_wal_payload::<CreateIndexQuery>(data)?;
                        self.create_index_replay(query).await.map(|_| ())
                    }
                    EntryType::DropIndex => {
                        let query = Self::decode_wal_payload::<DropIndexQuery>(data)?;
                        self.drop_index_replay(query).await.map(|_| ())
                    }
                    EntryType::Checkpoint
                    | EntryType::TransactionBegin
                    | EntryType::TransactionCommit
                    | EntryType::TransactionRollback => Ok(()),
                }
            }
            .await;

            result.map_err(|error| {
                ExecuteError::wrap(format!(
                    "WAL replay failed at entry {} ({:?}): {}",
                    index, entry.entry_type, error
                ))
            })?;
        }

        Ok(())
    }

    fn decode_wal_payload<T>(data: Option<&[u8]>) -> errors::Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let data =
            data.ok_or_else(|| ExecuteError::wrap("WAL entry missing payload".to_string()))?;
        bincode::deserialize(data).map_err(|error| ExecuteError::wrap(error.to_string()))
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
            Ok(data) => match encoder.decode::<TableSchema>(data.as_slice()) {
                Ok(table_config) => Ok(table_config),
                Err(error) => Err(ExecuteError::wrap(format!(
                    "invalid config data: {}",
                    error
                ))),
            },
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
    use crate::engine::ast::dml::insert::{InsertData, InsertQuery};
    use crate::engine::ast::dml::parts::insert_values::InsertValue;
    use crate::engine::ast::types::SQLExpression;
    use crate::engine::ast::types::{Column, DataType, TableName};
    use crate::engine::encoder::schema_encoder::StorageEncoder;
    use crate::engine::schema::table::TableSchema;
    use crate::engine::wal::types::{EntryType, InsertWALPayload, WALEntry};

    #[tokio::test]
    async fn replay_wal_returns_contextual_error_for_invalid_payload() {
        let config = LaunchConfig::default_for_base_path(PathBuf::from(
            "target/test_wal_replay/invalid_payload",
        ));
        let engine = DBEngine::new(config);
        let entry = WALEntry {
            entry_type: EntryType::Insert,
            data: Some(vec![0xff]),
            timestamp: 1,
            transaction_id: None,
            is_continuation: false,
        };

        let error = engine.replay_wal(&[entry]).await.unwrap_err();

        assert!(error.to_string().contains("entry 0"));
        assert!(error.to_string().contains("Insert"));
    }

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

    /// 행이 durable해진 뒤 WAL 체크포인트 경계가 진행되기 전에 크래시하면,
    /// 재시작 시 replay가 이미 디스크에 있는 행을 다시 추가할 수 있었습니다.
    /// unique 인덱스가 없는 테이블에서는 값으로 중복을 판별할 수 없으므로
    /// (중복 행이 합법) WAL에 남긴 start_row_index로 판단합니다 (#236).
    #[tokio::test]
    async fn insert_replay_is_idempotent_for_non_unique_table() {
        let base_path = PathBuf::from("target/test_wal_replay/idempotent_insert");
        if base_path.exists() {
            tokio::fs::remove_dir_all(&base_path).await.unwrap();
        }

        let engine = build_engine_with_table(&base_path, "events").await;
        let table_name = TableName::new(Some("rrdb".to_string()), "events".to_string());

        let query = insert_query(&table_name, 1);
        let payload = InsertWALPayload {
            query: query.clone(),
            start_row_index: 0,
            row_count: 1,
        };

        // 첫 replay: 행이 아직 없으므로 반영되어야 합니다.
        engine
            .insert_replay_with_payload(payload.clone())
            .await
            .unwrap();
        engine.flush_row_buffers().await.unwrap();
        assert_eq!(engine.next_row_index(&table_name).await.unwrap(), 1);

        // 두 번째 replay: 같은 WAL 엔트리가 다시 재생되어도 중복 삽입되면 안 됩니다.
        engine.insert_replay_with_payload(payload).await.unwrap();
        engine.flush_row_buffers().await.unwrap();
        assert_eq!(
            engine.next_row_index(&table_name).await.unwrap(),
            1,
            "replaying an already-applied INSERT must not duplicate the row"
        );
    }

    /// 멱등성 처리가 정상적인 중복 INSERT까지 막으면 안 됩니다. 서로 다른
    /// start_row_index를 가진 두 INSERT는 값이 같아도 모두 보존되어야 합니다.
    #[tokio::test]
    async fn legitimate_duplicate_inserts_are_preserved() {
        let base_path = PathBuf::from("target/test_wal_replay/duplicate_inserts");
        if base_path.exists() {
            tokio::fs::remove_dir_all(&base_path).await.unwrap();
        }

        let engine = build_engine_with_table(&base_path, "events").await;
        let table_name = TableName::new(Some("rrdb".to_string()), "events".to_string());

        for start_row_index in 0..3 {
            engine
                .insert_replay_with_payload(InsertWALPayload {
                    query: insert_query(&table_name, 7),
                    start_row_index,
                    row_count: 1,
                })
                .await
                .unwrap();
            engine.flush_row_buffers().await.unwrap();
        }

        assert_eq!(
            engine.next_row_index(&table_name).await.unwrap(),
            3,
            "identical rows inserted by distinct statements must all survive"
        );
    }

    fn insert_query(table_name: &TableName, value: i64) -> InsertQuery {
        InsertQuery {
            into_table: Some(table_name.clone()),
            columns: vec!["id".to_string()],
            data: InsertData::Values(vec![InsertValue {
                list: vec![Some(SQLExpression::Integer(value))],
            }]),
        }
    }

    async fn build_engine_with_table(base_path: &PathBuf, table: &str) -> DBEngine {
        let config = LaunchConfig::default_for_base_path(base_path);
        let table_name = TableName::new(Some("rrdb".to_string()), table.to_string());
        let table_path = PathBuf::from(&config.data_directory)
            .join("rrdb")
            .join("tables")
            .join(table);

        tokio::fs::create_dir_all(table_path.join("rows"))
            .await
            .unwrap();

        let table_config = TableSchema {
            table: table_name,
            columns: vec![
                Column::builder()
                    .set_name("id".to_string())
                    .set_data_type(DataType::Int)
                    .build(),
            ],
            primary_key: vec![],
            foreign_keys: vec![],
            unique_keys: vec![],
        };

        let encoder = StorageEncoder::new();
        tokio::fs::write(
            table_path.join("table.config"),
            encoder.encode(table_config),
        )
        .await
        .unwrap();

        DBEngine::new(config)
    }
}
