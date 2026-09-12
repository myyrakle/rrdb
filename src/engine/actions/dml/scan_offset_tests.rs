//! Regression tests and the reproducible row-read benchmark for #221.
#[path = "scan_offset_io.rs"]
mod io;
use super::*;
use crate::config::launch_config::LaunchConfig;
use crate::engine::index::IndexMeta;
use crate::engine::row_buffer::encode_row_frames;
use crate::engine::schema::row::{TableDataField, TableDataFieldType};

fn table() -> TableName {
    TableName::new(Some("rrdb".into()), "offsets".into())
}

fn row(value: &str) -> TableDataRow {
    TableDataRow {
        fields: vec![TableDataField {
            table_name: table(),
            column_name: "value".into(),
            data: TableDataFieldType::String(value.into()),
        }],
    }
}

fn values(rows: Vec<(RowLocation, TableDataRow)>) -> Vec<(usize, String)> {
    rows.into_iter()
        .map(|(location, row)| {
            let TableDataFieldType::String(value) = row.fields[0].data.clone() else {
                panic!("expected string")
            };
            (location.row_index, value)
        })
        .collect()
}

async fn engine(label: &str) -> DBEngine {
    let base = std::env::temp_dir().join(format!("rrdb-offset-{label}-{}", uuid::Uuid::new_v4()));
    let engine = DBEngine::new(LaunchConfig::default_for_base_path(&base));
    tokio::fs::create_dir_all(engine.row_segment_path(&table()).unwrap().parent().unwrap())
        .await
        .unwrap();
    engine.indices_loaded.set(()).unwrap();
    engine
        .index_manager
        .create_index(IndexMeta::new(
            "rrdb.offset_idx".into(),
            table(),
            "value".into(),
            false,
        ))
        .await
        .unwrap();
    engine
}

fn plan(eq: Option<&str>) -> IndexScanPlan {
    IndexScanPlan {
        index_name: "rrdb.offset_idx".into(),
        column_name: "value".into(),
        eq_key: eq.map(str::to_owned),
        start_key: None,
        end_key: None,
    }
}

async fn pointer(engine: &DBEngine, key: &str, location: &str) {
    engine
        .index_manager
        .insert("rrdb.offset_idx", key.into(), location.into())
        .await
        .unwrap();
}

async fn cleanup(engine: &DBEngine) {
    // All fixtures have unique directories, including concurrent lib/bin runs.
    tokio::fs::remove_dir_all(engine.get_data_directory().parent().unwrap())
        .await
        .unwrap();
}

#[tokio::test]
async fn offset_point_and_range_skip_unrelated_invalid_body() {
    let engine = engine("skip-body").await;
    let mut bytes = encode_live_row_frames(&[row("zero")]).unwrap();
    bytes.push(ROW_FRAME_LIVE);
    bytes.extend_from_slice(&100_000u32.to_le_bytes());
    bytes.extend_from_slice(&vec![0xff; 100_000]); // framed but not valid bincode
    bytes.extend_from_slice(&encode_live_row_frames(&[row("two")]).unwrap());
    tokio::fs::write(engine.row_segment_path(&table()).unwrap(), bytes)
        .await
        .unwrap();
    pointer(&engine, "a", "2").await;
    pointer(&engine, "b", "0").await;
    let point = engine.index_scan(table(), &plan(Some("a"))).await.unwrap();
    assert_eq!(values(point), vec![(2, "two".into())]);
    let range = engine.index_scan(table(), &plan(None)).await.unwrap();
    assert_eq!(values(range), vec![(2, "two".into()), (0, "zero".into())]);
    cleanup(&engine).await;
}

#[tokio::test]
async fn offset_statistics_skip_bodies_and_count_live_slots() {
    let engine = engine("statistics").await;
    let mut bytes = encode_row_frames(&[Some(row("zero")), None]).unwrap();
    bytes.extend_from_slice(&[0, 1, 0, 0, 0, 255]); // live, invalid payload
    tokio::fs::write(engine.row_segment_path(&table()).unwrap(), bytes)
        .await
        .unwrap();
    let stats = engine.table_statistics(&table()).await.unwrap();
    assert_eq!(stats.row_count, 2);
    cleanup(&engine).await;
}

#[tokio::test]
async fn offset_mixed_pending_and_disk_preserve_order() {
    let engine = engine("pending").await;
    engine
        .append_table_rows(&table(), &[row("disk")])
        .await
        .unwrap();
    engine.flush_row_buffers().await.unwrap();
    engine
        .append_table_rows(&table(), &[row("pending")])
        .await
        .unwrap();
    pointer(&engine, "a", "1").await;
    pointer(&engine, "b", "0").await;
    let rows = engine.index_scan(table(), &plan(None)).await.unwrap();
    assert_eq!(
        values(rows),
        vec![(1, "pending".into()), (0, "disk".into())]
    );
    assert_eq!(
        tokio::fs::read(engine.row_segment_path(&table()).unwrap())
            .await
            .unwrap(),
        encode_live_row_frames(&[row("disk")]).unwrap()
    );
    cleanup(&engine).await;
}
