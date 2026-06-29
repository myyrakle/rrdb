use std::collections::BTreeMap;

use super::IndexEntry;

/// A B-tree based index that lives in memory but can be persisted to disk.
///
/// The tree maps key strings to sets of row file paths (one key may map to
/// multiple rows if the index is not unique).
///
/// Design notes (issue #160):
/// - Uses Rust's std::collections::BTreeMap as the underlying B-tree
/// - Memory is the primary store; disk is the backup
/// - On every mutation, the caller is responsible for flushing to disk via
///   the IndexManager (which owns the IndexManager that coordinates disk I/O)
/// - The tree is loaded from disk on startup
#[derive(Debug, Clone)]
pub struct BTreeIndex {
    /// Column name this index is built on
    column_name: String,
    /// Whether this index enforces uniqueness
    is_unique: bool,
    /// The B-tree: key -> set of row paths
    tree: BTreeMap<String, Vec<String>>,
}

impl BTreeIndex {
    pub fn new(column_name: String, is_unique: bool) -> Self {
        Self {
            column_name,
            is_unique,
            tree: BTreeMap::<String, Vec<String>>::new(),
        }
    }

    pub fn column_name(&self) -> &str {
        &self.column_name
    }

    pub fn is_unique(&self) -> bool {
        self.is_unique
    }

