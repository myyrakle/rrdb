use super::*;
use crate::engine::SharedWALManager;
use crate::engine::parser::predule::{Parser, ParserContext};
use crate::engine::types::ExecuteResult;
use crate::engine::wal::endec::implements::bincode::{BincodeDecoder, BincodeEncoder};
use crate::engine::wal::manager::builder::WALBuilder;

async fn sql(engine: &DBEngine, wal: SharedWALManager, sql: &str) -> errors::Result<ExecuteResult> {
    let statement = Parser::with_string(sql.to_string())?
        .parse(ParserContext::default().set_default_database("rrdb".into()))?
        .remove(0);
    engine
        .process_query(statement, wal, "offset-test".into())
        .await
}
async fn sql_engine(label: &str) -> (DBEngine, SharedWALManager) {
    let base =
        PathBuf::from("target/test_row_offsets").join(format!("{label}-{}", uuid::Uuid::new_v4()));
    let config = LaunchConfig::default_for_base_path(base);
    tokio::fs::create_dir_all(&config.data_directory)
        .await
        .unwrap();
    tokio::fs::create_dir_all(&config.wal_directory)
        .await
        .unwrap();
    let wal = Arc::new(tokio::sync::Mutex::new(
        WALBuilder::new(&config)
            .build(BincodeDecoder::new(), BincodeEncoder::new())
            .await
            .unwrap(),
    ));
    let engine = DBEngine::new(config);
    sql(&engine, wal.clone(), "create database rrdb;")
        .await
        .unwrap();
    sql(
        &engine,
        wal.clone(),
        "create table offsets (id integer primary key, value varchar(4096));",
    )
    .await
    .unwrap();
    (engine, wal)
}

#[tokio::test]
async fn offset_cold_sql_statistics_do_not_decode_or_read_unselected_bodies() {
    let (engine, wal) = sql_engine("cold-sql").await;
    let data = (0..1000)
        .map(|id| format!("({id}, '{}')", "x".repeat(1024)))
        .collect::<Vec<_>>()
        .join(",");
    sql(
        &engine,
        wal.clone(),
        &format!("insert into offsets (id, value) values {data};"),
    )
    .await
    .unwrap();
    engine.flush_row_buffers_durable().await.unwrap();
    let mut config = (*engine.config).clone();
    config.max_query_memory_bytes = 64 * 1024;
    assert!(
        std::fs::metadata(engine.row_segment_path(&table()).unwrap())
            .unwrap()
            .len()
            > config.max_query_memory_bytes
    );
    let mut reopened = DBEngine::new(config);
    let counts = instrument(&mut reopened);
    let selected = sql(&reopened, wal, "select id from offsets where id = 999;")
        .await
        .unwrap();
    assert_eq!(selected.rows.len(), 1);
    let io = take(&counts);
    assert_eq!(io.reads.iter().filter(|(_, len)| *len == 5).count(), 1000);
    assert_eq!(io.reads.iter().filter(|(_, len)| *len > 5).count(), 1);
    assert_eq!(
        io.opens, 2,
        "one statistics handle and one selected-row handle"
    );
    assert!(
        reopened
            .row_buffer_pool
            .lock()
            .await
            .cached_rows(&reopened.row_segment_path(&table()).unwrap())
            .is_none()
    );
    cleanup(&engine).await;
}

