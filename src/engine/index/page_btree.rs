//! Page-backed B+tree index (issue #230).
//!
//! Replaces the "mutate an in-memory BTreeMap, then serialize the whole
//! index to disk" approach in `manager.rs` (which was O(n) per mutation,
//! O(n^2) across n inserts) with a real disk-facing B+tree: every mutation
//! reads and writes only the handful of pages it touches.
//!
//! Structure, modeled after SimpleDB:
//! - Leaf pages hold sorted `(key, row_path)` entries plus a `next_leaf`
//!   pointer, so range scans walk a linked list of leaves without touching
//!   internal pages.
//! - Internal pages hold separator keys and child page ids.
//! - Splits never separate a duplicate-key group across two leaves. If a
//!   single key's group of row_paths doesn't fit in one leaf page on its
//!   own (no split point exists), the group is chained into fixed-size
//!   "overflow" pages hanging off the leaf via `LeafPage::overflow`. This is
//!   simpler than general overflow-page support (no space reclamation, no
//!   compaction) but keeps every on-disk page fixed-size, which is what lets
//!   `PageStore` address pages by simple arithmetic. Documented limitation:
//!   an overflow chain is never shrunk back down once created, even after
//!   removes -- acceptable for this MVP since delete does not coalesce.
//! - Delete does not coalesce/rebalance underfull leaves (spec allows this
//!   for MVP); a leaf can become empty and just stays in the sibling chain.
//! - There is no WAL integration here: the existing WAL (`engine::wal`)
//!   does not currently cover index mutations at all (no references to the
//!   index module anywhere under `src/engine/wal`), so there is nothing to
//!   preserve ordering with yet. If index WAL logging is added later, it
//!   must be written before the corresponding page write, per the
//!   "WAL-before-page-write" convention used elsewhere in the engine.
//!
//! Follow-up TODOs (explicitly out of scope for this MVP pass):
//! - No free-list reuse of pages freed by deletes/coalescing.
//! - `PageStore` file IO blocks the async executor thread briefly per page
//!   (see its module doc); moving to `spawn_blocking` is future work.
//! - No page cache -- every read/write round-trips to disk.

use std::path::Path;

use crate::errors;
use crate::errors::execute_error::ExecuteError;

use super::page::{InternalPage, LeafEntry, LeafPage, Page, PageId, INDEX_PAGE_SIZE};
use super::page_store::PageStore;
use super::IndexEntry;

/// A boxed, pinned, `Send` future -- needed because `insert_into_subtree`
/// and `push_tail_into_overflow` recurse through `async fn`, which `rustc`
/// cannot otherwise turn into a finite-sized state machine.
type BoxFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

/// A page-backed B+tree index for a single column.
pub struct PageBackedBTreeIndex {
    store: PageStore,
    column_name: String,
    is_unique: bool,
}

impl PageBackedBTreeIndex {
    /// Create a brand new, empty index file at `path`.
    pub async fn create(
        path: &Path,
        column_name: String,
        is_unique: bool,
    ) -> errors::Result<Self> {
        let store = PageStore::create(path, INDEX_PAGE_SIZE).await?;
        Ok(Self {
            store,
            column_name,
            is_unique,
        })
    }

    /// Open an existing index file at `path`.
    pub async fn open(path: &Path, column_name: String, is_unique: bool) -> errors::Result<Self> {
        let store = PageStore::open(path).await?;
        Ok(Self {
            store,
            column_name,
            is_unique,
        })
    }

    pub fn column_name(&self) -> &str {
        &self.column_name
    }

    pub fn is_unique(&self) -> bool {
        self.is_unique
    }