    pub fn len(&self) -> usize {
        self.tree.values().map(|v| v.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Insert a key->row_path mapping into the index.
    /// Returns Err if a unique constraint is violated.
    pub fn insert(&mut self, key: String, row_path: String) -> Result<(), String> {
        if self.is_unique {
            if let Some(existing) = self.tree.get(&key) {
                if !existing.is_empty() {
                    return Err(format!(
                        "unique index violation on column '{}': key '{}' already exists",
                        self.column_name, key
                    ));
                }
            }
        }

        self.tree.entry(key).or_default().push(row_path);

        Ok(())
    }

    /// Remove a specific row_path from the index for the given key.
    /// Returns true if the entry was found and removed.
    pub fn remove(&mut self, key: &str, row_path: &str) -> bool {
        let removed;
        let empty_after;

        if let Some(paths) = self.tree.get_mut(key) {
            let before = paths.len();
            paths.retain(|p| p != row_path);
            removed = paths.len() < before;
            empty_after = paths.is_empty();
        } else {
            return false;
        }

        if empty_after {
            self.tree.remove(key);
        }

        removed
    }

    /// Remove an entire key from the index (all row paths).
    pub fn remove_key(&mut self, key: &str) -> Option<Vec<String>> {
        self.tree.remove(key)
    }

    /// Look up all row paths for an exact key match.
    pub fn get(&self, key: &str) -> Vec<String> {
        self.tree.get(key).cloned().unwrap_or_default()
    }

    /// Point lookup: returns the single row path for a unique index, or the
    /// first match for non-unique.
    pub fn get_one(&self, key: &str) -> Option<String> {
        self.tree.get(key).and_then(|v| v.first().cloned())
    }

    /// Range scan: return all row paths for keys in [start, end).
    pub fn range(&self, start: Option<&str>, end: Option<&str>) -> Vec<IndexEntry> {
        let mut result = Vec::new();

        match (start, end) {
            (Some(start), Some(end)) => {
                if start <= end {
                    for (key, paths) in self.tree.range(start.to_string()..end.to_string()) {
                        for path in paths {
                            result.push(IndexEntry {
                                key: key.clone(),
                                row_path: path.clone(),
                            });
                        }
                    }
                }
            }
            (Some(start), None) => {
                for (key, paths) in self.tree.range(start.to_string()..) {
                    for path in paths {
                        result.push(IndexEntry {
                            key: key.clone(),
                            row_path: path.clone(),
                        });
                    }
                }
            }
            (None, Some(end)) => {
                for (key, paths) in self.tree.range(..end.to_string()) {
                    for path in paths {
                        result.push(IndexEntry {
                            key: key.clone(),
                            row_path: path.clone(),
                        });
                    }
                }
            }
            (None, None) => {
                for (key, paths) in &self.tree {
                    for path in paths {
                        result.push(IndexEntry {
                            key: key.clone(),
                            row_path: path.clone(),
                        });
                    }
                }
            }
        }

        result
    }

    /// Full scan: return all entries in key order.
    pub fn scan_all(&self) -> Vec<IndexEntry> {
        self.range(None, None)
    }

    /// Serialize all entries to a Vec for disk persistence.
    pub fn to_entries(&self) -> Vec<IndexEntry> {
        self.scan_all()
    }

    /// Rebuild the tree from a list of entries (e.g., loaded from disk).
    pub fn from_entries(column_name: String, is_unique: bool, entries: Vec<IndexEntry>) -> Self {
        let mut tree: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for entry in entries {
            tree.entry(entry.key).or_default().push(entry.row_path);
        }
        Self {
            column_name,
            is_unique,
            tree,
        }
    }

    /// Update a key for a given row path: remove old key, insert new key.
    pub fn update(
        &mut self,
        old_key: &str,
        new_key: String,
        row_path: String,
    ) -> Result<(), String> {
        self.remove(old_key, &row_path);
        self.insert(new_key, row_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_index() -> BTreeIndex {
        BTreeIndex::new("age".to_string(), false)
    }

    fn make_unique_index() -> BTreeIndex {
        BTreeIndex::new("id".to_string(), true)
    }

    #[test]
    fn test_insert_and_get() {
        let mut idx = make_index();
        idx.insert("I:00000000000000000030".into(), "/db/tbl/rows/a".into())
            .unwrap();
        idx.insert("I:00000000000000000030".into(), "/db/tbl/rows/b".into())
            .unwrap();
        idx.insert("I:00000000000000000025".into(), "/db/tbl/rows/c".into())
            .unwrap();

        let results = idx.get("I:00000000000000000030");
        assert_eq!(results.len(), 2);
        assert!(results.contains(&"/db/tbl/rows/a".to_string()));
        assert!(results.contains(&"/db/tbl/rows/b".to_string()));

        let results = idx.get("I:00000000000000000025");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_unique_violation() {
        let mut idx = make_unique_index();
        idx.insert("S:alice".into(), "/db/tbl/rows/a".into())
            .unwrap();

        let result = idx.insert("S:alice".into(), "/db/tbl/rows/b".into());
        assert!(result.is_err());
        assert_eq!(idx.len(), 1);
    }

    #[test]
    fn test_unique_allows_different_keys() {
        let mut idx = make_unique_index();
        idx.insert("S:alice".into(), "/db/tbl/rows/a".into())
            .unwrap();
        idx.insert("S:bob".into(), "/db/tbl/rows/b".into()).unwrap();
        assert_eq!(idx.len(), 2);
    }

    #[test]
    fn test_remove() {
        let mut idx = make_index();
        idx.insert("S:hello".into(), "/db/tbl/rows/a".into())
            .unwrap();
        idx.insert("S:hello".into(), "/db/tbl/rows/b".into())
            .unwrap();

        assert!(idx.remove("S:hello", "/db/tbl/rows/a"));
        assert_eq!(idx.get("S:hello").len(), 1);

        assert!(idx.remove("S:hello", "/db/tbl/rows/b"));
        assert!(idx.get("S:hello").is_empty());
        assert!(idx.is_empty());
    }

    #[test]
    fn test_remove_nonexistent() {
        let mut idx = make_index();
        idx.insert("S:hello".into(), "/db/tbl/rows/a".into())
            .unwrap();

        assert!(!idx.remove("S:world", "/db/tbl/rows/a"));
        assert!(!idx.remove("S:hello", "/db/tbl/rows/nonexistent"));
    }

    #[test]
    fn test_remove_key() {
        let mut idx = make_index();
        idx.insert("S:a".into(), "/db/tbl/rows/1".into()).unwrap();
        idx.insert("S:b".into(), "/db/tbl/rows/2".into()).unwrap();

        let removed = idx.remove_key("S:a");
        assert_eq!(removed, Some(vec!["/db/tbl/rows/1".to_string()]));
        assert!(idx.get("S:a").is_empty());
        assert_eq!(idx.get("S:b").len(), 1);
    }

    #[test]
    fn test_range_scan() {
        let mut idx = make_index();
        idx.insert("I:00000000000000000010".into(), "/r/1".into())
            .unwrap();
        idx.insert("I:00000000000000000020".into(), "/r/2".into())
            .unwrap();
        idx.insert("I:00000000000000000030".into(), "/r/3".into())
            .unwrap();
        idx.insert("I:00000000000000000040".into(), "/r/4".into())
            .unwrap();

        // Range [20, 40)
        let results = idx.range(
            Some("I:00000000000000000020"),
            Some("I:00000000000000000040"),
        );
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].row_path, "/r/2");
        assert_eq!(results[1].row_path, "/r/3");

        // Range [20, end)
        let results = idx.range(Some("I:00000000000000000020"), None);
        assert_eq!(results.len(), 3);

        // Range (start, 30)
        let results = idx.range(None, Some("I:00000000000000000030"));
        assert_eq!(results.len(), 2);

        // Full scan
        let results = idx.range(None, None);
        assert_eq!(results.len(), 4);
    }

    #[test]
    fn test_scan_all_ordered() {
        let mut idx = make_index();
        idx.insert("S:banana".into(), "/r/3".into()).unwrap();
        idx.insert("S:apple".into(), "/r/1".into()).unwrap();
        idx.insert("S:cherry".into(), "/r/2".into()).unwrap();

        let entries = idx.scan_all();
        assert_eq!(entries.len(), 3);
        // BTreeMap maintains sorted order
        assert_eq!(entries[0].key, "S:apple");
        assert_eq!(entries[1].key, "S:banana");
        assert_eq!(entries[2].key, "S:cherry");
    }

    #[test]
    fn test_update() {
        let mut idx = make_unique_index();
        idx.insert("S:old".into(), "/r/1".into()).unwrap();

        idx.update("S:old", "S:new".into(), "/r/1".into()).unwrap();

        assert!(idx.get("S:old").is_empty());
        assert_eq!(idx.get("S:new").len(), 1);
        assert_eq!(idx.get_one("S:new"), Some("/r/1".to_string()));
    }

    #[test]
    fn test_to_entries_and_from_entries() {
        let mut idx = make_index();
        idx.insert("S:a".into(), "/r/1".into()).unwrap();
        idx.insert("S:b".into(), "/r/2".into()).unwrap();
        idx.insert("S:b".into(), "/r/3".into()).unwrap();

        let entries = idx.to_entries();
        assert_eq!(entries.len(), 3);

        let rebuilt = BTreeIndex::from_entries("age".to_string(), false, entries);
        assert_eq!(rebuilt.len(), 3);
        assert_eq!(rebuilt.get("S:a").len(), 1);
        assert_eq!(rebuilt.get("S:b").len(), 2);
    }

    #[test]
    fn test_get_one() {
        let mut idx = make_unique_index();
        idx.insert("S:alice".into(), "/r/1".into()).unwrap();
        assert_eq!(idx.get_one("S:alice"), Some("/r/1".to_string()));
        assert_eq!(idx.get_one("S:bob"), None);
    }

    #[test]
    fn test_len_and_is_empty() {
        let mut idx = make_index();
        assert!(idx.is_empty());
        assert_eq!(idx.len(), 0);

        idx.insert("S:a".into(), "/r/1".into()).unwrap();
        assert!(!idx.is_empty());
        assert_eq!(idx.len(), 1);

        idx.insert("S:a".into(), "/r/2".into()).unwrap();
        assert_eq!(idx.len(), 2);

        idx.remove("S:a", "/r/1");
        assert_eq!(idx.len(), 1);
    }
}