#[tokio::test]
async fn offset_wal_recovery_rebuilds_indexed_reads() {
    let (engine, wal) = sql_engine("recovery").await;
    sql(
        &engine,
        wal.clone(),
        "insert into offsets (id, value) values (1, 'old'), (2, 'remove');",
    )
    .await
    .unwrap();
    engine.flush_row_buffers_durable().await.unwrap();
    wal.lock().await.flush().await.unwrap();
    // Crash after WAL durability but BEFORE applying rows/index changes.
    // Replaying a fully applied SQL tail with already-written unique-index
    // pages is a known baseline failure, outside the direct-offset contract.
    use crate::engine::ast::{DMLStatement, SQLStatement};
    use crate::engine::parser::predule::{Parser, ParserContext};
    use crate::engine::wal::types::{EntryType, InsertWALPayload};
    for statement in [
        "insert into offsets (id, value) values (3, 'replayed');".to_string(),
        "update offsets set value = 'a much longer recovered value' where id = 1;".to_string(),
        "delete from offsets where id = 2;".to_string(),
    ] {
        let statement = Parser::with_string(statement)
            .unwrap()
            .parse(ParserContext::default().set_default_database("rrdb".into()))
            .unwrap()
            .remove(0);
        let (kind, payload) = match statement {
            SQLStatement::DML(DMLStatement::InsertQuery(query)) => (
                EntryType::Insert,
                bincode::serialize(&InsertWALPayload {
                    query,
                    start_row_index: 2,
                    row_count: 1,
                })
                .unwrap(),
            ),
            SQLStatement::DML(DMLStatement::UpdateQuery(query)) => {
                (EntryType::Set, bincode::serialize(&query).unwrap())
            }
            SQLStatement::DML(DMLStatement::DeleteQuery(query)) => {
                (EntryType::Delete, bincode::serialize(&query).unwrap())
            }
            other => panic!("unexpected replay statement: {other:?}"),
        };
        wal.lock()
            .await
            .append_record(kind, Some(payload), None)
            .await
            .unwrap();
    }
    wal.lock().await.sync().await.unwrap();
    let config = (*engine.config).clone();
    drop(engine); // buffered rows intentionally lost, WAL retained
    drop(wal);
    let recovered = DBEngine::new(config.clone());
    let mut wal = WALBuilder::new(&config)
        .build(BincodeDecoder::new(), BincodeEncoder::new())
        .await
        .unwrap();
    assert!(!wal.pending_entries().is_empty());
    recovered.recover_from_wal(&mut wal).await.unwrap();
    let reopened = DBEngine::new(config);
    let range = IndexScanPlan {
        index_name: "rrdb.offsets_pkey".into(),
        column_name: "id".into(),
        eq_key: None,
        start_key: None,
        end_key: None,
    };
    let rows = reopened.index_scan(table(), &range).await.unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows.iter()
            .map(|(_, row)| row.fields[0].data.clone())
            .collect::<Vec<_>>(),
        vec![
            TableDataFieldType::Integer(1),
            TableDataFieldType::Integer(3)
        ]
    );
    assert_eq!(
        rows[0].1.fields[1].data,
        TableDataFieldType::String("a much longer recovered value".into())
    );
    assert_eq!(rows[1].0.row_index, 2, "tombstone retains logical slot");
    cleanup(&reopened).await;
}

#[tokio::test]
async fn offset_uncached_suffix_fallback_preserves_order_and_statistics() {
    let mut engine = engine("suffix").await;
    let counts = instrument(&mut engine);
    engine
        .append_table_rows(
            &table(),
            &(0..5).map(|id| row(&id.to_string())).collect::<Vec<_>>(),
        )
        .await
        .unwrap();
    engine.flush_row_buffers().await.unwrap();
    pointer(&engine, "a", "4").await;
    pointer(&engine, "b", "2").await;
    engine.index_scan(table(), &plan(Some("a"))).await.unwrap();
    let path = engine.row_segment_path(&table()).unwrap();
    // Same cache shape as a real cap crossing, without a million I/O calls.
    let mut directory = FrameDirectory::default();
    for id in 0..5 {
        directory
            .extend(&encode_row_frames(&[Some(row(&id.to_string()))]).unwrap())
            .unwrap();
    }
    directory.frames.truncate(2);
    engine
        .row_buffer_pool
        .lock()
        .await
        .cache_directory(path, directory);
    take(&counts);
    assert_eq!(
        values(engine.index_scan(table(), &plan(None)).await.unwrap()),
        vec![(4, "4".into()), (2, "2".into())]
    );
    assert_eq!(
        take(&counts)
            .reads
            .iter()
            .filter(|(_, len)| *len == 5)
            .count(),
        4
    );
    assert_eq!(
        engine.table_statistics(&table()).await.unwrap().row_count,
        5
    );
    engine
        .append_table_rows(&table(), &[row("5")])
        .await
        .unwrap();
    engine.flush_row_buffers().await.unwrap();
    pointer(&engine, "c", "5").await;
    assert_eq!(
        values(engine.index_scan(table(), &plan(Some("c"))).await.unwrap()),
        vec![(5, "5".into())]
    );
    cleanup(&engine).await;
}