    /// Insert a key -> row_path mapping. Errors (without mutating anything)
    /// if this is a unique index and `key` already has an entry.
    pub async fn insert(&self, key: String, row_path: String) -> errors::Result<()> {
        let root = self.store.root_page_id().await?;

        let Some(root_id) = root else {
            // Empty tree: allocate the very first leaf and make it the root.
            let leaf_id = self.store.allocate_page().await?;
            let leaf = LeafPage {
                entries: vec![LeafEntry { key, row_path }],
                next_leaf: None,
                overflow: None,
            };
            self.store.write_page(leaf_id, &Page::Leaf(leaf)).await?;
            self.store.set_root_page_id(Some(leaf_id)).await?;
            return Ok(());
        };

        if let Some((sep_key, new_child)) = self.insert_into_subtree(root_id, key, row_path).await?
        {
            // Root split: create a new internal root pointing at the old
            // root and the newly split-off sibling.
            let new_root_id = self.store.allocate_page().await?;
            let new_root = InternalPage {
                keys: vec![sep_key],
                children: vec![root_id, new_child],
            };
            self.store
                .write_page(new_root_id, &Page::Internal(new_root))
                .await?;
            self.store.set_root_page_id(Some(new_root_id)).await?;
        }

        Ok(())
    }

    /// Insert into the subtree rooted at `page_id`. Returns
    /// `Some((separator_key, new_sibling_page_id))` if `page_id`'s page had
    /// to split, in which case the caller (the parent, or `insert` for the
    /// root) must link in the new sibling.
    fn insert_into_subtree<'a>(
        &'a self,
        page_id: PageId,
        key: String,
        row_path: String,
    ) -> BoxFuture<'a, errors::Result<Option<(String, PageId)>>> {
        Box::pin(async move {
            match self.store.read_page(page_id).await? {
                Page::Leaf(mut leaf) => {
                    if self.is_unique {
                        let exists_elsewhere = leaf.entries.iter().any(|e| e.key == key);
                        let exists_in_overflow = if let Some(overflow_id) = leaf.overflow {
                            self.overflow_contains_key(overflow_id, &key).await?
                        } else {
                            false
                        };
                        if exists_elsewhere || exists_in_overflow {
                            return Err(ExecuteError::wrap(format!(
                                "unique index violation on column '{}': key '{}' already exists",
                                self.column_name, key
                            )));
                        }
                    }

                    let pos = leaf.entries.partition_point(|e| e.key < key);
                    leaf.entries.insert(pos, LeafEntry { key, row_path });

                    self.rebalance_leaf_after_insert(page_id, leaf).await
                }
                Page::Internal(internal) => {
                    let child_index = internal.keys.partition_point(|sep| *sep <= key);
                    let child_id = internal.children[child_index];

                    let split = self.insert_into_subtree(child_id, key, row_path).await?;

                    let Some((sep_key, new_child)) = split else {
                        return Ok(None);
                    };

                    let mut internal = internal;
                    internal.keys.insert(child_index, sep_key);
                    internal.children.insert(child_index + 1, new_child);

                    self.rebalance_internal_after_insert(page_id, internal)
                        .await
                }
            }
        })
    }

    /// Check whether an overflow chain (for a single duplicate-key group)
    /// contains `key`. All entries in a chain share one key by
    /// construction, so this only needs to look at the first page.
    async fn overflow_contains_key(&self, overflow_id: PageId, key: &str) -> errors::Result<bool> {
        match self.store.read_page(overflow_id).await? {
            Page::Leaf(overflow) => Ok(overflow.entries.first().is_some_and(|e| e.key == key)),
            Page::Internal(_) => Err(ExecuteError::wrap(
                "corrupt index: overflow pointer references an internal page",
            )),
        }
    }

    /// The key shared by every entry in an overflow chain (all entries in a
    /// chain belong to the same duplicate-key group by construction, so
    /// only the first page needs to be read).
    async fn overflow_key(&self, overflow_id: PageId) -> errors::Result<String> {
        match self.store.read_page(overflow_id).await? {
            Page::Leaf(overflow) => overflow
                .entries
                .first()
                .map(|e| e.key.clone())
                .ok_or_else(|| ExecuteError::wrap("corrupt index: empty overflow page")),
            Page::Internal(_) => Err(ExecuteError::wrap(
                "corrupt index: overflow pointer references an internal page",
            )),
        }
    }

    /// After inserting into `leaf.entries`, write it back -- splitting or
    /// pushing into overflow first if it no longer fits in one page.
    async fn rebalance_leaf_after_insert(
        &self,
        page_id: PageId,
        mut leaf: LeafPage,
    ) -> errors::Result<Option<(String, PageId)>> {
        if super::page::encode_page(&Page::Leaf(leaf.clone()), self.store.page_size()).is_ok() {
            self.store.write_page(page_id, &Page::Leaf(leaf)).await?;
            return Ok(None);
        }

        if let Some(split_at) = find_split_point(&leaf.entries) {
            let sibling_entries = leaf.entries.split_off(split_at);
            let separator = sibling_entries[0].key.clone();

            // An overflow chain always holds the tail of a single
            // duplicate-key group (see the module doc comment), so it must
            // stay attached to whichever side of the split still holds that
            // key's resident entries -- not unconditionally follow the new
            // sibling. Otherwise `get`/`scan_all` silently lose the chain
            // once it ends up hanging off a leaf for the wrong key.
            let overflow_stays_with_leaf = match leaf.overflow {
                Some(overflow_id) => {
                    let overflow_key = self.overflow_key(overflow_id).await?;
                    leaf.entries.last().is_some_and(|e| e.key == overflow_key)
                }
                None => false,
            };

            let sibling_id = self.store.allocate_page().await?;
            let (leaf_overflow, sibling_overflow) = if overflow_stays_with_leaf {
                (leaf.overflow, None)
            } else {
                (None, leaf.overflow)
            };

            let sibling = LeafPage {
                entries: sibling_entries,
                next_leaf: leaf.next_leaf,
                overflow: sibling_overflow,
            };
            leaf.overflow = leaf_overflow;
            leaf.next_leaf = Some(sibling_id);

            // Whichever side keeps a non-`None` `overflow` pointer is
            // guaranteed (by the invariant above) to hold a single
            // homogeneous key, since a leaf only ever has an overflow chain
            // while its resident entries are all one key. Keeping that
            // pointer (rather than always dropping it, as before) plus the
            // new `next_leaf` pointer can occasionally push that side a
            // few bytes over budget; if so, it's always safe to shed more
            // of its (single-key) tail into the chain it already owns.
            if leaf.overflow.is_some()
                && super::page::encode_page(&Page::Leaf(leaf.clone()), self.store.page_size())
                    .is_err()
            {
                self.push_tail_into_overflow(page_id, leaf).await?;
            } else {
                self.store.write_page(page_id, &Page::Leaf(leaf)).await?;
            }

            if sibling.overflow.is_some()
                && super::page::encode_page(&Page::Leaf(sibling.clone()), self.store.page_size())
                    .is_err()
            {
                self.push_tail_into_overflow(sibling_id, sibling).await?;
            } else {
                self.store
                    .write_page(sibling_id, &Page::Leaf(sibling))
                    .await?;
            }

            return Ok(Some((separator, sibling_id)));
        }

        // Every entry in this leaf shares one key: normal splitting would
        // separate a duplicate-key group, which is not allowed. Push the
        // tail of the page into a chained overflow page instead (see the
        // module doc comment for the tradeoffs of this approach).
        self.push_tail_into_overflow(page_id, leaf).await?;
        Ok(None)
    }

    /// Move entries off the back of `leaf` into a newly allocated overflow
    /// page (chained via `leaf.overflow`) until `leaf` itself fits in one
    /// page again. Recurses if the overflow page itself doesn't fit.
    fn push_tail_into_overflow<'a>(
        &'a self,
        page_id: PageId,
        mut leaf: LeafPage,
    ) -> BoxFuture<'a, errors::Result<()>> {
        Box::pin(async move {
            let mut moved = Vec::new();
            loop {
                if super::page::encode_page(&Page::Leaf(leaf.clone()), self.store.page_size())
                    .is_ok()
                {
                    break;
                }
                match leaf.entries.pop() {
                    Some(entry) => moved.insert(0, entry),
                    None => {
                        return Err(ExecuteError::wrap(
                            "a single index entry does not fit within one page; increase INDEX_PAGE_SIZE",
                        ));
                    }
                }
            }

            let overflow_id = self.store.allocate_page().await?;
            let overflow_page = LeafPage {
                entries: moved,
                next_leaf: None,
                overflow: leaf.overflow,
            };
            leaf.overflow = Some(overflow_id);

            self.store.write_page(page_id, &Page::Leaf(leaf)).await?;

            // The overflow page might itself be too large (a key with an
            // enormous number of duplicates); recurse to chain further.
            if super::page::encode_page(&Page::Leaf(overflow_page.clone()), self.store.page_size())
                .is_ok()
            {
                self.store
                    .write_page(overflow_id, &Page::Leaf(overflow_page))
                    .await
            } else {
                self.push_tail_into_overflow(overflow_id, overflow_page)
                    .await
            }
        })
    }

    async fn rebalance_internal_after_insert(
        &self,
        page_id: PageId,
        internal: InternalPage,
    ) -> errors::Result<Option<(String, PageId)>> {
        if super::page::encode_page(&Page::Internal(internal.clone()), self.store.page_size())
            .is_ok()
        {
            self.store
                .write_page(page_id, &Page::Internal(internal))
                .await?;
            return Ok(None);
        }

        let mut internal = internal;
        let split_at = internal.keys.len() / 2;
        let separator = internal.keys[split_at].clone();

        let sibling_keys = internal.keys.split_off(split_at + 1);
        internal.keys.pop(); // drop the promoted separator itself
        let sibling_children = internal.children.split_off(split_at + 1);

        let sibling_id = self.store.allocate_page().await?;
        let sibling = InternalPage {
            keys: sibling_keys,
            children: sibling_children,
        };

        self.store
            .write_page(page_id, &Page::Internal(internal))
            .await?;
        self.store
            .write_page(sibling_id, &Page::Internal(sibling))
            .await?;

        Ok(Some((separator, sibling_id)))
    }

    /// Find the leaf page id whose key range contains `key` (or would
    /// contain it, if absent).
    async fn find_leaf(&self, key: &str) -> errors::Result<Option<PageId>> {
        let Some(root_id) = self.store.root_page_id().await? else {
            return Ok(None);
        };

        let mut current = root_id;
        loop {
            match self.store.read_page(current).await? {
                Page::Leaf(_) => return Ok(Some(current)),
                Page::Internal(internal) => {
                    let child_index = internal.keys.partition_point(|sep| sep.as_str() <= key);
                    current = internal.children[child_index];
                }
            }
        }
    }

    /// Find the leftmost leaf page id (used for full scans and open-start
    /// range scans).
    async fn find_leftmost_leaf(&self) -> errors::Result<Option<PageId>> {
        let Some(root_id) = self.store.root_page_id().await? else {
            return Ok(None);
        };

        let mut current = root_id;
        loop {
            match self.store.read_page(current).await? {
                Page::Leaf(_) => return Ok(Some(current)),
                Page::Internal(internal) => {
                    current = internal.children[0];
                }
            }
        }
    }

    /// Collect all entries for `page_id`'s leaf plus any overflow chain.
    async fn leaf_entries_with_overflow(&self, page_id: PageId) -> errors::Result<Vec<LeafEntry>> {
        let leaf = match self.store.read_page(page_id).await? {
            Page::Leaf(leaf) => leaf,
            Page::Internal(_) => {
                return Err(ExecuteError::wrap("corrupt index: expected a leaf page"))
            }
        };

        let mut entries = leaf.entries;
        let mut next_overflow = leaf.overflow;
        while let Some(overflow_id) = next_overflow {
            match self.store.read_page(overflow_id).await? {
                Page::Leaf(overflow) => {
                    entries.extend(overflow.entries);
                    next_overflow = overflow.overflow;
                }
                Page::Internal(_) => {
                    return Err(ExecuteError::wrap(
                        "corrupt index: overflow pointer references an internal page",
                    ));
                }
            }
        }

        Ok(entries)
    }

    pub async fn get(&self, key: &str) -> errors::Result<Vec<String>> {
        let Some(leaf_id) = self.find_leaf(key).await? else {
            return Ok(Vec::new());
        };

        let entries = self.leaf_entries_with_overflow(leaf_id).await?;
        Ok(entries
            .into_iter()
            .filter(|e| e.key == key)
            .map(|e| e.row_path)
            .collect())
    }

    pub async fn get_one(&self, key: &str) -> errors::Result<Option<String>> {
        Ok(self.get(key).await?.into_iter().next())
    }

    /// Remove a single `(key, row_path)` mapping. Returns `true` if it was
    /// found and removed. Does not coalesce underfull leaves.
    pub async fn remove(&self, key: &str, row_path: &str) -> errors::Result<bool> {
        let Some(leaf_id) = self.find_leaf(key).await? else {
            return Ok(false);
        };

        let leaf = match self.store.read_page(leaf_id).await? {
            Page::Leaf(leaf) => leaf,
            Page::Internal(_) => {
                return Err(ExecuteError::wrap("corrupt index: expected a leaf page"))
            }
        };

        if let Some(idx) = leaf
            .entries
            .iter()
            .position(|e| e.key == key && e.row_path == row_path)
        {
            let mut leaf = leaf;
            leaf.entries.remove(idx);
            self.store.write_page(leaf_id, &Page::Leaf(leaf)).await?;
            return Ok(true);
        }

        // Walk the overflow chain looking for the entry.
        let mut next_overflow = leaf.overflow;
        while let Some(overflow_id) = next_overflow {
            let mut overflow = match self.store.read_page(overflow_id).await? {
                Page::Leaf(overflow) => overflow,
                Page::Internal(_) => {
                    return Err(ExecuteError::wrap(
                        "corrupt index: overflow pointer references an internal page",
                    ));
                }
            };

            if let Some(idx) = overflow
                .entries
                .iter()
                .position(|e| e.key == key && e.row_path == row_path)
            {
                overflow.entries.remove(idx);
                self.store
                    .write_page(overflow_id, &Page::Leaf(overflow))
                    .await?;
                return Ok(true);
            }

            next_overflow = overflow.overflow;
        }

        Ok(false)
    }

    /// Update a key for a given row path: remove the old mapping, insert
    /// the new one. Validates uniqueness before mutating.
    pub async fn update(
        &self,
        old_key: &str,
        new_key: String,
        row_path: String,
    ) -> errors::Result<()> {
        let old_exists = self.get(old_key).await?.iter().any(|p| p == &row_path);
        if !old_exists {
            return Err(ExecuteError::wrap(format!(
                "cannot update: row_path '{}' not found under key '{}'",
                row_path, old_key
            )));
        }

        if self.is_unique && old_key != new_key && self.get_one(&new_key).await?.is_some() {
            return Err(ExecuteError::wrap(format!(
                "unique index violation on column '{}': key '{}' already exists",
                self.column_name, new_key
            )));
        }

        self.remove(old_key, &row_path).await?;
        self.insert(new_key, row_path).await
    }

    /// Range scan over `[start, end)`. `None` on either bound means
    /// unbounded on that side.
    pub async fn range(
        &self,
        start: Option<&str>,
        end: Option<&str>,
    ) -> errors::Result<Vec<IndexEntry>> {
        if start.zip(end).is_some_and(|(s, e)| s > e) {
            return Ok(Vec::new());
        }

        let start_leaf = match start {
            Some(key) => self.find_leaf(key).await?,
            None => self.find_leftmost_leaf().await?,
        };

        let Some(mut current) = start_leaf else {
            return Ok(Vec::new());
        };

        let mut result = Vec::new();
        loop {
            let entries = self.leaf_entries_with_overflow(current).await?;
            let next_leaf = match self.store.read_page(current).await? {
                Page::Leaf(leaf) => leaf.next_leaf,
                Page::Internal(_) => {
                    return Err(ExecuteError::wrap("corrupt index: expected a leaf page"))
                }
            };

            let mut done = false;
            for entry in entries {
                if start.is_some_and(|s| entry.key.as_str() < s) {
                    continue;
                }
                if end.is_some_and(|e| entry.key.as_str() >= e) {
                    done = true;
                    break;
                }
                result.push(IndexEntry {
                    key: entry.key,
                    row_path: entry.row_path,
                });
            }

            if done {
                break;
            }
            match next_leaf {
                Some(next) => current = next,
                None => break,
            }
        }

        Ok(result)
    }

    pub async fn scan_all(&self) -> errors::Result<Vec<IndexEntry>> {
        self.range(None, None).await
    }

    /// Total number of `(key, row_path)` entries in the index.
    pub async fn len(&self) -> errors::Result<usize> {
        Ok(self.scan_all().await?.len())
    }

    /// Number of distinct keys in the index.
    pub async fn distinct_keys(&self) -> errors::Result<usize> {
        let entries = self.scan_all().await?;
        let mut count = 0;
        let mut last_key: Option<&str> = None;
        for entry in &entries {
            if last_key != Some(entry.key.as_str()) {
                count += 1;
                last_key = Some(entry.key.as_str());
            }
        }
        Ok(count)
    }
}

