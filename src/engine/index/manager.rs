use std::collections::HashMap;
use std::path::PathBuf;

use tokio::sync::{Mutex, RwLock};

use crate::engine::encoder::schema_encoder::StorageEncoder;
use crate::errors;
use crate::errors::execute_error::ExecuteError;

use super::btree::BTreeIndex;
use super::{IndexEntry, IndexMeta};

/// IndexFileManager manages a single index file on disk.
/// Each index is persisted as a BSON-encoded file containing:
/// - IndexMeta (index name, table, column, uniqueness)
/// - Vec<IndexEntry> (the actual key->row_path mappings)
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
struct IndexFile {
    meta: IndexMeta,
    entries: Vec<IndexEntry>,
}

/// IndexManager coordinates in-memory B-tree indices with disk persistence.
///
/// Design (issue #160):
/// 1. B-tree indices are loaded into memory on startup (`load_all`)
/// 2. On every mutation (insert/remove/update), memory is updated first,
///    then the change is flushed to disk (`save_index`)
/// 3. Disk is the backup; memory is the primary query path
/// 4. Each index file lives at `<table_path>/index/<index_name>.idx`
pub struct IndexManager {
    /// Map: index_name -> in-memory B-tree
    indices: RwLock<HashMap<String, BTreeIndex>>,
    /// Map: index_name -> IndexMeta
    metas: RwLock<HashMap<String, IndexMeta>>,
    /// Base directory for all indices (usually data_directory)
    base_directory: PathBuf,
    encoder: StorageEncoder,
    /// Serializes create_index calls to prevent TOCTOU races
    create_lock: Mutex<()>,
}

impl IndexManager {
    pub fn new(base_directory: PathBuf) -> Self {
        Self {
            indices: RwLock::new(HashMap::new()),
            metas: RwLock::new(HashMap::new()),
            base_directory,
            encoder: StorageEncoder::new(),
            create_lock: Mutex::new(()),
        }
    }

    /// Compute the on-disk path for an index file.
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

        // Create empty in-memory tree
        let tree = BTreeIndex::new(meta.column_name.clone(), meta.is_unique);

        // Persist to disk
        let file_data = IndexFile {
            meta: meta.clone(),
            entries: Vec::new(),
        };
        let encoded = self.encoder.encode(&file_data);
        tokio::fs::write(&file_path, encoded)
            .await
            .map_err(|e| ExecuteError::wrap(format!("failed to write index file: {}", e)))?;

        // Update in-memory state
        {
            let mut indices = self.indices.write().await;
            let mut metas = self.metas.write().await;
            indices.insert(index_name.clone(), tree);
            metas.insert(index_name, meta);
        }

