use std::collections::HashSet;

use crate::engine::DBEngine;
use crate::engine::SharedWALManager;
use crate::engine::actions::index::{qualified_index_name, row_index_key};
use crate::engine::ast::ddl::create_index::CreateIndexQuery;
use crate::engine::index::{IndexEntry, IndexMeta};
use crate::engine::types::{
    ExecuteColumn, ExecuteColumnType, ExecuteField, ExecuteResult, ExecuteRow,
};
use crate::engine::wal::types::EntryType;
use crate::errors;
use crate::errors::execute_error::ExecuteError;

impl DBEngine {
    pub async fn create_index(
        &self,
        query: CreateIndexQuery,
        wal_manager: SharedWALManager,
    ) -> errors::Result<ExecuteResult> {
        self.ensure_indices_loaded().await?;

        let database_name = query
            .table
            .database_name
            .clone()
            .ok_or_else(|| ExecuteError::wrap("database name is required".to_string()))?;

        // 테이블 존재 검증
        let table_config = self.get_table_config_cached(query.table.clone()).await?;

        // TODO(#217): 다중 컬럼 인덱스 지원
        if query.columns.len() != 1 {
            return Err(ExecuteError::wrap(
                "multi-column indexes are not supported yet".to_string(),
            ));
        }

        let column_name = &query.columns[0];

        // 컬럼 존재 검증
        if !table_config
            .columns
            .iter()
            .any(|column| &column.name == column_name)
        {
            return Err(ExecuteError::wrap(format!(
                "column '{}' not exists",
                column_name
            )));
        }

        let index_name = qualified_index_name(&database_name, &query.index_name);

        if self.index_manager.get_meta(&index_name).await.is_some() {
            if query.if_not_exists {
                return Ok(Self::index_result(format!(
                    "index already exists, skipped: {}",
                    query.index_name
                )));
            }

            return Err(ExecuteError::wrap(format!(
                "index '{}' already exists",
                query.index_name
            )));
        }

        // WAL-first: 인덱스 매니저를 변경하기 전에 먼저 durable하게 기록합니다.
        let wal_payload =
            bincode::serialize(&query).map_err(|error| ExecuteError::wrap(error.to_string()))?;
        wal_manager
            .lock()
            .await
            .append_record(EntryType::CreateIndex, Some(wal_payload), None)
            .await?;

        self.create_index_apply(&query).await
    }

    /// Re-applies a previously WAL-logged CREATE INDEX during crash recovery
    /// replay. Idempotent: if the index metadata already exists (e.g. the
    /// crash happened after the metadata was created but before the backfill
    /// finished), it re-runs the backfill instead of failing.
    pub(crate) async fn create_index_replay(
        &self,
        query: CreateIndexQuery,
    ) -> errors::Result<ExecuteResult> {
        self.ensure_indices_loaded().await?;
        self.create_index_apply(&query).await
    }

    async fn create_index_apply(&self, query: &CreateIndexQuery) -> errors::Result<ExecuteResult> {
        let table = query.table.clone();
        let database_name = table
            .database_name
            .clone()
            .ok_or_else(|| ExecuteError::wrap("database name is required".to_string()))?;
        let column_name = query.columns[0].clone();
        let index_name = qualified_index_name(&database_name, &query.index_name);

        if self.index_manager.get_meta(&index_name).await.is_none() {
            let meta = IndexMeta::new(
                index_name.clone(),
                table.clone(),
                column_name.clone(),
                query.is_unique,
            );

            self.index_manager.create_index(meta).await?;
        }

        // 기존 행 색인 (backfill). replace_entries는 전체 교체이므로 재실행해도 안전합니다.
        let rows = self.full_scan(table.clone()).await?;

        let mut entries = Vec::new();
        let mut seen_keys = HashSet::new();

        for (location, row) in &rows {
            if let Some(key) = row_index_key(row, &column_name) {
                if query.is_unique && !seen_keys.insert(key.clone()) {
                    // 고유 제약 위반: 생성한 인덱스를 롤백
                    let _ = self.index_manager.drop_index(&index_name).await;
                    return Err(ExecuteError::wrap(format!(
                        "cannot create unique index '{}': column '{}' contains duplicate values",
                        query.index_name, column_name
                    )));
                }

                entries.push(IndexEntry {
                    key,
                    row_path: location.row_index.to_string(),
                });
            }
        }

        if let Err(error) = self
            .index_manager
            .replace_entries(&index_name, entries)
            .await
        {
            let _ = self.index_manager.drop_index(&index_name).await;
            return Err(error);
        }

        // 새 인덱스의 distinct 통계가 반영되도록 캐시 무효화
        self.statistics_manager.invalidate(&table).await;

        Ok(Self::index_result(format!(
            "index created: {}",
            query.index_name
        )))
    }

