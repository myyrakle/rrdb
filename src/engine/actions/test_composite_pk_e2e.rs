use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::config::launch_config::LaunchConfig;
use crate::engine::ast::types::TableName;
use crate::engine::parser::predule::{Parser, ParserContext};
use crate::engine::types::{ExecuteField, ExecuteResult};
use crate::engine::wal::endec::implements::bincode::{BincodeDecoder, BincodeEncoder};
use crate::engine::wal::manager::builder::WALBuilder;
use crate::engine::{DBEngine, SharedWALManager};

/// 복합 PRIMARY KEY 엔드투엔드 통합 테스트 (#220)
///
/// 커버리지:
/// - 복합 PK 테이블 생성 시 `{table}_pkey` 인덱스 자동 생성
/// - 복합 키 유니크 강제: 개별 중복 + 배치 중복 거부, 다른 조합은 허용
/// - 복합 키 조회: 옵티마이저가 복합 인덱스 prefix 스캔 미지원이라도
///   (FullScan 폴백) 정확한 결과 반환 — 특히 큰 테이블에서 회귀 테스트
/// - UPDATE/DELETE 유지보수
/// - 재기동 후 persisted index 재적재
/// - 인덱스 유지보수 실패 시 롤백 (orphan row 없음)

async fn build_test_engine(test_name: &str) -> (DBEngine, SharedWALManager) {
    let base_path = PathBuf::from("target/test_composite_pk").join(test_name);

    // 남은 이전 실행 정리 — remove_dir_all은 존재하지 않아도 실패를 신경 쓰지 않음
    let _ = tokio::fs::remove_dir_all(&base_path).await;

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

fn memberships_table() -> TableName {
    TableName::new(Some("rrdb".to_owned()), "memberships".to_owned())
}

async fn setup_memberships_table(engine: &DBEngine, wal: SharedWALManager) {
    execute_sql(engine, wal.clone(), "create database rrdb;")
        .await
        .unwrap();
    execute_sql(
        engine,
        wal,
        "create table memberships (user_id integer, group_id integer, primary key (user_id, group_id));",
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn composite_pk_table_auto_creates_composite_index() {
    let (engine, wal) = build_test_engine("auto_composite_index").await;
    setup_memberships_table(&engine, wal).await;

    let meta = engine
        .index_manager
        .get_meta("rrdb.memberships_pkey")
        .await
        .expect("composite PK index should be auto-created");

    assert_eq!(meta.columns(), &["user_id", "group_id"]);
    assert!(meta.is_unique);
    assert_eq!(meta.table_name, memberships_table());
}

#[tokio::test]
async fn composite_pk_rejects_duplicate_key_combinations() {
    let (engine, wal) = build_test_engine("duplicate_combinations").await;
    setup_memberships_table(&engine, wal.clone()).await;

    execute_sql(
        &engine,
        wal.clone(),
        "insert into memberships (user_id, group_id) values (1, 1);",
    )
    .await
    .unwrap();

    // 같은 조합 재삽입 거부
    let duplicate = execute_sql(
        &engine,
        wal.clone(),
        "insert into memberships (user_id, group_id) values (1, 1);",
    )
    .await;
    assert!(duplicate.is_err(), "identical combination must be rejected");

    // 배치 내 중복 조합 거부
    let batch_duplicate = execute_sql(
        &engine,
        wal.clone(),
        "insert into memberships (user_id, group_id) values (2, 2), (2, 2);",
    )
    .await;
    assert!(batch_duplicate.is_err(), "batch duplicate must be rejected");

    // 개별 컬럼 값이 같아도 조합이 다르면 허용 (복합 PK의 핵심)
    execute_sql(
        &engine,
        wal.clone(),
        "insert into memberships (user_id, group_id) values (1, 2);",
    )
    .await
    .expect("different combination with reused user_id must be accepted");
    execute_sql(
        &engine,
        wal.clone(),
        "insert into memberships (user_id, group_id) values (2, 1);",
    )
    .await
    .expect("different combination with reused group_id must be accepted");

    assert_eq!(
        engine.full_scan(memberships_table()).await.unwrap().len(),
        3
    );
}

#[tokio::test]
async fn composite_pk_lookup_returns_exactly_the_matching_rows() {
    let (engine, wal) = build_test_engine("lookup").await;
    setup_memberships_table(&engine, wal.clone()).await;

    execute_sql(
        &engine,
        wal.clone(),
        "insert into memberships (user_id, group_id) values (1, 1), (1, 2), (2, 1), (3, 3);",
    )
    .await
    .unwrap();

    // PK 선행 컬럼 동등 조건: user_id = 1 → 2행
    let result = execute_sql(
        &engine,
        wal.clone(),
        "select group_id from memberships where user_id = 1;",
    )
    .await
    .unwrap();
    assert_eq!(result.rows.len(), 2, "user_id = 1 must match two rows");

    // 정확한 조합 조회: user_id = 1 and group_id = 2 → 1행
    let result = execute_sql(
        &engine,
        wal.clone(),
        "select user_id from memberships where user_id = 1 and group_id = 2;",
    )
    .await
    .unwrap();
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0].fields[0], ExecuteField::Integer(1));
}

#[tokio::test]
async fn composite_pk_lookup_is_correct_on_a_large_table() {
    // 회귀 테스트: 옵티마이저가 복합 인덱스를 IndexScan 후보에서 제외하지
    // 않던 시절, 큰 테이블에서 첫 컬럼 동등 조건이 raw eq_key로 인덱스를
    // 조회해 조건을 만족하는 행을 전부 누락했다 (0행 반환).
    // prefix 스캔 미지원 복합 인덱스는 FullScan으로 폴백해야 정확하다 (#220).
    let (engine, wal) = build_test_engine("lookup_large").await;
    setup_memberships_table(&engine, wal.clone()).await;

    let mut values = String::new();
    for i in 0..1000 {
        if i > 0 {
            values.push_str(", ");
        }
        values.push_str(&format!("({}, {})", i, i % 7));
    }

    execute_sql(
        &engine,
        wal.clone(),
        &format!(
            "insert into memberships (user_id, group_id) values {};",
            values
        ),
    )
    .await
    .unwrap();

    // 첫 컬럼 동등 조건: user_id = 42 → 정확히 1행
    let result = execute_sql(
        &engine,
        wal.clone(),
        "select group_id from memberships where user_id = 42;",
    )
    .await
    .unwrap();
    assert_eq!(
        result.rows.len(),
        1,
        "user_id = 42 must match exactly one row"
    );
    assert_eq!(result.rows[0].fields[0], ExecuteField::Integer(42 % 7));

    // 존재하지 않는 키: 0행 (과거 버그로는 0행이 정답처럼 보였지만
    // 존재하는 키도 0행이 나왔었다. 이번엔 존재하는 키가 1행 나오는지가 핵심)
    let result = execute_sql(
        &engine,
        wal.clone(),
        "select user_id from memberships where user_id = 9999;",
    )
    .await
    .unwrap();
    assert_eq!(result.rows.len(), 0);

    // 범위 조건도 정확해야 함: 500 <= user_id < 510 → 10행
    let result = execute_sql(
        &engine,
        wal.clone(),
        "select user_id from memberships where user_id >= 500 and user_id < 510;",
    )
    .await
    .unwrap();
    assert_eq!(result.rows.len(), 10);
}

#[tokio::test]
async fn composite_pk_maintains_index_across_update_and_delete() {
    let (engine, wal) = build_test_engine("update_delete").await;
    setup_memberships_table(&engine, wal.clone()).await;

    execute_sql(
        &engine,
        wal.clone(),
        "insert into memberships (user_id, group_id) values (1, 1), (2, 2);",
    )
    .await
    .unwrap();

    assert_eq!(
        engine
            .index_manager
            .len("rrdb.memberships_pkey")
            .await
            .unwrap(),
        2
    );

    // UPDATE: 복합 PK 컬럼 변경 — (2,2) -> (2,3)
    execute_sql(
        &engine,
        wal.clone(),
        "update memberships set group_id = 3 where user_id = 2 and group_id = 2;",
    )
    .await
    .unwrap();

    // 변경된 조합으로 조회 가능
    let result = execute_sql(
        &engine,
        wal.clone(),
        "select user_id from memberships where user_id = 2 and group_id = 3;",
    )
    .await
    .unwrap();
    assert_eq!(result.rows.len(), 1);

    // 기존 조합으로는 조회 불가
    let result = execute_sql(
        &engine,
        wal.clone(),
        "select user_id from memberships where user_id = 2 and group_id = 2;",
    )
    .await
    .unwrap();
    assert_eq!(result.rows.len(), 0);

    // UPDATE로 유니크 위반 유도 — (1,1)을 이미 존재하는 (2,3)으로
    let conflict = execute_sql(
        &engine,
        wal.clone(),
        "update memberships set user_id = 2, group_id = 3 where user_id = 1 and group_id = 1;",
    )
    .await;
    assert!(
        conflict.is_err(),
        "duplicate combination via update must fail"
    );

    // DELETE: 인덱스 항목 제거
    execute_sql(
        &engine,
        wal.clone(),
        "delete from memberships where user_id = 1 and group_id = 1;",
    )
    .await
    .unwrap();

    assert_eq!(
        engine
            .index_manager
            .len("rrdb.memberships_pkey")
            .await
            .unwrap(),
        1,
        "index entries must track live rows after delete"
    );
}

#[tokio::test]
async fn composite_pk_index_survives_restart() {
    let (engine, wal) = build_test_engine("restart").await;
    setup_memberships_table(&engine, wal.clone()).await;

    execute_sql(
        &engine,
        wal.clone(),
        "insert into memberships (user_id, group_id) values (7, 8);",
    )
    .await
    .unwrap();

    engine.flush_row_buffers_durable().await.unwrap();

    let restarted = DBEngine::new(engine.config.as_ref().clone());
    let result = execute_sql(
        &restarted,
        wal,
        "select user_id from memberships where user_id = 7 and group_id = 8;",
    )
    .await
    .unwrap();
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0].fields[0], ExecuteField::Integer(7));
}

#[tokio::test]
async fn failed_composite_pk_insert_does_not_leave_orphan_row() {
    let (engine, wal) = build_test_engine("orphan").await;
    setup_memberships_table(&engine, wal.clone()).await;

    let (first, second) = tokio::join!(
        execute_sql(
            &engine,
            wal.clone(),
            "insert into memberships (user_id, group_id) values (1, 1);"
        ),
        execute_sql(
            &engine,
            wal.clone(),
            "insert into memberships (user_id, group_id) values (1, 1);"
        ),
    );

    let successes = [&first, &second].iter().filter(|r| r.is_ok()).count();
    assert_eq!(
        successes, 1,
        "exactly one of the racing identical inserts should succeed"
    );

    let rows = engine.full_scan(memberships_table()).await.unwrap();
    assert_eq!(
        rows.len(),
        1,
        "a failed composite-PK insert must not leave an orphan row"
    );

    let index_entries = engine
        .index_manager
        .scan_all("rrdb.memberships_pkey")
        .await
        .unwrap();
    assert_eq!(
        index_entries.len(),
        rows.len(),
        "index entry count must match live row count after rollback"
    );
}
