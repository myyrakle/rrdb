use std::collections::HashMap;
use std::path::PathBuf;

use tokio::sync::{Mutex, RwLock};

use crate::errors;
use crate::errors::execute_error::ExecuteError;

use super::page_btree::PageBackedBTreeIndex;
use super::{IndexEntry, IndexMeta};

/// IndexManager coordinates page-backed B+tree indices with disk persistence.
///
/// Design (issue #230, superseding the full-snapshot-rewrite design from
/// issue #160):
/// 1. Each index is a `PageBackedBTreeIndex` (see `page_btree.rs`): a real
///    disk-facing B+tree where every mutation reads/writes only the pages
///    it touches, instead of serializing the whole index on every insert.
/// 2. A small `<index_name>.meta` sidecar file holds the `IndexMeta`
///    (index/table/column names, uniqueness) next to the page-backed
///    `<index_name>.idx` data file. The meta file is written once at
///    `create_index` time and never rewritten.
/// 3. `load_all`/`load_database_indices` rediscover indices by scanning for
///    `.meta` files and opening their sibling `.idx` page file -- this is an
///    on-disk format change from the old full-blob `.idx` files (pre-#230),
///    which is acceptable since this feature has not shipped yet. Old-style
///    `.idx` files without a `.meta` sidecar are simply not picked up by
///    `load_all` (no panic, no crash -- see its doc comment).
pub struct IndexManager {
    /// Map: index_name -> page-backed B+tree
    indices: RwLock<HashMap<String, PageBackedBTreeIndex>>,
    /// Map: index_name -> IndexMeta
    metas: RwLock<HashMap<String, IndexMeta>>,
    /// Base directory for all indices (usually data_directory)
    base_directory: PathBuf,
    /// Serializes create_index calls to prevent TOCTOU races
    create_lock: Mutex<()>,
}

impl IndexManager {
    pub fn new(base_directory: PathBuf) -> Self {
        Self {
            indices: RwLock::new(HashMap::new()),
            metas: RwLock::new(HashMap::new()),
            base_directory,
            create_lock: Mutex::new(()),
        }
    }

    /// Compute the on-disk path for an index's page-backed data file.
    /// Structure: <base>/<database>/tables/<table>/index/<index_name>.idx
    fn index_file_path(&self, meta: &IndexMeta) -> PathBuf {
        let database_name = meta
            .table_name
            .database_name
            .clone()
            .unwrap_or_else(|| "rrdb".to_string());
        let table_name = &meta.table_name.table_name;

        self.base_directory
            .join(database_name)
            .join("tables")
            .join(table_name)
            .join("index")
            .join(format!("{}.idx", meta.index_name))
    }

    /// Compute the on-disk path for an index's small metadata sidecar file.
    fn meta_file_path(&self, meta: &IndexMeta) -> PathBuf {
        self.index_file_path(meta).with_extension("meta")
    }

    /// Write the (tiny, fixed-at-creation) IndexMeta sidecar file.
    async fn write_meta_file(&self, meta: &IndexMeta) -> errors::Result<()> {
        let path = self.meta_file_path(meta);
        let encoded = bincode::serialize(meta)
            .map_err(|e| ExecuteError::wrap(format!("failed to encode index meta: {}", e)))?;
        tokio::fs::write(&path, encoded)
            .await
            .map_err(|e| ExecuteError::wrap(format!("failed to write index meta file: {}", e)))
    }