/// Find an index `i` in `[1, entries.len())` such that `entries[i-1].key !=
/// entries[i].key`, as close to the midpoint as possible. Returns `None` if
/// every entry shares the same key (the leaf cannot be split without
/// breaking a duplicate-key group).
fn find_split_point(entries: &[LeafEntry]) -> Option<usize> {
    let len = entries.len();
    if len < 2 {
        return None;
    }
    let mid = len / 2;
    let max_delta = mid.max(len - mid);

    for delta in 0..=max_delta {
        let up = mid + delta;
        if up > 0 && up < len && entries[up - 1].key != entries[up].key {
            return Some(up);
        }
        if delta <= mid {
            let down = mid - delta;
            if down > 0 && down < len && entries[down - 1].key != entries[down].key {
                return Some(down);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> std::path::PathBuf {
        let dir = std::path::PathBuf::from("target/test_page_btree");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        let _ = std::fs::remove_file(&path);
        path
    }

    #[tokio::test]
    async fn insert_then_reload_from_disk_finds_the_entry() {
        let path = temp_path("insert_reload.idx");

        {
            let idx = PageBackedBTreeIndex::create(&path, "id".to_string(), false)
                .await
                .unwrap();
            idx.insert("I:001".to_string(), "/r/1".to_string())
                .await
                .unwrap();
        }

        {
            let idx = PageBackedBTreeIndex::open(&path, "id".to_string(), false)
                .await
                .unwrap();
            let results = idx.get("I:001").await.unwrap();
            assert_eq!(results, vec!["/r/1".to_string()]);
        }
    }

    fn int_key(i: i64) -> String {
        format!("I:{:020}", i)
    }

    #[tokio::test]
    async fn unique_index_rejects_duplicate_key_without_mutating_state() {
        let path = temp_path("unique_violation.idx");
        let idx = PageBackedBTreeIndex::create(&path, "id".to_string(), true)
            .await
            .unwrap();

        idx.insert("I:001".to_string(), "/r/1".to_string())
            .await
            .unwrap();

        let result = idx.insert("I:001".to_string(), "/r/2".to_string()).await;
        assert!(result.is_err());

        // The failed insert must not have mutated the index.
        assert_eq!(idx.get("I:001").await.unwrap(), vec!["/r/1".to_string()]);
        assert_eq!(idx.len().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn remove_then_reload_no_longer_finds_the_entry() {
        let path = temp_path("remove_reload.idx");

        {
            let idx = PageBackedBTreeIndex::create(&path, "name".to_string(), false)
                .await
                .unwrap();
            idx.insert("S:alice".to_string(), "/r/1".to_string())
                .await
                .unwrap();
            idx.insert("S:alice".to_string(), "/r/2".to_string())
                .await
                .unwrap();

            let removed = idx.remove("S:alice", "/r/1").await.unwrap();
            assert!(removed);
            // Removing something already gone returns false, not an error.
            assert!(!idx.remove("S:alice", "/r/1").await.unwrap());
        }

        {
            let idx = PageBackedBTreeIndex::open(&path, "name".to_string(), false)
                .await
                .unwrap();
            assert_eq!(idx.get("S:alice").await.unwrap(), vec!["/r/2".to_string()]);
            assert_eq!(idx.len().await.unwrap(), 1);
        }
    }

    #[tokio::test]
    async fn update_then_reload_moves_the_key() {
        let path = temp_path("update_reload.idx");

        {
            let idx = PageBackedBTreeIndex::create(&path, "name".to_string(), true)
                .await
                .unwrap();
            idx.insert("S:old".to_string(), "/r/1".to_string())
                .await
                .unwrap();
            idx.update("S:old", "S:new".to_string(), "/r/1".to_string())
                .await
                .unwrap();
        }

        {
            let idx = PageBackedBTreeIndex::open(&path, "name".to_string(), true)
                .await
                .unwrap();
            assert!(idx.get("S:old").await.unwrap().is_empty());
            assert_eq!(
                idx.get_one("S:new").await.unwrap(),
                Some("/r/1".to_string())
            );
        }
    }

    #[tokio::test]
    async fn update_rejects_unique_violation_on_new_key() {
        let path = temp_path("update_unique_violation.idx");
        let idx = PageBackedBTreeIndex::create(&path, "name".to_string(), true)
            .await
            .unwrap();

        idx.insert("S:taken".to_string(), "/r/1".to_string())
            .await
            .unwrap();
        idx.insert("S:mine".to_string(), "/r/2".to_string())
            .await
            .unwrap();

        let result = idx
            .update("S:mine", "S:taken".to_string(), "/r/2".to_string())
            .await;
        assert!(result.is_err());

        // Original mapping must still be intact.
        assert_eq!(
            idx.get_one("S:mine").await.unwrap(),
            Some("/r/2".to_string())
        );
    }

    #[tokio::test]
    async fn inserting_enough_entries_forces_leaf_splits_and_range_scan_still_works() {
        let path = temp_path("split_range.idx");
        let idx = PageBackedBTreeIndex::create(&path, "id".to_string(), false)
            .await
            .unwrap();

        // INDEX_PAGE_SIZE is 4096 bytes; each entry here is roughly
        // 30-40 bytes once bincode+header overhead is counted, so a few
        // hundred inserts guarantees multiple leaf (and likely internal)
        // page splits.
        let n = 800;
        for i in 0..n {
            idx.insert(int_key(i), format!("/r/{}", i)).await.unwrap();
        }

        assert_eq!(idx.len().await.unwrap(), n as usize);
        assert_eq!(idx.distinct_keys().await.unwrap(), n as usize);

        // Range scan across many leaves.
        let start = int_key(100);
        let end = int_key(200);
        let results = idx.range(Some(&start), Some(&end)).await.unwrap();
        assert_eq!(results.len(), 100);
        assert_eq!(results[0].key, start);
        assert_eq!(results.last().unwrap().key, int_key(199));

        // Full scan preserves global sorted order across all leaves.
        let all = idx.scan_all().await.unwrap();
        assert_eq!(all.len(), n as usize);
        for w in all.windows(2) {
            assert!(w[0].key < w[1].key);
        }

        // Exact-match get still works after splitting.
        assert_eq!(idx.get(&int_key(750)).await.unwrap(), vec!["/r/750".to_string()]);
    }

    #[tokio::test]
    async fn duplicate_key_group_survives_splits_and_reload() {
        let path = temp_path("duplicates.idx");

        {
            let idx = PageBackedBTreeIndex::create(&path, "status".to_string(), false)
                .await
                .unwrap();

            // A large duplicate-key group, big enough to overflow a single
            // leaf page (see module doc comment on the overflow-chain
            // policy), interleaved with enough distinct keys on either side
            // to also force normal leaf splits elsewhere in the tree.
            for i in 0..300 {
                idx.insert(format!("I:before:{:05}", i), format!("/before/{}", i))
                    .await
                    .unwrap();
            }
            for i in 0..200 {
                idx.insert("S:dup".to_string(), format!("/dup/{}", i))
                    .await
                    .unwrap();
            }
            for i in 0..300 {
                idx.insert(format!("I:zzafter:{:05}", i), format!("/after/{}", i))
                    .await
                    .unwrap();
            }

            let dups = idx.get("S:dup").await.unwrap();
            assert_eq!(dups.len(), 200);
        }

        {
            let idx = PageBackedBTreeIndex::open(&path, "status".to_string(), false)
                .await
                .unwrap();
            let dups = idx.get("S:dup").await.unwrap();
            assert_eq!(dups.len(), 200);
            for i in 0..200 {
                assert!(dups.contains(&format!("/dup/{}", i)));
            }

            // Removing one duplicate leaves the rest intact.
            assert!(idx.remove("S:dup", "/dup/0").await.unwrap());
            assert_eq!(idx.get("S:dup").await.unwrap().len(), 199);
        }
    }

    #[tokio::test]
    async fn overflow_chain_stays_with_its_key_when_a_larger_key_splits_the_leaf() {
        let path = temp_path("overflow_split.idx");

        {
            let idx = PageBackedBTreeIndex::create(&path, "status".to_string(), false)
                .await
                .unwrap();

            // Enough duplicates of a single key to force an overflow chain
            // while this remains the only (root) leaf.
            for i in 0..200 {
                idx.insert("K:dup".to_string(), format!("/dup/{}", i))
                    .await
                    .unwrap();
            }
            assert_eq!(idx.get("K:dup").await.unwrap().len(), 200);

            // A single larger key lands in the same (only) leaf and forces
            // it to split. Before the fix, the overflow chain
            // unconditionally followed the new sibling -- which here holds
            // the unrelated "Z:after" key -- silently orphaning "K:dup"'s
            // overflow entries.
            idx.insert("Z:after".to_string(), "/after/0".to_string())
                .await
                .unwrap();

            assert_eq!(idx.get("K:dup").await.unwrap().len(), 200);
            assert_eq!(
                idx.get("Z:after").await.unwrap(),
                vec!["/after/0".to_string()]
            );

            let all = idx.scan_all().await.unwrap();
            assert_eq!(all.len(), 201);
            for w in all.windows(2) {
                assert!(w[0].key <= w[1].key);
            }
        }

        // The fix must survive a reload from disk too.
        {
            let idx = PageBackedBTreeIndex::open(&path, "status".to_string(), false)
                .await
                .unwrap();
            assert_eq!(idx.get("K:dup").await.unwrap().len(), 200);

            let all = idx.scan_all().await.unwrap();
            assert_eq!(all.len(), 201);
            for w in all.windows(2) {
                assert!(w[0].key <= w[1].key);
            }

            let range = idx.range(Some("K:dup"), None).await.unwrap();
            assert_eq!(range.len(), 201);
        }
    }

    #[tokio::test]
    async fn find_split_point_returns_none_when_every_entry_shares_one_key() {
        let entries: Vec<LeafEntry> = (0..5)
            .map(|i| LeafEntry {
                key: "S:dup".to_string(),
                row_path: format!("/r/{}", i),
            })
            .collect();
        assert_eq!(find_split_point(&entries), None);
    }

    #[tokio::test]
    async fn find_split_point_never_separates_a_duplicate_group() {
        let entries = vec![
            LeafEntry { key: "a".to_string(), row_path: "1".to_string() },
            LeafEntry { key: "b".to_string(), row_path: "1".to_string() },
            LeafEntry { key: "b".to_string(), row_path: "2".to_string() },
            LeafEntry { key: "b".to_string(), row_path: "3".to_string() },
            LeafEntry { key: "c".to_string(), row_path: "1".to_string() },
        ];
        let split = find_split_point(&entries).unwrap();
        assert_ne!(entries[split - 1].key, entries[split].key);
    }
}
