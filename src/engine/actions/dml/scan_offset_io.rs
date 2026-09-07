#[path = "scan_offset_integration.rs"]
mod integration;
use super::*;
use crate::common::fs::{FileSystem, FileSystemEntry, RandomAccessFile, RealFileSystem};
use crate::engine::QUERY_MEMORY_TRACKER;
use crate::engine::query_memory::{QueryMemoryTracker, QueryMemoryTrackerRef};
use crate::engine::row_buffer::RowBufferPool;
use std::sync::{Arc, Mutex as StdMutex};

#[derive(Default, Debug, Clone)]
struct IoCounts {
    opens: usize,
    reads: Vec<(u64, usize)>,
}
struct CountingFs(Arc<StdMutex<IoCounts>>);
struct CountingFile {
    inner: Box<dyn RandomAccessFile>,
    counts: Arc<StdMutex<IoCounts>>,
}
#[async_trait::async_trait]
impl RandomAccessFile for CountingFile {
    async fn file_len(&self) -> std::io::Result<u64> {
        self.inner.file_len().await
    }
    async fn read_exact_at(&mut self, offset: u64, buffer: &mut [u8]) -> std::io::Result<()> {
        self.counts
            .lock()
            .unwrap()
            .reads
            .push((offset, buffer.len()));
        self.inner.read_exact_at(offset, buffer).await
    }
}
#[async_trait::async_trait]
impl FileSystem for CountingFs {
    async fn open_random_access(&self, path: &Path) -> std::io::Result<Box<dyn RandomAccessFile>> {
        self.0.lock().unwrap().opens += 1;
        Ok(Box::new(CountingFile {
            inner: RealFileSystem.open_random_access(path).await?,
            counts: self.0.clone(),
        }))
    }
    async fn create_dir(&self, path: &str) -> std::io::Result<()> {
        RealFileSystem.create_dir(path).await
    }
    async fn write_file(&self, path: &str, content: &[u8]) -> std::io::Result<()> {
        RealFileSystem.write_file(path, content).await
    }
    async fn read_dir(&self, path: &str) -> std::io::Result<Vec<FileSystemEntry>> {
        RealFileSystem.read_dir(path).await
    }
    async fn read(&self, path: &Path) -> std::io::Result<Vec<u8>> {
        RealFileSystem.read(path).await
    }
    async fn metadata(&self, path: &Path) -> std::io::Result<u64> {
        RealFileSystem.metadata(path).await
    }
    async fn truncate(&self, path: &Path, len: u64) -> std::io::Result<()> {
        RealFileSystem.truncate(path, len).await
    }
}

fn instrument(engine: &mut DBEngine) -> Arc<StdMutex<IoCounts>> {
    let counts = Arc::new(StdMutex::new(IoCounts::default()));
    engine.file_system = Arc::new(CountingFs(counts.clone()));
    counts
}
fn take(counts: &Arc<StdMutex<IoCounts>>) -> IoCounts {
    std::mem::take(&mut *counts.lock().unwrap())
}
async fn budget<T>(limit: u64, task: impl std::future::Future<Output = T>) -> T {
    let tracker = Arc::new(QueryMemoryTracker::new(limit));
    let handle = QueryMemoryTrackerRef(Some(std::ptr::NonNull::from(tracker.as_ref())));
    QUERY_MEMORY_TRACKER.scope(handle, task).await
}

#[tokio::test]
async fn offset_io_is_headers_then_only_selected_bodies_and_incremental_appends() {
    let mut engine = engine("io").await;
    let counts = instrument(&mut engine);
    let rows: Vec<_> = (0..100)
        .map(|id| row(&format!("{id}:{}", "x".repeat(1000))))
        .collect();
    engine.append_table_rows(&table(), &rows).await.unwrap();
    engine.flush_row_buffers().await.unwrap();
    pointer(&engine, "a", "99").await;
    pointer(&engine, "b", "0").await;
    let result = engine.index_scan(table(), &plan(Some("a"))).await.unwrap();
    assert_eq!(result[0].1.fields, rows[99].fields);
    let cold = take(&counts);
    assert_eq!(cold.opens, 1);
    assert_eq!(cold.reads.iter().filter(|(_, len)| *len == 5).count(), 100);
    assert_eq!(cold.reads.len(), 101);
    assert!(
        engine
            .row_buffer_pool
            .lock()
            .await
            .cached_rows(&engine.row_segment_path(&table()).unwrap())
            .is_none()
    );
    engine.index_scan(table(), &plan(None)).await.unwrap();
    let hot = take(&counts);
    assert_eq!(hot.opens, 1);
    assert_eq!(hot.reads.len(), 2);
    assert!(hot.reads.iter().all(|(_, len)| *len > 5));
    for id in 100..120 {
        engine
            .append_table_rows(&table(), &[row(&id.to_string())])
            .await
            .unwrap();
        engine.flush_row_buffers().await.unwrap();
        pointer(&engine, &id.to_string(), &id.to_string()).await;
        assert_eq!(
            values(
                engine
                    .index_scan(table(), &plan(Some(&id.to_string())))
                    .await
                    .unwrap()
            ),
            vec![(id, id.to_string())]
        );
        let io = take(&counts);
        assert_eq!(io.opens, 1);
        assert_eq!(
            io.reads.len(),
            1,
            "append flush must not rescan existing headers"
        );
    }
    cleanup(&engine).await;
}