    /// Register a new index: create an empty in-memory B-tree and persist
    /// the metadata to disk.
    ///
    /// Holds an exclusive `create_lock` for the entire flow to prevent
    /// concurrent create_index calls from racing past the existence check.
    pub async fn create_index(&self, meta: IndexMeta) -> errors::Result<()> {
        let _guard = self.create_lock.lock().await;
        let index_name = meta.index_name.clone();

        // Check if already exists (under the exclusive lock)
        {
            let metas = self.metas.read().await;
            if metas.contains_key(&index_name) {
                return Err(ExecuteError::wrap(format!(
                    "index '{}' already exists",
                    index_name
                )));
            }
        }

        let file_path = self.index_file_path(&meta);

        // Create parent directory if needed
        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                ExecuteError::wrap(format!("failed to create index directory: {}", e))
            })?;
        }

        // Persist the meta sidecar, then create the (empty) page-backed
        // data file.
        self.write_meta_file(&meta).await?;
        let tree =
            PageBackedBTreeIndex::create(&file_path, meta.column_name.clone(), meta.is_unique)
                .await?;

        // Update in-memory state
        {
            let mut indices = self.indices.write().await;
            let mut metas = self.metas.write().await;
            indices.insert(index_name.clone(), tree);
            metas.insert(index_name, meta);
        }

        Ok(())
    }

    /// Drop an index: remove from memory and delete both disk files (the
    /// page-backed data file and its meta sidecar).
    pub async fn drop_index(&self, index_name: &str) -> errors::Result<()> {
        let meta = {
            let metas = self.metas.read().await;
            metas.get(index_name).cloned()
        };

        let meta = match meta {
            Some(m) => m,
            None => {
                return Err(ExecuteError::wrap(format!(
                    "index '{}' not found",
                    index_name
                )));
            }
        };

        let file_path = self.index_file_path(&meta);
        let meta_path = self.meta_file_path(&meta);

        // Remove from disk
        if file_path.exists() {
            tokio::fs::remove_file(&file_path)
                .await
                .map_err(|e| ExecuteError::wrap(format!("failed to remove index file: {}", e)))?;
        }
        if meta_path.exists() {
            tokio::fs::remove_file(&meta_path).await.map_err(|e| {
                ExecuteError::wrap(format!("failed to remove index meta file: {}", e))
            })?;
        }

        // Remove from memory
        {
            let mut indices = self.indices.write().await;
            let mut metas = self.metas.write().await;
            indices.remove(index_name);
            metas.remove(index_name);
        }

        Ok(())
    }

    /// Insert a key->row_path into an index. This reads/writes only the
    /// handful of pages the insert touches (root, target leaf, and any
    /// split siblings) -- not the whole index file (issue #230).
    ///
    /// The write lock on the index map is held for the whole call, which
    /// serializes mutations to a given index the same way the previous
    /// (in-memory-then-disk) design did; see `page_btree.rs` for why a
    /// single page-backed tree isn't safe under fully concurrent mutation.
    pub async fn insert(
        &self,
        index_name: &str,
        key: String,
        row_path: String,
    ) -> errors::Result<()> {
        let indices = self.indices.write().await;
        let tree = indices
            .get(index_name)
            .ok_or_else(|| ExecuteError::wrap(format!("index '{}' not found", index_name)))?;
        tree.insert(key, row_path).await
    }

    /// Remove a key->row_path from an index, touching only the owning leaf
    /// (and its overflow chain, if any).
    pub async fn remove(
        &self,
        index_name: &str,
        key: &str,
        row_path: &str,
    ) -> errors::Result<bool> {
        let indices = self.indices.write().await;
        let tree = indices
            .get(index_name)
            .ok_or_else(|| ExecuteError::wrap(format!("index '{}' not found", index_name)))?;
        tree.remove(key, row_path).await
    }

    /// Update a key for a given row path in an index (remove old mapping,
    /// insert new one; see `PageBackedBTreeIndex::update`).
    pub async fn update(
        &self,
        index_name: &str,
        old_key: &str,
        new_key: String,
        row_path: String,
    ) -> errors::Result<()> {
        let indices = self.indices.write().await;
        let tree = indices
            .get(index_name)
            .ok_or_else(|| ExecuteError::wrap(format!("index '{}' not found", index_name)))?;
        tree.update(old_key, new_key, row_path).await
    }

    /// Look up row paths for an exact key match.
    pub async fn get(&self, index_name: &str, key: &str) -> errors::Result<Vec<String>> {
        let indices = self.indices.read().await;
        let tree = indices
            .get(index_name)
            .ok_or_else(|| ExecuteError::wrap(format!("index '{}' not found", index_name)))?;
        tree.get(key).await
    }

    /// Point lookup for unique index.
    pub async fn get_one(&self, index_name: &str, key: &str) -> errors::Result<Option<String>> {
        let indices = self.indices.read().await;
        let tree = indices
            .get(index_name)
            .ok_or_else(|| ExecuteError::wrap(format!("index '{}' not found", index_name)))?;
        tree.get_one(key).await
    }

    /// Range scan on an index.
    pub async fn range(
        &self,
        index_name: &str,
        start: Option<&str>,
        end: Option<&str>,
    ) -> errors::Result<Vec<IndexEntry>> {
        let indices = self.indices.read().await;
        let tree = indices
            .get(index_name)
            .ok_or_else(|| ExecuteError::wrap(format!("index '{}' not found", index_name)))?;
        tree.range(start, end).await
    }

    /// Full scan on an index.
    pub async fn scan_all(&self, index_name: &str) -> errors::Result<Vec<IndexEntry>> {
        let indices = self.indices.read().await;
        let tree = indices
            .get(index_name)
            .ok_or_else(|| ExecuteError::wrap(format!("index '{}' not found", index_name)))?;
        tree.scan_all().await
    }

    /// List all index names.
    pub async fn list_indices(&self) -> Vec<String> {
        let metas = self.metas.read().await;
        metas.keys().cloned().collect()
    }

    /// 특정 테이블에 속한 인덱스 메타 목록을 반환합니다.
    pub async fn indices_for_table(
        &self,
        table_name: &crate::engine::ast::types::TableName,
    ) -> Vec<IndexMeta> {
        let metas = self.metas.read().await;
        metas
            .values()
            .filter(|meta| meta.table_name == *table_name)
            .cloned()
            .collect()
    }

    /// 인덱스의 고유 키 개수를 반환합니다 (통계용).
    pub async fn distinct_keys(&self, index_name: &str) -> errors::Result<usize> {
        let indices = self.indices.read().await;
        let tree = indices
            .get(index_name)
            .ok_or_else(|| ExecuteError::wrap(format!("index '{}' not found", index_name)))?;
        tree.distinct_keys().await
    }

    /// 인덱스 전체 엔트리를 교체합니다.
    /// CREATE INDEX 백필, DELETE 압축 후 재구축 등에 사용합니다.
    ///
    /// Not a hot path (called once per CREATE INDEX backfill or compaction,
    /// not per row), so rebuilding by deleting and recreating the
    /// page-backed file, then inserting each entry, is acceptable even
    /// though it is O(n log n) rather than a bulk page-loader.
    pub async fn replace_entries(
        &self,
        index_name: &str,
        entries: Vec<IndexEntry>,
    ) -> errors::Result<()> {
        let meta = {
            let metas = self.metas.read().await;
            metas
                .get(index_name)
                .cloned()
                .ok_or_else(|| ExecuteError::wrap(format!("index '{}' not found", index_name)))?
        };

        let file_path = self.index_file_path(&meta);
        if file_path.exists() {
            tokio::fs::remove_file(&file_path)
                .await
                .map_err(|e| ExecuteError::wrap(format!("failed to remove index file: {}", e)))?;
        }

        let tree =
            PageBackedBTreeIndex::create(&file_path, meta.column_name.clone(), meta.is_unique)
                .await?;
        for entry in entries {
            tree.insert(entry.key, entry.row_path).await?;
        }

        let mut indices = self.indices.write().await;
        indices.insert(index_name.to_string(), tree);

        Ok(())
    }

    /// 테이블에 속한 인덱스의 메모리 상태를 제거합니다.
    /// 디스크 파일은 테이블 디렉토리와 함께 삭제되는 경우(DROP TABLE)에 사용합니다.
    pub async fn remove_table_indices(&self, table_name: &crate::engine::ast::types::TableName) {
        let names: Vec<String> = {
            let metas = self.metas.read().await;
            metas
                .values()
                .filter(|meta| meta.table_name == *table_name)
                .map(|meta| meta.index_name.clone())
                .collect()
        };

        let mut indices = self.indices.write().await;
        let mut metas = self.metas.write().await;
        for name in names {
            indices.remove(&name);
            metas.remove(&name);
        }
    }

    /// 데이터베이스에 속한 인덱스의 메모리 상태를 제거합니다 (DROP DATABASE).
    pub async fn remove_database_indices(&self, database_name: &str) {
        let names: Vec<String> = {
            let metas = self.metas.read().await;
            metas
                .values()
                .filter(|meta| meta.table_name.database_name.as_deref() == Some(database_name))
                .map(|meta| meta.index_name.clone())
                .collect()
        };

        let mut indices = self.indices.write().await;
        let mut metas = self.metas.write().await;
        for name in names {
            indices.remove(&name);
            metas.remove(&name);
        }
    }

    /// Get index meta.
    pub async fn get_meta(&self, index_name: &str) -> Option<IndexMeta> {
        let metas = self.metas.read().await;
        metas.get(index_name).cloned()
    }

    /// Get the number of entries in an index.
    pub async fn len(&self, index_name: &str) -> errors::Result<usize> {
        let indices = self.indices.read().await;
        let tree = indices
            .get(index_name)
            .ok_or_else(|| ExecuteError::wrap(format!("index '{}' not found", index_name)))?;
        tree.len().await
    }

    /// Load all indices from disk into memory. Called on server startup to
    /// restore index state.
    ///
    /// Discovers indices by scanning for `.meta` sidecar files and opening
    /// their sibling page-backed `.idx` file (see the struct doc comment).
    /// Pre-#230 `.idx` files that predate this sidecar (a single bincode
    /// blob with no `.meta` file next to it) are silently skipped rather
    /// than erroring, since this on-disk format has not shipped yet.
    pub async fn load_all(&self, index_directory: &PathBuf) -> errors::Result<()> {
        if !index_directory.exists() {
            return Ok(());
        }

        let mut dir_entries = tokio::fs::read_dir(index_directory)
            .await
            .map_err(|e| ExecuteError::wrap(format!("failed to read index directory: {}", e)))?;

        loop {
            match dir_entries.next_entry().await {
                Ok(Some(entry)) => {
                    let path = entry.path();

                    if !path.is_file() {
                        continue;
                    }

                    match path.extension().and_then(|e| e.to_str()) {
                        Some("meta") => {
                            let data = tokio::fs::read(&path).await.map_err(|e| {
                                ExecuteError::wrap(format!(
                                    "failed to read index meta file {:?}: {}",
                                    path, e
                                ))
                            })?;

                            let meta: IndexMeta = bincode::deserialize(&data).map_err(|e| {
                                ExecuteError::wrap(format!(
                                    "failed to decode index meta file {:?}: {}",
                                    path, e
                                ))
                            })?;

                            let idx_path = path.with_extension("idx");
                            let tree = PageBackedBTreeIndex::open(
                                &idx_path,
                                meta.column_name.clone(),
                                meta.is_unique,
                            )
                            .await?;

                            let index_name = meta.index_name.clone();

                            let mut indices = self.indices.write().await;
                            let mut metas = self.metas.write().await;
                            indices.insert(index_name.clone(), tree);
                            metas.insert(index_name, meta);
                        }
                        _ => continue,
                    }
                }
                Ok(None) => break, // end of directory
                Err(e) => {
                    return Err(ExecuteError::wrap(format!(
                        "failed to read index directory entry: {}",
                        e
                    )));
                }
            }
        }

        Ok(())
    }

    /// Load all indices for a given database by recursively scanning all
    /// table index directories. Called on startup.
    pub async fn load_database_indices(&self, database_name: &str) -> errors::Result<()> {
        let db_path = self.base_directory.join(database_name).join("tables");

        if !db_path.exists() {
            return Ok(());
        }

        let mut table_entries = tokio::fs::read_dir(&db_path)
            .await
            .map_err(|e| ExecuteError::wrap(format!("failed to read tables dir: {}", e)))?;

        loop {
            match table_entries.next_entry().await {
                Ok(Some(table_entry)) => {
                    let table_path = table_entry.path();
                    if !table_path.is_dir() {
                        continue;
                    }

                    let index_dir = table_path.join("index");
                    if index_dir.exists() {
                        self.load_all(&index_dir).await?;
                    }
                }
                Ok(None) => break, // end of directory
                Err(e) => {
                    return Err(ExecuteError::wrap(format!(
                        "failed to read table directory entry: {}",
                        e
                    )));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::ast::types::TableName;

    async fn setup_temp_dir(test_name: &str) -> PathBuf {
        let dir = PathBuf::from(format!("target/test_index_data/{}", test_name));
        if dir.exists() {
            tokio::fs::remove_dir_all(&dir).await.unwrap();
        }
        tokio::fs::create_dir_all(&dir).await.unwrap();
        dir
    }

    fn make_meta(name: &str, column: &str, unique: bool) -> IndexMeta {
        IndexMeta::new(
            name.to_string(),
            TableName {
                database_name: Some("testdb".to_string()),
                table_name: "testtable".to_string(),
            },
            column.to_string(),
            unique,
        )
    }

    #[tokio::test]
    async fn test_create_and_query_index() {
        let dir = setup_temp_dir("create_and_query").await;
        let manager = IndexManager::new(dir);

        let meta = make_meta("idx_name", "name", false);
        manager.create_index(meta).await.unwrap();

        manager
            .insert("idx_name", "S:alice".into(), "/r/1".into())
            .await
            .unwrap();
        manager
            .insert("idx_name", "S:bob".into(), "/r/2".into())
            .await
            .unwrap();
        manager
            .insert("idx_name", "S:alice".into(), "/r/3".into())
            .await
            .unwrap();

        let results = manager.get("idx_name", "S:alice").await.unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.contains(&"/r/1".to_string()));
        assert!(results.contains(&"/r/3".to_string()));
    }

    #[tokio::test]
    async fn test_unique_index_enforcement() {
        let dir = setup_temp_dir("unique_enforcement").await;
        let manager = IndexManager::new(dir);

        let meta = make_meta("idx_id", "id", true);
        manager.create_index(meta).await.unwrap();

        manager
            .insert("idx_id", "I:001".into(), "/r/1".into())
            .await
            .unwrap();

        let result = manager
            .insert("idx_id", "I:001".into(), "/r/2".into())
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_persist_and_reload() {
        let dir = setup_temp_dir("persist_reload").await;

        // Create and populate
        {
            let manager = IndexManager::new(dir.clone());
            let meta = make_meta("idx_age", "age", false);
            manager.create_index(meta).await.unwrap();

            manager
                .insert("idx_age", "I:030".into(), "/r/1".into())
                .await
                .unwrap();
            manager
                .insert("idx_age", "I:025".into(), "/r/2".into())
                .await
                .unwrap();
            manager
                .insert("idx_age", "I:035".into(), "/r/3".into())
                .await
                .unwrap();

            assert_eq!(manager.len("idx_age").await.unwrap(), 3);
        }

        // Reload from disk
        {
            let manager = IndexManager::new(dir.clone());
            manager.load_database_indices("testdb").await.unwrap();

            assert_eq!(manager.len("idx_age").await.unwrap(), 3);

            let results = manager.get("idx_age", "I:030").await.unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0], "/r/1");

            // Verify sorted order on scan
            let entries = manager.scan_all("idx_age").await.unwrap();
            assert_eq!(entries.len(), 3);
            assert_eq!(entries[0].key, "I:025");
            assert_eq!(entries[1].key, "I:030");
            assert_eq!(entries[2].key, "I:035");
        }
    }

    #[tokio::test]
    async fn test_remove_from_index() {
        let dir = setup_temp_dir("remove").await;
        let manager = IndexManager::new(dir);

        let meta = make_meta("idx_name", "name", false);
        manager.create_index(meta).await.unwrap();

        manager
            .insert("idx_name", "S:alice".into(), "/r/1".into())
            .await
            .unwrap();
        manager
            .insert("idx_name", "S:alice".into(), "/r/2".into())
            .await
            .unwrap();

        // Remove one entry
        let removed = manager.remove("idx_name", "S:alice", "/r/1").await.unwrap();
        assert!(removed);

        let results = manager.get("idx_name", "S:alice").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], "/r/2");

        // Remove the other
        let removed = manager.remove("idx_name", "S:alice", "/r/2").await.unwrap();
        assert!(removed);

        assert_eq!(manager.len("idx_name").await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_update_index() {
        let dir = setup_temp_dir("update").await;
        let manager = IndexManager::new(dir);

        let meta = make_meta("idx_name", "name", true);
        manager.create_index(meta).await.unwrap();

        manager
            .insert("idx_name", "S:old".into(), "/r/1".into())
            .await
            .unwrap();

        manager
            .update("idx_name", "S:old", "S:new".into(), "/r/1".into())
            .await
            .unwrap();

        assert!(manager.get("idx_name", "S:old").await.unwrap().is_empty());
        assert_eq!(
            manager.get_one("idx_name", "S:new").await.unwrap(),
            Some("/r/1".to_string())
        );
    }

    #[tokio::test]
    async fn test_drop_index() {
        let dir = setup_temp_dir("drop").await;
        let manager = IndexManager::new(dir);

        let meta = make_meta("idx_name", "name", false);
        manager.create_index(meta).await.unwrap();

        manager
            .insert("idx_name", "S:alice".into(), "/r/1".into())
            .await
            .unwrap();

        manager.drop_index("idx_name").await.unwrap();

        // Should be gone
        assert!(manager.get_meta("idx_name").await.is_none());
        assert!(manager.list_indices().await.is_empty());
    }

    #[tokio::test]
    async fn test_range_query() {
        let dir = setup_temp_dir("range").await;
        let manager = IndexManager::new(dir);

        let meta = make_meta("idx_age", "age", false);
        manager.create_index(meta).await.unwrap();

        for i in 10..=50 {
            let key = format!("I:{:020}", i);
            let path = format!("/r/{}", i);
            manager.insert("idx_age", key, path).await.unwrap();
        }

        // Range [20, 30)
        let results = manager
            .range(
                "idx_age",
                Some("I:00000000000000000020"),
                Some("I:00000000000000000030"),
            )
            .await
            .unwrap();
        assert_eq!(results.len(), 10);
    }

    #[tokio::test]
    async fn test_create_duplicate_index_fails() {
        let dir = setup_temp_dir("duplicate").await;
        let manager = IndexManager::new(dir);

        let meta = make_meta("idx_dup", "col", false);
        manager.create_index(meta).await.unwrap();

        let meta2 = make_meta("idx_dup", "col", false);
        let result = manager.create_index(meta2).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_load_empty_directory() {
        let dir = setup_temp_dir("empty_dir").await;
        let manager = IndexManager::new(dir.clone());

        // Should not error on non-existent directory
        let result = manager.load_database_indices("nonexistent").await;
        assert!(result.is_ok());

        // Should not error on empty directory
        let empty_dir = dir.join("empty");
        tokio::fs::create_dir_all(&empty_dir).await.unwrap();
        let result = manager.load_all(&empty_dir).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_multiple_indices() {
        let dir = setup_temp_dir("multiple").await;
        let manager = IndexManager::new(dir);

        let meta1 = make_meta("idx_name", "name", false);
        manager.create_index(meta1).await.unwrap();

        let meta2 = make_meta("idx_age", "age", false);
        manager.create_index(meta2).await.unwrap();

        let meta3 = make_meta("idx_id", "id", true);
        manager.create_index(meta3).await.unwrap();

        let indices = manager.list_indices().await;
        assert_eq!(indices.len(), 3);

        manager
            .insert("idx_name", "S:alice".into(), "/r/1".into())
            .await
            .unwrap();
        manager
            .insert("idx_age", "I:030".into(), "/r/1".into())
            .await
            .unwrap();
        manager
            .insert("idx_id", "I:001".into(), "/r/1".into())
            .await
            .unwrap();

        // Reload from disk
        let manager2 = IndexManager::new(manager.base_directory.clone());
        manager2.load_database_indices("testdb").await.unwrap();

        assert_eq!(manager2.list_indices().await.len(), 3);
        assert_eq!(manager2.len("idx_name").await.unwrap(), 1);
        assert_eq!(manager2.len("idx_age").await.unwrap(), 1);
        assert_eq!(manager2.len("idx_id").await.unwrap(), 1);
    }
}