        Ok(())
    }

    /// Drop an index: remove from memory and delete the disk file.
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

        // Remove from disk
        if file_path.exists() {
            tokio::fs::remove_file(&file_path)
                .await
                .map_err(|e| ExecuteError::wrap(format!("failed to remove index file: {}", e)))?;
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

    /// Insert a key->row_path into an index. Updates memory and disk.
    ///
    /// The write lock is held for the in-memory mutation. Then the lock is
    /// released and a snapshot is written to disk. If save_index fails, the
    /// in-memory change is reverted.
    pub async fn insert(
        &self,
        index_name: &str,
        key: String,
        row_path: String,
    ) -> errors::Result<()> {
        let (meta, entries) = {
            let mut indices = self.indices.write().await;
            let tree = indices
                .get_mut(index_name)
                .ok_or_else(|| ExecuteError::wrap(format!("index '{}' not found", index_name)))?;

            tree.insert(key.clone(), row_path.clone())
                .map_err(ExecuteError::wrap)?;

            // Snapshot while we hold the lock
            let metas = self.metas.read().await;
            let meta = metas.get(index_name).ok_or_else(|| {
                ExecuteError::wrap(format!("index meta '{}' not found", index_name))
            })?;
            (meta.clone(), tree.to_entries())
        };

        // Disk I/O outside the lock -- avoids deadlock
        if let Err(e) = self.write_index_file(&meta, &entries).await {
            // Revert the in-memory change
            let mut indices = self.indices.write().await;
            if let Some(tree) = indices.get_mut(index_name) {
                tree.remove(&key, &row_path);
            }
            return Err(e);
        }

        Ok(())
    }

    /// Remove a key->row_path from an index. Updates memory and disk.
    ///
    /// Lock is released before disk I/O. If save_index fails, the in-memory
    /// change is reverted.
    pub async fn remove(
        &self,
        index_name: &str,
        key: &str,
        row_path: &str,
    ) -> errors::Result<bool> {
        let (removed, meta, entries) = {
            let mut indices = self.indices.write().await;
            let tree = indices
                .get_mut(index_name)
                .ok_or_else(|| ExecuteError::wrap(format!("index '{}' not found", index_name)))?;

            let removed = tree.remove(key, row_path);

            if removed {
                let metas = self.metas.read().await;
                let meta = metas.get(index_name).ok_or_else(|| {
                    ExecuteError::wrap(format!("index meta '{}' not found", index_name))
                })?;
                (removed, meta.clone(), tree.to_entries())
            } else {
                return Ok(false);
            }
        };

        // Disk I/O outside the lock
        if let Err(e) = self.write_index_file(&meta, &entries).await {
            // Re-insert on failure
            let mut indices = self.indices.write().await;
            if let Some(tree) = indices.get_mut(index_name) {
                tree.insert(key.to_string(), row_path.to_string())
                    .map_err(ExecuteError::wrap)?;
            }
            return Err(e);
        }

        Ok(removed)
    }

    /// Update a key for a given row path in an index.
    ///
    /// Lock is released before disk I/O. If save_index fails, the in-memory
    /// change is reverted.
    pub async fn update(
        &self,
        index_name: &str,
        old_key: &str,
        new_key: String,
        row_path: String,
    ) -> errors::Result<()> {
        let (meta, entries) = {
            let mut indices = self.indices.write().await;
            let tree = indices
                .get_mut(index_name)
                .ok_or_else(|| ExecuteError::wrap(format!("index '{}' not found", index_name)))?;

            tree.update(old_key, new_key.clone(), row_path.clone())
                .map_err(ExecuteError::wrap)?;

            let metas = self.metas.read().await;
            let meta = metas.get(index_name).ok_or_else(|| {
                ExecuteError::wrap(format!("index meta '{}' not found", index_name))
            })?;
            (meta.clone(), tree.to_entries())
        };

        // Disk I/O outside the lock
        if let Err(e) = self.write_index_file(&meta, &entries).await {
            // Revert: remove new key, re-insert old key
            let mut indices = self.indices.write().await;
            if let Some(tree) = indices.get_mut(index_name) {
                tree.remove(&new_key, &row_path);
                tree.insert(old_key.to_string(), row_path.to_string())
                    .map_err(ExecuteError::wrap)?;
            }
            return Err(e);
        }

        Ok(())
    }

    /// Look up row paths for an exact key match.
    pub async fn get(&self, index_name: &str, key: &str) -> errors::Result<Vec<String>> {
        let indices = self.indices.read().await;
        let tree = indices
            .get(index_name)
            .ok_or_else(|| ExecuteError::wrap(format!("index '{}' not found", index_name)))?;
        Ok(tree.get(key))
    }

    /// Point lookup for unique index.
    pub async fn get_one(&self, index_name: &str, key: &str) -> errors::Result<Option<String>> {
        let indices = self.indices.read().await;
        let tree = indices
            .get(index_name)
            .ok_or_else(|| ExecuteError::wrap(format!("index '{}' not found", index_name)))?;
        Ok(tree.get_one(key))
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
        Ok(tree.range(start, end))
    }

    /// Full scan on an index.
    pub async fn scan_all(&self, index_name: &str) -> errors::Result<Vec<IndexEntry>> {
        let indices = self.indices.read().await;
        let tree = indices
            .get(index_name)
            .ok_or_else(|| ExecuteError::wrap(format!("index '{}' not found", index_name)))?;
        Ok(tree.scan_all())
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
        Ok(tree.distinct_keys())
    }

    /// 인덱스 전체 엔트리를 교체합니다.
    /// CREATE INDEX 백필, DELETE 압축 후 재구축 등에 사용합니다.
    /// 디스크 기록이 성공한 경우에만 메모리 상태를 교체합니다.
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

        self.write_index_file(&meta, &entries).await?;

        let tree = BTreeIndex::from_entries(meta.column_name.clone(), meta.is_unique, entries);

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
        Ok(tree.len())
    }

    /// Persist a single index's full state to disk.
    /// This is called after every mutation to ensure disk backup is current.
    #[allow(dead_code)]
    async fn save_index(&self, index_name: &str) -> errors::Result<()> {
        let (meta, entries) = self.snapshot_index(index_name).await?;
        self.write_index_file(&meta, &entries).await
    }

    /// Read a snapshot of (meta, entries) from the in-memory state.
    #[allow(dead_code)]
    async fn snapshot_index(
        &self,
        index_name: &str,
    ) -> errors::Result<(IndexMeta, Vec<IndexEntry>)> {
        let indices = self.indices.read().await;
        let metas = self.metas.read().await;

        let tree = indices
            .get(index_name)
            .ok_or_else(|| ExecuteError::wrap(format!("index '{}' not found", index_name)))?;
        let meta = metas
            .get(index_name)
            .ok_or_else(|| ExecuteError::wrap(format!("index meta '{}' not found", index_name)))?;

        Ok((meta.clone(), tree.to_entries()))
    }

    /// Write index data to disk atomically (temp file + rename).
    /// Does not touch any locks -- pure disk I/O.
    async fn write_index_file(
        &self,
        meta: &IndexMeta,
        entries: &[IndexEntry],
    ) -> errors::Result<()> {
        let file_path = self.index_file_path(meta);
        let file_data = IndexFile {
            meta: meta.clone(),
            entries: entries.to_vec(),
        };
        let encoded = self.encoder.encode(&file_data);

        let temp_path = file_path.with_extension("idx.tmp");
        tokio::fs::write(&temp_path, encoded)
            .await
            .map_err(|e| ExecuteError::wrap(format!("failed to write index temp file: {}", e)))?;

        tokio::fs::rename(&temp_path, &file_path)
            .await
            .map_err(|e| ExecuteError::wrap(format!("failed to rename index file: {}", e)))?;

        Ok(())
    }

    /// Load all indices from disk into memory.
    /// Called on server startup to restore index state.
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
                        Some("idx") => {
                            let data = tokio::fs::read(&path).await.map_err(|e| {
                                ExecuteError::wrap(format!(
                                    "failed to read index file {:?}: {}",
                                    path, e
                                ))
                            })?;

                            let file_data: IndexFile =
                                self.encoder.decode(data.as_slice()).ok_or_else(|| {
                                    ExecuteError::wrap(format!(
                                        "failed to decode index file {:?}",
                                        path
                                    ))
                                })?;

                            let tree = BTreeIndex::from_entries(
                                file_data.meta.column_name.clone(),
                                file_data.meta.is_unique,
                                file_data.entries,
                            );

                            let index_name = file_data.meta.index_name.clone();

                            let mut indices = self.indices.write().await;
                            let mut metas = self.metas.write().await;
                            indices.insert(index_name.clone(), tree);
                            metas.insert(index_name, file_data.meta);
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
