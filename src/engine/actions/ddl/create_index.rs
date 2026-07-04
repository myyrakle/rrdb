use std::collections::HashSet;

use crate::engine::DBEngine;
use crate::engine::actions::index::{qualified_index_name, row_index_key};
use crate::engine::ast::ddl::create_index::CreateIndexQuery;
use crate::engine::index::{IndexEntry, IndexMeta};
use crate::engine::types::{
    ExecuteColumn, ExecuteColumnType, ExecuteField, ExecuteResult, ExecuteRow,
};
use crate::errors;
use crate::errors::execute_error::ExecuteError;

impl DBEngine {
    pub async fn create_index(&self, query: CreateIndexQuery) -> errors::Result<ExecuteResult> {
        self.ensure_indices_loaded().await?;

        let table = query.table.clone();
        let database_name = table
            .database_name
            .clone()
            .ok_or_else(|| ExecuteError::wrap("database name is required".to_string()))?;

        // 테이블 존재 검증
        let table_config = self.get_table_config_cached(table.clone()).await?;

        // TODO(#217): 다중 컬럼 인덱스 지원
        if query.columns.len() != 1 {
            return Err(ExecuteError::wrap(
                "multi-column indexes are not supported yet".to_string(),
            ));
        }

        let column_name = query.columns[0].clone();

        // 컬럼 존재 검증
        if !table_config
            .columns
            .iter()
            .any(|column| column.name == column_name)
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

        let meta = IndexMeta::new(
            index_name.clone(),
            table.clone(),
            column_name.clone(),
            query.is_unique,
        );

        self.index_manager.create_index(meta).await?;

        // 기존 행 색인 (backfill)
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

    use crate::config::launch_config::LaunchConfig;
    use crate::engine::ast::dml::plan::select::scan::ScanType;
    use crate::engine::ast::dml::plan::select::select_plan::SelectPlanItem;
    use crate::engine::ast::types::TableName;
    use crate::engine::optimizer::predule::Optimizer;
    use crate::engine::parser::predule::{Parser, ParserContext};
    use crate::engine::types::{ExecuteField, ExecuteResult};
    use crate::engine::wal::endec::implements::bincode::{BincodeDecoder, BincodeEncoder};
    use crate::engine::wal::manager::builder::WALBuilder;
    use crate::engine::{DBEngine, SharedWALManager};

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

        // DELETE: 세그먼트 압축 후 인덱스가 재구축되어야 함
        execute_sql(&engine, wal.clone(), "delete from users where id = 1;")
            .await
            .unwrap();

        assert_eq!(
            engine.index_manager.len("rrdb.users_pkey").await.unwrap(),
            2
        );

        // 압축으로 row index가 이동한 뒤에도 인덱스 조회가 정확해야 함
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
        let restarted = DBEngine::new(engine.config.as_ref().clone());
        let result = execute_sql(&restarted, wal, "select score from users where id = 100;")
            .await
            .unwrap();
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0].fields[0], ExecuteField::Integer(1000));
    }
}