    pub(crate) fn index_result(message: String) -> ExecuteResult {
        ExecuteResult::new(
            vec![ExecuteColumn {
                name: "desc".into(),
                data_type: ExecuteColumnType::String,
            }],
            vec![ExecuteRow {
                fields: vec![ExecuteField::String(message)],
            }],
        )
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use tokio::sync::Mutex;

    use tokio::io::AsyncWriteExt;

    use crate::config::launch_config::LaunchConfig;
    use crate::engine::ast::dml::plan::select::scan::ScanType;
    use crate::engine::ast::dml::plan::select::select_plan::SelectPlanItem;
    use crate::engine::ast::types::TableName;
    use crate::engine::ast::{DDLStatement, DMLStatement, SQLStatement};
    use crate::engine::optimizer::predule::Optimizer;
    use crate::engine::parser::predule::{Parser, ParserContext};
    use crate::engine::types::{ExecuteField, ExecuteResult};
    use crate::engine::wal::endec::implements::bincode::{BincodeDecoder, BincodeEncoder};
    use crate::engine::wal::endec::{WALDecoder, WALEncoder};
    use crate::engine::wal::manager::builder::WALBuilder;
    use crate::engine::wal::types::{EntryType, WALEntry};
    use crate::engine::{DBEngine, SharedWALManager};

    /// 엔진의 정상 write path(WAL append + apply)를 거치지 않고 WAL 세그먼트
    /// 파일에 엔트리를 직접 기록합니다. "WAL에는 기록됐지만 아직 테이블/인덱스에
    /// 반영되지 않은 채 크래시가 발생한 상태"를 시뮬레이션하는 데 사용합니다.
    ///
    /// `sequence`는 반드시 이 시점에 WALManager가 쓰고 있는 "현재" 세그먼트여야
    /// 합니다. 이미 checkpoint된(즉, 실제로 반영이 끝난) 이전 세그먼트에 엔트리를
    /// 추가하면, 재생 시 이미 적용된 연산까지 함께 재실행되어 데이터가
    /// 중복됩니다.
    async fn write_raw_wal_entry(
        config: &LaunchConfig,
        sequence: usize,
        entry_type: EntryType,
        payload: Vec<u8>,
    ) {
        let entry = WALEntry {
            entry_type,
            data: Some(payload),
            timestamp: 0,
            transaction_id: None,
            is_continuation: false,
        };

        let encoder = BincodeEncoder::new();
        let encoded = encoder.encode(&entry).unwrap();

        let mut frame = Vec::with_capacity(size_of::<u32>() + encoded.len());
        frame.extend_from_slice(&(encoded.len() as u32).to_le_bytes());
        frame.extend_from_slice(&encoded);

        let wal_path = PathBuf::from(&config.wal_directory)
            .join(format!("{:08X}.{}", sequence, config.wal_extension));

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&wal_path)
            .await
            .unwrap();

        file.write_all(&frame).await.unwrap();
        file.flush().await.unwrap();
    }

    async fn build_test_engine(test_name: &str) -> (DBEngine, SharedWALManager) {
        let base_path = PathBuf::from("target/test_index_integration").join(test_name);
        if base_path.exists() {
            tokio::fs::remove_dir_all(&base_path).await.unwrap();
        }

        let config = LaunchConfig::default_for_base_path(&base_path);
        tokio::fs::create_dir_all(&config.data_directory)
            .await
            .unwrap();
        tokio::fs::create_dir_all(&config.wal_directory)
            .await
            .unwrap();

        let wal = WALBuilder::new(&config)
            .build(BincodeDecoder::new(), BincodeEncoder::new())
            .await
            .unwrap();

        (DBEngine::new(config), Arc::new(Mutex::new(wal)))
    }

