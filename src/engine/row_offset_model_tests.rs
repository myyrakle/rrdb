//! Model-based coverage for indexed reads across mutations and engine reopen.
use crate::{
    config::launch_config::LaunchConfig,
    engine::{
        DBEngine, SharedWALManager,
        ast::{dml::plan::select::scan::IndexScanPlan, types::TableName},
        index::field_to_key,
        parser::predule::{Parser, ParserContext},
        schema::row::{TableDataFieldType, TableDataRow},
        wal::{
            endec::implements::bincode::{BincodeDecoder, BincodeEncoder},
            manager::builder::WALBuilder,
        },
    },
};
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::Mutex;

async fn sql(engine: &DBEngine, wal: SharedWALManager, text: &str) {
    let mut parser = Parser::with_string(text.to_string()).unwrap();
    let statement = parser
        .parse(ParserContext::default().set_default_database("rrdb".to_string()))
        .unwrap()
        .remove(0);
    engine
        .process_query(statement, wal, "independent-model".to_string())
        .await
        .unwrap();
}
fn pair(row: TableDataRow) -> (i64, String) {
    let id = row.fields.iter().find(|f| f.column_name == "id").unwrap();
    let value = row
        .fields
        .iter()
        .find(|f| f.column_name == "payload")
        .unwrap();
    let TableDataFieldType::Integer(id) = &id.data else {
        panic!("id is not integer")
    };
    let TableDataFieldType::String(value) = &value.data else {
        panic!("payload is not string")
    };
    (*id, value.clone())
}
fn plan(eq: Option<i64>, start: Option<i64>, end: Option<i64>) -> IndexScanPlan {
    let key = |id| field_to_key(&TableDataFieldType::Integer(id));
    IndexScanPlan {
        index_name: "rrdb.items_pkey".to_string(),
        column_name: "id".to_string(),
        eq_key: eq.map(key),
        start_key: start.map(key),
        end_key: end.map(key),
    }
}

#[tokio::test]
async fn independent_offset_reads_match_model_across_mutation_and_reopen() {
    let base = std::path::PathBuf::from(format!(
        "target/independent_offset_model_{}",
        std::process::id()
    ));
    if base.exists() {
        tokio::fs::remove_dir_all(&base).await.unwrap();
    }
    let config = LaunchConfig::default_for_base_path(&base);
    tokio::fs::create_dir_all(&config.data_directory)
        .await
        .unwrap();
    tokio::fs::create_dir_all(&config.wal_directory)
        .await
        .unwrap();
    let wal = Arc::new(Mutex::new(
        WALBuilder::new(&config)
            .build(BincodeDecoder::new(), BincodeEncoder::new())
            .await
            .unwrap(),
    ));
    let mut engine = DBEngine::new(config.clone());
    sql(&engine, wal.clone(), "create database rrdb;").await;
    sql(
        &engine,
        wal.clone(),
        "create table items (id integer primary key, payload varchar(4096));",
    )
    .await;
    let table = TableName::new(Some("rrdb".to_string()), "items".to_string());
    let mut expected = BTreeMap::<i64, String>::new();
    let mut seed = 0x221_cafe_u64;
    for step in 0..96 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let id = ((seed >> 32) % 24) as i64;
        let payload = "v".repeat(1 + ((seed >> 8) % 2048) as usize);
        match (expected.contains_key(&id), step % 3) {
            (false, _) => {
                sql(
                    &engine,
                    wal.clone(),
                    &format!("insert into items (id, payload) values ({id}, '{payload}');"),
                )
                .await;
                expected.insert(id, payload);
            }
            (true, 0) => {
                sql(
                    &engine,
                    wal.clone(),
                    &format!("delete from items where id = {id};"),
                )
                .await;
                expected.remove(&id);
            }
            (true, _) => {
                sql(
                    &engine,
                    wal.clone(),
                    &format!("update items set payload = '{payload}' where id = {id};"),
                )
                .await;
                expected.insert(id, payload);
            }
        }
        // A reopened engine has neither decoded rows nor an offset directory.
        // Other iterations verify buffered/dirty rows immediately after mutation.
        if step % 4 == 0 {
            engine.flush_row_buffers_durable().await.unwrap();
            engine = DBEngine::new(config.clone());
        }
        let indexed: Vec<_> = engine
            .index_scan(table.clone(), &plan(None, None, None))
            .await
            .unwrap()
            .into_iter()
            .map(|(_, r)| pair(r))
            .collect();
        let model: Vec<_> = expected
            .iter()
            .map(|(id, value)| (*id, value.clone()))
            .collect();
        assert_eq!(indexed, model, "unbounded range at step {step}");
        let bounded: Vec<_> = engine
            .index_scan(table.clone(), &plan(None, Some(5), Some(18)))
            .await
            .unwrap()
            .into_iter()
            .map(|(_, r)| pair(r))
            .collect();
        let model_range: Vec<_> = expected
            .range(5..18)
            .map(|(id, value)| (*id, value.clone()))
            .collect();
        assert_eq!(bounded, model_range, "half-open range at step {step}");
        let point: Vec<_> = engine
            .index_scan(table.clone(), &plan(Some(id), None, None))
            .await
            .unwrap()
            .into_iter()
            .map(|(_, r)| pair(r))
            .collect();
        let model_point: Vec<_> = expected
            .get(&id)
            .map(|value| (id, value.clone()))
            .into_iter()
            .collect();
        assert_eq!(point, model_point, "point/missing at step {step}");
        let mut full: Vec<_> = engine
            .full_scan(table.clone())
            .await
            .unwrap()
            .into_iter()
            .map(|(_, r)| pair(r))
            .collect();
        full.sort_by_key(|(id, _)| *id);
        assert_eq!(full, model, "full scan oracle at step {step}");
    }
    engine.flush_row_buffers_durable().await.unwrap();
    drop(engine);
    drop(wal);
    tokio::fs::remove_dir_all(base).await.unwrap();
}