#[tokio::test]
async fn offset_physical_rename_forgets_old_namespace_even_if_config_update_fails() {
    for database in [false, true] {
        let (engine, wal) = sql_engine("rename").await;
        sql(
            &engine,
            wal.clone(),
            "insert into offsets (id, value) values (1, 'old');",
        )
        .await
        .unwrap();
        engine.flush_row_buffers().await.unwrap();
        engine.row_count_for_statistics(&table()).await.unwrap();
        engine
            .append_table_rows(&table(), &[row("pending")])
            .await
            .unwrap();
        let old_path = engine.row_segment_path(&table()).unwrap();
        // The PK index holds open files inside the directory. Windows cannot
        // rename a directory with open children; index-handle retargeting is an
        // existing DDL limitation, outside this offset-invalidation regression.
        engine.index_manager.remove_table_indices(&table()).await;
        assert!(
            engine
                .row_buffer_pool
                .lock()
                .await
                .directory(&old_path)
                .is_some(),
            "releasing index handles must leave the offset directory warm"
        );
        // Table rename has an existing post-rename config lookup defect. Only
        // verify this change's namespace boundary, not that unrelated DDL behavior.
        let rename_result = sql(
            &engine,
            wal,
            if database {
                "alter database rrdb rename to moved;"
            } else {
                "alter table offsets rename to moved;"
            },
        )
        .await;
        assert!(
            !old_path.exists(),
            "physical rename must actually have occurred (database={database}): {rename_result:?}"
        );
        assert!(
            engine
                .row_buffer_pool
                .lock()
                .await
                .directory(&old_path)
                .is_none()
        );
        // Do not assert pending-row retargeting or the existing table-config
        // rename behavior: those baseline issues are not changed by this PR.
        cleanup(&engine).await;
    }
}

#[tokio::test]
async fn offset_failed_rename_retains_directory() {
    for database in [false, true] {
        let (engine, wal) = sql_engine("rename-failure").await;
        sql(
            &engine,
            wal.clone(),
            "insert into offsets (id, value) values (1, 'disk');",
        )
        .await
        .unwrap();
        engine.flush_row_buffers().await.unwrap();
        engine.row_count_for_statistics(&table()).await.unwrap();
        let old_path = engine.row_segment_path(&table()).unwrap();
        let destination = if database {
            engine.get_data_directory().join("moved")
        } else {
            engine.get_data_directory().join("rrdb/moved")
        };
        tokio::fs::write(&destination, b"blocks rename")
            .await
            .unwrap();
        let query = if database {
            "alter database rrdb rename to moved;"
        } else {
            "alter table offsets rename to moved;"
        };
        assert!(sql(&engine, wal, query).await.is_err());
        assert!(
            engine
                .row_buffer_pool
                .lock()
                .await
                .directory(&old_path)
                .is_some()
        );
        assert!(old_path.exists());
        cleanup(&engine).await;
    }
}