    async fn execute_sql(
        engine: &DBEngine,
        wal: SharedWALManager,
        sql: &str,
    ) -> crate::errors::Result<ExecuteResult> {
        let mut parser = Parser::with_string(sql.to_string())?;
        let mut statements =
            parser.parse(ParserContext::default().set_default_database("rrdb".to_string()))?;
        let statement = statements.remove(0);

        engine
            .process_query(statement, wal, "test-connection".to_string())
            .await
    }

    fn users_table() -> TableName {
        TableName::new(Some("rrdb".to_string()), "users".to_string())
    }

    async fn setup_users_table(engine: &DBEngine, wal: SharedWALManager) {
        execute_sql(engine, wal.clone(), "create database rrdb;")
            .await
            .unwrap();
        execute_sql(
            engine,
            wal,
            "create table users (id integer primary key, score integer);",
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn create_table_auto_creates_primary_key_index() {
        let (engine, wal) = build_test_engine("pk_auto_index").await;
        setup_users_table(&engine, wal.clone()).await;

        let meta = engine
            .index_manager
            .get_meta("rrdb.users_pkey")
            .await
            .expect("primary key index should be auto-created");

        assert_eq!(meta.column_name, "id");
        assert!(meta.is_unique);
        assert_eq!(meta.table_name, users_table());
    }

    #[tokio::test]
    async fn primary_key_index_rejects_duplicate_inserts() {
        let (engine, wal) = build_test_engine("pk_unique").await;
        setup_users_table(&engine, wal.clone()).await;

        execute_sql(
            &engine,
            wal.clone(),
            "insert into users (id, score) values (1, 10);",
        )
        .await
        .unwrap();

        let duplicated = execute_sql(
            &engine,
            wal.clone(),
            "insert into users (id, score) values (1, 20);",
        )
        .await;
        assert!(duplicated.is_err());

        // 배치 내 중복도 거부
        let batch_duplicated = execute_sql(
            &engine,
            wal.clone(),
            "insert into users (id, score) values (2, 10), (2, 20);",
        )
        .await;
        assert!(batch_duplicated.is_err());

        // 다른 값은 정상 입력
        execute_sql(
            &engine,
            wal,
            "insert into users (id, score) values (2, 20);",
        )
        .await
        .unwrap();

        assert_eq!(
            engine.index_manager.len("rrdb.users_pkey").await.unwrap(),
            2
        );
    }

    #[tokio::test]
    async fn create_index_backfills_existing_rows() {
        let (engine, wal) = build_test_engine("backfill").await;
        setup_users_table(&engine, wal.clone()).await;

        execute_sql(
            &engine,
            wal.clone(),
            "insert into users (id, score) values (1, 10), (2, 20), (3, 20);",
        )
        .await
        .unwrap();

        execute_sql(
            &engine,
            wal.clone(),
            "create index users_score_idx on users (score);",
        )
        .await
        .unwrap();

        assert_eq!(
            engine
                .index_manager
                .len("rrdb.users_score_idx")
                .await
                .unwrap(),
            3
        );

        // 중복 값이 있는 컬럼에 unique 인덱스 생성은 실패해야 함
        let unique_on_duplicates = execute_sql(
            &engine,
            wal.clone(),
            "create unique index users_score_uniq on users (score);",
        )
        .await;
        assert!(unique_on_duplicates.is_err());
        assert!(
            engine
                .index_manager
                .get_meta("rrdb.users_score_uniq")
                .await
                .is_none()
        );

        // IF NOT EXISTS는 기존 인덱스를 건너뜀
        execute_sql(
            &engine,
            wal,
            "create index if not exists users_score_idx on users (score);",
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn update_and_delete_maintain_indexes() {
        let (engine, wal) = build_test_engine("dml_maintenance").await;
        setup_users_table(&engine, wal.clone()).await;

        execute_sql(
            &engine,
            wal.clone(),
            "insert into users (id, score) values (1, 10), (2, 20), (3, 30);",
        )
        .await
        .unwrap();

        // UPDATE: 인덱스 컬럼 값 변경
        execute_sql(
            &engine,
            wal.clone(),
            "update users set id = 20 where id = 2;",
        )
        .await
        .unwrap();

        let result = execute_sql(
            &engine,
            wal.clone(),
            "select score from users where id = 20;",
        )
        .await
        .unwrap();
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0].fields[0], ExecuteField::Integer(20));

        // UPDATE로 고유 제약 위반 시 실패해야 함
        let conflict = execute_sql(
            &engine,
            wal.clone(),
            "update users set id = 1 where id = 3;",
        )
        .await;
        assert!(conflict.is_err());

        // DELETE: 소프트 삭제 -- row index는 그대로 유지되고 tombstone만 표시되며,
        // 삭제된 행의 인덱스 항목만 제거됩니다 (세그먼트 재작성/압축 없음).
        execute_sql(&engine, wal.clone(), "delete from users where id = 1;")
            .await
            .unwrap();

        assert_eq!(
            engine.index_manager.len("rrdb.users_pkey").await.unwrap(),
            2
        );

        // 삭제된 행은 더 이상 인덱스로 조회되지 않아야 함
        let deleted = execute_sql(&engine, wal.clone(), "select score from users where id = 1;")
            .await
            .unwrap();
        assert_eq!(deleted.rows.len(), 0);

        // row index가 이동하지 않으므로 남은 행은 계속 정확히 조회되어야 함
        let result = execute_sql(&engine, wal, "select score from users where id = 3;")
            .await
            .unwrap();
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0].fields[0], ExecuteField::Integer(30));
    }