#[tokio::test]
async fn offset_memory_budget_precedes_directory_body_and_cached_clone_allocations() {
    let mut engine = engine("budget").await;
    let counts = instrument(&mut engine);
    engine
        .append_table_rows(&table(), &[row("selected"), row(&"x".repeat(1_000_000))])
        .await
        .unwrap();
    engine.flush_row_buffers().await.unwrap();
    pointer(&engine, "a", "0").await;
    // Refuses directory allocation while only one five-byte header has been read.
    let error = budget(100, engine.index_scan(table(), &plan(Some("a"))))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("memory limit exceeded"));
    assert!(take(&counts).reads.iter().all(|(_, len)| *len == 5));
    assert_eq!(
        values(
            budget(10_000, engine.index_scan(table(), &plan(Some("a"))))
                .await
                .unwrap()
        ),
        vec![(0, "selected".into())]
    );
    take(&counts);
    let error = budget(100, engine.index_scan(table(), &plan(Some("a"))))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("memory limit exceeded"));
    assert!(
        take(&counts).reads.is_empty(),
        "reserve body budget before I/O/allocation"
    );
    engine.full_scan(table()).await.unwrap();
    assert_eq!(
        values(
            budget(1000, engine.index_scan(table(), &plan(Some("a"))))
                .await
                .unwrap()
        ),
        vec![(0, "selected".into())]
    );
    assert!(
        take(&counts).reads.is_empty(),
        "clone only selected cached row, no disk access"
    );
    cleanup(&engine).await;
}

#[tokio::test]
async fn offset_rewrite_growth_shrink_delete_append_and_reopen() {
    let engine = engine("rewrite").await;
    engine
        .append_table_rows(&table(), &[row("zero"), row("one"), row("two")])
        .await
        .unwrap();
    engine.flush_row_buffers().await.unwrap();
    for id in 0..3 {
        pointer(&engine, &id.to_string(), &id.to_string()).await;
    }
    engine.index_scan(table(), &plan(None)).await.unwrap();
    for text in ["x".repeat(20_000), "s".into()] {
        engine
            .update_table_rows(&table(), HashMap::from([(0, row(&text))]))
            .await
            .unwrap();
        // Dirty rows beat old on-disk offsets, with no forced flush.
        assert_eq!(
            values(engine.index_scan(table(), &plan(Some("0"))).await.unwrap()),
            vec![(0, text.clone())]
        );
        engine.flush_row_buffers().await.unwrap();
        assert!(
            engine
                .row_buffer_pool
                .lock()
                .await
                .directory(&engine.row_segment_path(&table()).unwrap())
                .is_none()
        );
        *engine.row_buffer_pool.lock().await = RowBufferPool::default();
        assert_eq!(
            values(engine.index_scan(table(), &plan(Some("2"))).await.unwrap()),
            vec![(2, "two".into())]
        );
    }
    engine
        .delete_table_rows(&table(), HashSet::from([1]))
        .await
        .unwrap();
    assert!(
        engine
            .index_scan(table(), &plan(Some("1")))
            .await
            .unwrap_err()
            .to_string()
            .contains("out of sync")
    );
    engine.flush_row_buffers().await.unwrap();
    assert_eq!(
        engine
            .append_table_rows(&table(), &[row("three")])
            .await
            .unwrap(),
        3
    );
    pointer(&engine, "3", "3").await;
    engine.flush_row_buffers_durable().await.unwrap();
    let reopened = DBEngine::new((*engine.config).clone());
    assert_eq!(
        values(
            reopened
                .index_scan(table(), &plan(Some("3")))
                .await
                .unwrap()
        ),
        vec![(3, "three".into())]
    );
    assert!(
        reopened
            .index_scan(table(), &plan(Some("1")))
            .await
            .unwrap_err()
            .to_string()
            .contains("out of sync")
    );
    assert_eq!(
        reopened.table_statistics(&table()).await.unwrap().row_count,
        3
    );
    cleanup(&engine).await;
}