#[test]
fn offset_namespace_invalidation_is_component_aware() {
    use crate::engine::row_offsets::FrameDirectory;
    let mut pool = RowBufferPool::default();
    let path = std::path::PathBuf::from("data/db_extra/tables/t/rows/00000001.rows");
    pool.cache_directory(path.clone(), FrameDirectory::default());
    pool.invalidate_directory_under(std::path::Path::new("data/db"));
    assert!(pool.directory(&path).is_some());
    pool.invalidate_directory_under(std::path::Path::new("data/db_extra"));
    assert!(pool.directory(&path).is_none());
}

/// Run: cargo test --release --lib offset_read_benchmark -- --ignored --nocapture
/// Every "cold-directory" sample clears decoded rows and offsets, NOT OS pages.
/// Warm index samples retain offsets, never a full decoded-row cache. Full scan
/// samples include filtering identical row-index ranges after materialization.
#[tokio::test]
#[ignore = "release-mode I/O benchmark, not a timing assertion"]
async fn offset_read_benchmark() {
    use std::time::Instant;
    for n in [1000, 10_000] {
        let mut engine = engine("bench").await;
        let counts = instrument(&mut engine);
        let rows: Vec<_> = (0..n)
            .map(|id| row(&format!("{id:05}:{}", "x".repeat(1024))))
            .collect();
        engine.append_table_rows(&table(), &rows).await.unwrap();
        engine.flush_row_buffers_durable().await.unwrap();
        let start = n / 2;
        for id in start..start + 64 {
            pointer(&engine, &format!("{id:05}"), &id.to_string()).await;
        }
        let point = plan(Some(&format!("{start:05}")));
        let range = plan(None);
        let bytes = std::fs::metadata(engine.row_segment_path(&table()).unwrap())
            .unwrap()
            .len();
        println!("DATA rows={n} segment_bytes={bytes} range_matches=64 OS_CACHE=not_evicted");
        for (name, repeats, cold, full, ranged) in [
            ("full-cold-decoded-point", 3, true, true, false),
            ("offset-cold-directory-point", 3, true, false, false),
            ("offset-warm-point", 30, false, false, false),
            ("offset-cold-directory-range", 3, true, false, true),
            ("offset-warm-range", 10, false, false, true),
            ("full-cold-decoded-range", 3, true, true, true),
            ("full-warm-decoded-range", 10, false, true, true),
        ] {
            let mut elapsed = std::time::Duration::ZERO;
            take(&counts);
            for _ in 0..repeats {
                if cold {
                    *engine.row_buffer_pool.lock().await = RowBufferPool::default();
                }
                let timer = Instant::now();
                let result = if full {
                    engine
                        .full_scan(table())
                        .await
                        .unwrap()
                        .into_iter()
                        .filter(|(location, _)| {
                            if ranged {
                                (start..start + 64).contains(&location.row_index)
                            } else {
                                location.row_index == start
                            }
                        })
                        .collect::<Vec<_>>()
                } else {
                    engine
                        .index_scan(table(), if ranged { &range } else { &point })
                        .await
                        .unwrap()
                };
                elapsed += timer.elapsed();
                assert_eq!(result.len(), if ranged { 64 } else { 1 });
                for (location, data) in result {
                    assert_eq!(data.fields, rows[location.row_index].fields);
                }
            }
            let io = take(&counts);
            // Full scans use tokio::fs::read directly, outside CountingFile.
            // Report their expected bytes from file length + reset cache state,
            // not a measured disk-I/O counter. Random read lengths are observed.
            println!(
                "BENCH n={n} mode={name} repeats={repeats} mean_us={:.1} random_opens={} headers={} body_reads={} random_requested_bytes={} full_segment_bytes_expected={}",
                elapsed.as_secs_f64() * 1e6 / f64::from(repeats),
                io.opens,
                io.reads.iter().filter(|(_, len)| *len == 5).count(),
                io.reads.iter().filter(|(_, len)| *len != 5).count(),
                io.reads.iter().map(|(_, len)| *len).sum::<usize>(),
                if full && cold {
                    bytes * repeats as u64
                } else {
                    0
                }
            );
        }
        cleanup(&engine).await;
    }
}