    #[tokio::test]
    async fn drop_index_removes_index_and_selects_still_work() {
        let (engine, wal) = build_test_engine("drop_index").await;
        setup_users_table(&engine, wal.clone()).await;

        execute_sql(
            &engine,
            wal.clone(),
            "insert into users (id, score) values (1, 10), (2, 20);",
        )
        .await
        .unwrap();

        // 존재하지 않는 인덱스는 실패, IF EXISTS는 성공
        assert!(
            execute_sql(&engine, wal.clone(), "drop index missing_idx;")
                .await
                .is_err()
        );
        execute_sql(&engine, wal.clone(), "drop index if exists missing_idx;")
            .await
            .unwrap();

        execute_sql(&engine, wal.clone(), "drop index users_pkey;")
            .await
            .unwrap();
        assert!(
            engine
                .index_manager
                .get_meta("rrdb.users_pkey")
                .await
                .is_none()
        );

        // 인덱스 없이도 FullScan으로 정상 동작
        let result = execute_sql(&engine, wal, "select score from users where id = 2;")
            .await
            .unwrap();
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0].fields[0], ExecuteField::Integer(20));
    }

    #[tokio::test]
    async fn optimizer_selects_index_scan_for_large_table_and_returns_correct_rows() {
        let (engine, wal) = build_test_engine("optimizer_index_scan").await;
        setup_users_table(&engine, wal.clone()).await;

        // 인덱스 스캔이 비용상 유리해지도록 충분한 행을 입력
        let values = (1..=600)
            .map(|i| format!("({}, {})", i, i * 10))
            .collect::<Vec<_>>()
            .join(", ");
        execute_sql(
            &engine,
            wal.clone(),
            &format!("insert into users (id, score) values {};", values),
        )
        .await
        .unwrap();

        // 옵티마이저가 IndexScan을 선택하는지 검증
        let mut parser =
            Parser::with_string("select score from users where id = 42;".to_string()).unwrap();
        let statement = parser
            .parse(ParserContext::default().set_default_database("rrdb".to_string()))
            .unwrap()
            .remove(0);

        let query = match statement {
            crate::engine::ast::SQLStatement::DML(
                crate::engine::ast::DMLStatement::SelectQuery(query),
            ) => query,
            other => panic!("expected select query, got {:?}", other),
        };

        let context = engine.build_optimizer_context(&users_table()).await;
        let optimizer = Optimizer::with_context(context);
        let plan = optimizer.optimize_select(query).await.unwrap();

        match &plan.list[0] {
            SelectPlanItem::From(from) => match &from.scan {
                ScanType::IndexScan(index_scan) => {
                    assert_eq!(index_scan.index_name, "rrdb.users_pkey");
                }
                other => panic!("expected IndexScan, got {:?}", other),
            },
            other => panic!("expected From plan, got {:?}", other),
        }

        // 실제 실행 결과 검증 (동등 조건)
        let result = execute_sql(
            &engine,
            wal.clone(),
            "select score from users where id = 42;",
        )
        .await
        .unwrap();
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0].fields[0], ExecuteField::Integer(420));

        // 범위 조건 검증
        let result = execute_sql(
            &engine,
            wal.clone(),
            "select score from users where id > 595 and id <= 598;",
        )
        .await
        .unwrap();
        assert_eq!(result.rows.len(), 3);

        // 엔진 재기동 후 디스크에서 인덱스를 다시 적재해 동작해야 함
        // 참고: row buffer 풀 도입으로 재기동 전 flush 필수
        engine.flush_row_buffers_durable().await.unwrap();
        let restarted = DBEngine::new(engine.config.as_ref().clone());
        let result = execute_sql(&restarted, wal, "select score from users where id = 100;")
            .await
            .unwrap();
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0].fields[0], ExecuteField::Integer(1000));
    }

    #[tokio::test]
    async fn wal_replay_reconstructs_insert_after_simulated_crash() {
        let (engine, wal) = build_test_engine("wal_replay_insert").await;
        setup_users_table(&engine, wal.clone()).await;

        let mut parser =
            Parser::with_string("insert into users (id, score) values (7, 70);".to_string())
                .unwrap();
        let statement = parser
            .parse(ParserContext::default().set_default_database("rrdb".to_string()))
            .unwrap()
            .remove(0);
        let insert_query = match statement {
            SQLStatement::DML(DMLStatement::InsertQuery(query)) => query,
            other => panic!("expected insert query, got {:?}", other),
        };

        // WAL에는 기록되었지만 아직 반영되지 않은 채 크래시가 난 상황을 시뮬레이션합니다:
        // engine.insert()를 거치지 않고 WAL 파일에 직접 엔트리를 씁니다.
        let config = engine.config.as_ref().clone();
        let sequence = wal.lock().await.current_sequence();
        let payload = bincode::serialize(&insert_query).unwrap();
        write_raw_wal_entry(&config, sequence, EntryType::Insert, payload).await;

        // 재기동 시뮬레이션: 같은 데이터 디렉터리로 새 엔진과 새 WALManager를 빌드합니다.
        let restarted = DBEngine::new(config.clone());
        let wal_manager2 = WALBuilder::new(&config)
            .build(BincodeDecoder::new(), BincodeEncoder::new())
            .await
            .unwrap();

        let pending = wal_manager2.pending_entries().to_vec();
        assert_eq!(
            pending.len(),
            1,
            "the un-checkpointed insert should be pending replay"
        );

        restarted.replay_wal(&pending).await.unwrap();

        let rows = restarted.full_scan(users_table()).await.unwrap();
        assert_eq!(rows.len(), 1);

        let wal2 = Arc::new(Mutex::new(wal_manager2));
        let result = execute_sql(&restarted, wal2, "select score from users where id = 7;")
            .await
            .unwrap();
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0].fields[0], ExecuteField::Integer(70));
    }

    #[tokio::test]
    async fn wal_replay_reconstructs_create_index_after_simulated_crash() {
        let (engine, wal) = build_test_engine("wal_replay_create_index").await;
        setup_users_table(&engine, wal.clone()).await;

        execute_sql(
            &engine,
            wal.clone(),
            "insert into users (id, score) values (1, 10), (2, 20), (3, 20);",
        )
        .await
        .unwrap();

        let mut parser =
            Parser::with_string("create index users_score_idx on users (score);".to_string())
                .unwrap();
        let statement = parser
            .parse(ParserContext::default().set_default_database("rrdb".to_string()))
            .unwrap()
            .remove(0);
        let create_index_query = match statement {
            SQLStatement::DDL(DDLStatement::CreateIndexQuery(query)) => query,
            other => panic!("expected create index query, got {:?}", other),
        };

        // 앞선 INSERT는 정상 경로로 이미 반영되었으므로, row buffer + WAL checkpoint로
        // durable 경계를 만들어 재생 대상에서 제외합니다 (그렇지 않으면 재생 시 INSERT가
        // 중복 적용됩니다). 이후 CREATE INDEX만 WAL에 기록되고 아직 인덱스
        // 매니저에는 반영되지 않은 채 크래시가 난 상황을 시뮬레이션합니다.
        engine.flush_row_buffers_durable().await.unwrap();
        wal.lock().await.flush().await.unwrap();

        let config = engine.config.as_ref().clone();
        let sequence = wal.lock().await.current_sequence();
        let payload = bincode::serialize(&create_index_query).unwrap();
        write_raw_wal_entry(&config, sequence, EntryType::CreateIndex, payload).await;

        let restarted = DBEngine::new(config.clone());
        let wal_manager2 = WALBuilder::new(&config)
            .build(BincodeDecoder::new(), BincodeEncoder::new())
            .await
            .unwrap();

        let pending = wal_manager2.pending_entries().to_vec();
        assert_eq!(pending.len(), 1);

        restarted.replay_wal(&pending).await.unwrap();

        assert!(
            restarted
                .index_manager
                .get_meta("rrdb.users_score_idx")
                .await
                .is_some(),
            "replay should recreate the index metadata"
        );
        assert_eq!(
            restarted
                .index_manager
                .len("rrdb.users_score_idx")
                .await
                .unwrap(),
            3,
            "replay should backfill the index from existing rows"
        );
    }

    #[tokio::test]
    async fn wal_replay_reconstructs_drop_index_after_simulated_crash() {
        let (engine, wal) = build_test_engine("wal_replay_drop_index").await;
        setup_users_table(&engine, wal.clone()).await;

        execute_sql(
            &engine,
            wal.clone(),
            "create index users_score_idx on users (score);",
        )
        .await
        .unwrap();

        assert!(
            engine
                .index_manager
                .get_meta("rrdb.users_score_idx")
                .await
                .is_some()
        );

        let mut parser = Parser::with_string("drop index users_score_idx;".to_string()).unwrap();
        let statement = parser
            .parse(ParserContext::default().set_default_database("rrdb".to_string()))
            .unwrap()
            .remove(0);
        let drop_index_query = match statement {
            SQLStatement::DDL(DDLStatement::DropIndexQuery(query)) => query,
            other => panic!("expected drop index query, got {:?}", other),
        };

        // 앞선 CREATE INDEX는 정상 경로로 이미 반영되었으므로, checkpoint로
        // durable 경계를 만들어 재생 대상에서 제외합니다. 이후 DROP INDEX만
        // WAL에 기록되고 아직 인덱스 매니저에는 반영되지 않은 채 크래시가 난
        // 상황을 시뮬레이션합니다.
        wal.lock().await.flush().await.unwrap();

        let config = engine.config.as_ref().clone();
        let sequence = wal.lock().await.current_sequence();
        let payload = bincode::serialize(&drop_index_query).unwrap();
        write_raw_wal_entry(&config, sequence, EntryType::DropIndex, payload).await;

        let restarted = DBEngine::new(config.clone());
        let wal_manager2 = WALBuilder::new(&config)
            .build(BincodeDecoder::new(), BincodeEncoder::new())
            .await
            .unwrap();

        let pending = wal_manager2.pending_entries().to_vec();
        assert_eq!(pending.len(), 1);

        restarted.replay_wal(&pending).await.unwrap();

        assert!(
            restarted
                .index_manager
                .get_meta("rrdb.users_score_idx")
                .await
                .is_none(),
            "replay should drop the index"
        );
    }

    #[tokio::test]
    async fn update_wal_first_writes_entry_before_applying_mutation() {
        let (engine, wal) = build_test_engine("update_wal_first").await;
        setup_users_table(&engine, wal.clone()).await;

        execute_sql(
            &engine,
            wal.clone(),
            "insert into users (id, score) values (1, 10), (2, 20);",
        )
        .await
        .unwrap();

        // 고유 제약 위반으로 인덱스 반영은 실패하지만, WAL-first이므로 WAL
        // 엔트리는 이미 기록된 뒤여야 합니다.
        let result = execute_sql(
            &engine,
            wal.clone(),
            "update users set id = 1 where id = 2;",
        )
        .await;
        assert!(result.is_err());

        let config = engine.config.as_ref().clone();
        let wal_path = PathBuf::from(&config.wal_directory)
            .join(format!("{:08X}.{}", 1, config.wal_extension));
        let content = tokio::fs::read(&wal_path).await.unwrap();
        let decoder = BincodeDecoder::new();
        let entries = decoder.decode(&content).unwrap();
        assert!(
            entries
                .iter()
                .any(|entry| matches!(entry.entry_type, EntryType::Set)),
            "WAL should already contain the Set entry even though the mutation was rolled back"
        );

        // 실패 시 인덱스/테이블 반영은 롤백되어 실제 데이터는 변하지 않아야 합니다.
        let result = execute_sql(&engine, wal, "select score from users where id = 2;")
            .await
            .unwrap();
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0].fields[0], ExecuteField::Integer(20));
    }
}