#[tokio::test]
async fn offset_bad_frames_and_stale_pointers_fail_without_panics() {
    let mut engine = engine("malformed").await;
    let counts = instrument(&mut engine);
    pointer(&engine, "a", "0").await;
    let path = engine.row_segment_path(&table()).unwrap();
    for bytes in [
        vec![0],
        vec![0, 0xff, 0xff, 0xff, 0xff],
        vec![0, 1, 0, 0, 0, 0xff],
    ] {
        *engine.row_buffer_pool.lock().await = RowBufferPool::default();
        tokio::fs::write(&path, bytes).await.unwrap();
        assert!(engine.index_scan(table(), &plan(Some("a"))).await.is_err());
        assert!(take(&counts).reads.iter().all(|(_, len)| *len <= 5));
    }
    for marker in [1, 255] {
        // preserve historical nonzero tombstone semantics
        *engine.row_buffer_pool.lock().await = RowBufferPool::default();
        tokio::fs::write(&path, [marker, 0, 0, 0, 0]).await.unwrap();
        assert!(
            engine
                .index_scan(table(), &plan(Some("a")))
                .await
                .unwrap_err()
                .to_string()
                .contains("out of sync")
        );
    }
    pointer(&engine, "invalid", "not-an-index").await;
    pointer(&engine, "missing", "99999").await;
    assert!(
        engine
            .index_scan(table(), &plan(Some("invalid")))
            .await
            .unwrap_err()
            .to_string()
            .contains("invalid row path")
    );
    assert!(
        engine
            .index_scan(table(), &plan(Some("missing")))
            .await
            .unwrap_err()
            .to_string()
            .contains("out of sync")
    );
    assert!(
        engine
            .index_scan(table(), &plan(Some("empty")))
            .await
            .unwrap()
            .is_empty()
    );
    cleanup(&engine).await;
}

#[tokio::test]
async fn offset_drop_recreate_discards_all_row_state_and_not_prefix_siblings() {
    use crate::engine::ast::ddl::drop_database::DropDatabaseQuery;
    use crate::engine::ast::ddl::drop_table::DropTableQuery;
    for database in [false, true] {
        let engine = engine("drop").await;
        engine
            .append_table_rows(&table(), &[row("old")])
            .await
            .unwrap();
        engine.flush_row_buffers().await.unwrap();
        pointer(&engine, "a", "0").await;
        engine.index_scan(table(), &plan(Some("a"))).await.unwrap();
        engine
            .append_table_rows(&table(), &[row("unflushed")])
            .await
            .unwrap();
        let sibling = if database {
            TableName::new(Some("rrdb_other".into()), "offsets".into())
        } else {
            TableName::new(Some("rrdb".into()), "offsets_other".into())
        };
        engine
            .append_table_rows(&sibling, &[row("sibling")])
            .await
            .unwrap();
        if database {
            engine
                .drop_database(DropDatabaseQuery::builder().set_name("rrdb".into()))
                .await
                .unwrap();
        } else {
            engine
                .drop_table(DropTableQuery::builder().set_table(table()))
                .await
                .unwrap();
        }
        engine.flush_row_buffers_durable().await.unwrap();
        assert!(
            !engine.row_segment_path(&table()).unwrap().exists(),
            "dropped buffer resurrected namespace"
        );
        assert_eq!(engine.full_scan(sibling).await.unwrap().len(), 1);
        assert_eq!(
            engine
                .append_table_rows(&table(), &[row("new")])
                .await
                .unwrap(),
            0
        );
        engine.flush_row_buffers().await.unwrap();
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
        pointer(&engine, "a", "0").await;
        // Equal serialized length deliberately defeats length-only generation checks.
        assert_eq!(
            values(engine.index_scan(table(), &plan(Some("a"))).await.unwrap()),
            vec![(0, "new".into())]
        );
        cleanup(&engine).await;
    }
}

#[tokio::test]
async fn offset_failed_rewrite_does_not_leave_stale_directory() {
    let engine = engine("failed-rewrite").await;
    engine
        .append_table_rows(&table(), &[row("old"), row("other")])
        .await
        .unwrap();
    engine.flush_row_buffers().await.unwrap();
    pointer(&engine, "a", "1").await;
    engine.index_scan(table(), &plan(Some("a"))).await.unwrap();
    engine
        .update_table_rows(&table(), HashMap::from([(0, row(&"grown".repeat(1000)))]))
        .await
        .unwrap();
    // Force metadata publication failure AFTER data replacement, without new production hooks.
    let meta_temp = engine
        .row_segment_meta_path(&table())
        .unwrap()
        .with_extension("bin.tmp");
    tokio::fs::create_dir(&meta_temp).await.unwrap();
    assert!(engine.flush_row_buffers().await.is_err());
    assert!(
        engine
            .row_buffer_pool
            .lock()
            .await
            .directory(&engine.row_segment_path(&table()).unwrap())
            .is_none()
    );
    assert_eq!(
        values(engine.index_scan(table(), &plan(Some("a"))).await.unwrap()),
        vec![(1, "other".into())]
    );
    tokio::fs::remove_dir(&meta_temp).await.unwrap();
    engine.flush_row_buffers().await.unwrap();
    *engine.row_buffer_pool.lock().await = RowBufferPool::default();
    assert_eq!(
        values(engine.index_scan(table(), &plan(Some("a"))).await.unwrap()),
        vec![(1, "other".into())]
    );
    cleanup(&engine).await;
}
