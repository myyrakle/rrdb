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
//!   an overflow chain is not compacted -- entries are not migrated between
//!   overflow pages to close gaps (issue #235) -- but a page that becomes
//!   fully empty is unlinked and reclaimed (issue #232).
//! - Delete reclaims pages that become empty: an emptied leaf is spliced
//!   out of the `next_leaf` chain, removed from its parent, and returned to
//!   the page store's free-list, and a root left with one child collapses
//!   (issue #232). Underfull but non-empty pages are still not rebalanced
//!   or redistributed, since that would mean rewriting separator keys.
//! - There is no WAL integration here: the existing WAL (`engine::wal`)
//!   does not currently cover index mutations at all (no references to the
//!   index module anywhere under `src/engine/wal`), so there is nothing to
//!   preserve ordering with yet. If index WAL logging is added later, it
//!   must be written before the corresponding page write, per the
//!   "WAL-before-page-write" convention used elsewhere in the engine.
//!
//! Follow-up TODOs (explicitly out of scope for this MVP pass):
//! - `PageStore` file IO blocks the async executor thread briefly per page
//!   (see its module doc); moving to `spawn_blocking` is future work.
//! - No page cache -- every read/write round-trips to disk.
//! - No redistribution/coalescing of underfull (but non-empty) pages, and
//!   no compaction of duplicate-key overflow chains (issue #235).

use std::path::Path;

use crate::errors;
use crate::errors::execute_error::ExecuteError;

use super::IndexEntry;
use super::page::{INDEX_PAGE_SIZE, InternalPage, LeafEntry, LeafPage, Page, PageId};
use super::page_store::PageStore;

/// A boxed, pinned, `Send` future -- needed because `insert_into_subtree`
/// and `push_tail_into_overflow` recurse through `async fn`, which `rustc`
/// cannot otherwise turn into a finite-sized state machine.
type BoxFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

/// A root-to-leaf descent: the leaf reached, plus the internal pages walked
/// through and which child slot was taken at each (see `find_leaf_path`).
struct LeafPath {
    leaf_id: PageId,
    parents: Vec<(PageId, usize)>,
}

/// A page-backed B+tree index for a single column.
pub struct PageBackedBTreeIndex {
    store: PageStore,
    column_name: String,
    is_unique: bool,
}

impl PageBackedBTreeIndex {
    /// Create a brand new, empty index file at `path`.
    pub async fn create(path: &Path, column_name: String, is_unique: bool) -> errors::Result<Self> {
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

    /// Number of pages currently on the free-list (issue #232).
    /// Diagnostic helper, also used by tests.
    pub async fn free_page_count(&self) -> errors::Result<usize> {
        self.store.free_page_count().await
    }

    /// Number of page slots the index file has ever allocated (issue #232).
    /// Diagnostic helper, also used by tests.
    pub async fn allocated_page_count(&self) -> errors::Result<PageId> {
        self.store.allocated_page_count().await
    }

    /// Read `page_id`, requiring it to be a leaf page.
    ///
    /// Encountering an internal or freed page here means a pointer somewhere
    /// in the tree outlived the page it pointed at, so this reports
    /// corruption rather than silently returning empty results.
    async fn read_leaf(&self, page_id: PageId) -> errors::Result<LeafPage> {
        match self.store.read_page(page_id).await? {
            Page::Leaf(leaf) => Ok(leaf),
            Page::Internal(_) => Err(ExecuteError::wrap(format!(
                "corrupt index: expected a leaf page at {}, found an internal page",
                page_id
            ))),
            Page::Free(_) => Err(ExecuteError::wrap(format!(
                "corrupt index: expected a leaf page at {}, found a freed page",
                page_id
            ))),
        }
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
                Page::Free(_) => Err(ExecuteError::wrap(format!(
                    "corrupt index: insert descended into freed page {}",
                    page_id
                ))),
            }
        })
    }

    /// Check whether an overflow chain (for a single duplicate-key group)
    /// contains `key`. All entries in a chain share one key by
    /// construction, so this only needs to look at the first page.
    async fn overflow_contains_key(&self, overflow_id: PageId, key: &str) -> errors::Result<bool> {
        let overflow = self.read_leaf(overflow_id).await?;
        Ok(overflow.entries.first().is_some_and(|e| e.key == key))
    }

    /// The key shared by every entry in an overflow chain (all entries in a
    /// chain belong to the same duplicate-key group by construction, so
    /// only the first page needs to be read).
    async fn overflow_key(&self, overflow_id: PageId) -> errors::Result<String> {
        self.read_leaf(overflow_id)
            .await?
            .entries
            .first()
            .map(|e| e.key.clone())
            .ok_or_else(|| ExecuteError::wrap("corrupt index: empty overflow page"))
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

    /// The root-to-leaf descent taken to reach a leaf, recorded so that
    /// reclaiming an emptied leaf can fix up its parent (issue #232).
    ///
    /// `parents` holds `(internal_page_id, index_into_children)` pairs from
    /// the root down to the leaf's immediate parent, which is the last
    /// element. It is empty when the leaf is itself the root.
    async fn find_leaf_path(&self, key: &str) -> errors::Result<Option<LeafPath>> {
        let Some(root_id) = self.store.root_page_id().await? else {
            return Ok(None);
        };

        let mut parents = Vec::new();
        let mut current = root_id;
        loop {
            match self.store.read_page(current).await? {
                Page::Leaf(_) => {
                    return Ok(Some(LeafPath {
                        leaf_id: current,
                        parents,
                    }));
                }
                Page::Internal(internal) => {
                    let child_index = internal.keys.partition_point(|sep| sep.as_str() <= key);
                    parents.push((current, child_index));
                    current = internal.children[child_index];
                }
                Page::Free(_) => {
                    return Err(ExecuteError::wrap(format!(
                        "corrupt index: tree descent reached freed page {}",
                        current
                    )));
                }
            }
        }
    }

    /// Unlink a now-empty leaf from the tree and return its page to the
    /// free-list (issue #232).
    ///
    /// Three things have to stay consistent:
    /// 1. the `next_leaf` chain used by range scans, so the predecessor
    ///    leaf is re-pointed past the removed leaf;
    /// 2. the parent internal page, which loses the child pointer and the
    ///    separator key that introduced it;
    /// 3. the root, which is dropped entirely when the last leaf goes away
    ///    and collapsed when an internal root is left with a single child.
    async fn reclaim_empty_leaf(&self, path: &LeafPath) -> errors::Result<()> {
        let leaf_id = path.leaf_id;
        let leaf = self.read_leaf(leaf_id).await?;

        let Some(&(parent_id, child_index)) = path.parents.last() else {
            // The leaf is the root. An empty root leaf is left in place --
            // freeing it would mean clearing `root_page_id`, and the empty
            // tree state is already represented by that being `None`. Only
            // do so when it truly holds nothing.
            if leaf.entries.is_empty() && leaf.overflow.is_none() {
                self.store.set_root_page_id(None).await?;
                self.store.free_page(leaf_id).await?;
            }
            return Ok(());
        };

        // Re-point the previous leaf in the scan chain past this one.
        if let Some(previous_id) = self.find_previous_leaf(path).await? {
            let mut previous = self.read_leaf(previous_id).await?;
            previous.next_leaf = leaf.next_leaf;
            self.store
                .write_page(previous_id, &Page::Leaf(previous))
                .await?;
        }

        let mut parent = match self.store.read_page(parent_id).await? {
            Page::Internal(internal) => internal,
            _ => {
                return Err(ExecuteError::wrap(format!(
                    "corrupt index: expected an internal page at {}",
                    parent_id
                )));
            }
        };

        if parent.children.get(child_index) != Some(&leaf_id) {
            return Err(ExecuteError::wrap(format!(
                "corrupt index: parent {} does not point at leaf {} at index {}",
                parent_id, leaf_id, child_index
            )));
        }

        parent.children.remove(child_index);
        // `keys[i]` separates `children[i]` from `children[i + 1]`, so drop
        // the separator that introduced the removed child: the one to its
        // left, or (for the leftmost child) the one to its right.
        if !parent.keys.is_empty() {
            let key_index = if child_index == 0 { 0 } else { child_index - 1 };
            parent.keys.remove(key_index);
        }

        self.store.free_page(leaf_id).await?;

        if parent.children.is_empty() {
            // Should not happen (a parent always has >= 2 children), but
            // handle it rather than leaving a childless internal page.
            self.store
                .write_page(parent_id, &Page::Internal(parent))
                .await?;
            return Ok(());
        }

        self.store
            .write_page(parent_id, &Page::Internal(parent.clone()))
            .await?;

        // If the root collapsed to a single child, promote that child so the
        // tree does not accumulate pointless levels.
        if path.parents.len() == 1 && parent.children.len() == 1 {
            let only_child = parent.children[0];
            self.store.set_root_page_id(Some(only_child)).await?;
            self.store.free_page(parent_id).await?;
        }

        Ok(())
    }

    /// Find the leaf immediately preceding the leaf reached by `path`, i.e.
    /// the one whose `next_leaf` points at it. Returns `None` when the leaf
    /// is the leftmost one.
    ///
    /// This walks back up the recorded descent to the nearest ancestor that
    /// has a left sibling subtree, then descends that subtree's rightmost
    /// spine -- O(tree height) reads, rather than scanning the whole leaf
    /// chain.
    async fn find_previous_leaf(&self, path: &LeafPath) -> errors::Result<Option<PageId>> {
        // Nearest ancestor where we did not take the leftmost child.
        let Some((ancestor_id, child_index)) = path
            .parents
            .iter()
            .rev()
            .find(|(_, child_index)| *child_index > 0)
            .copied()
        else {
            // Every step went left: the leaf is the leftmost in the tree.
            return Ok(None);
        };

        let ancestor = match self.store.read_page(ancestor_id).await? {
            Page::Internal(internal) => internal,
            _ => {
                return Err(ExecuteError::wrap(format!(
                    "corrupt index: expected an internal page at {}",
                    ancestor_id
                )));
            }
        };

        // Descend the rightmost spine of the left sibling subtree.
        let mut current = ancestor.children[child_index - 1];
        loop {
            match self.store.read_page(current).await? {
                Page::Leaf(_) => return Ok(Some(current)),
                Page::Internal(internal) => {
                    current = *internal.children.last().ok_or_else(|| {
                        ExecuteError::wrap(format!(
                            "corrupt index: internal page {} has no children",
                            current
                        ))
                    })?;
                }
                Page::Free(_) => {
                    return Err(ExecuteError::wrap(format!(
                        "corrupt index: descent reached freed page {}",
                        current
                    )));
                }
            }
        }
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
                Page::Free(_) => {
                    return Err(ExecuteError::wrap(format!(
                        "corrupt index: tree descent reached freed page {}",
                        current
                    )));
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
                Page::Free(_) => {
                    return Err(ExecuteError::wrap(format!(
                        "corrupt index: tree descent reached freed page {}",
                        current
                    )));
                }
            }
        }
    }

    /// Collect all entries for `page_id`'s leaf plus any overflow chain.
    async fn leaf_entries_with_overflow(&self, page_id: PageId) -> errors::Result<Vec<LeafEntry>> {
        let leaf = self.read_leaf(page_id).await?;

        let mut entries = leaf.entries;
        let mut next_overflow = leaf.overflow;
        while let Some(overflow_id) = next_overflow {
            let overflow = self.read_leaf(overflow_id).await?;
            entries.extend(overflow.entries);
            next_overflow = overflow.overflow;
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
    /// found and removed.
    ///
    /// Page reclamation (issue #232): when a removal empties an overflow
    /// page or a leaf page, that page is unlinked from its chain and
    /// returned to the page store's free-list so a later insert can reuse
    /// the slot. Underfull (but non-empty) pages are deliberately left
    /// alone -- redistributing entries between siblings would require
    /// rewriting separator keys in internal pages, which is out of scope
    /// here; only fully empty pages are reclaimed.
    pub async fn remove(&self, key: &str, row_path: &str) -> errors::Result<bool> {
        let Some(path) = self.find_leaf_path(key).await? else {
            return Ok(false);
        };
        let leaf_id = path.leaf_id;

        let leaf = self.read_leaf(leaf_id).await?;

        if let Some(idx) = leaf
            .entries
            .iter()
            .position(|e| e.key == key && e.row_path == row_path)
        {
            let mut leaf = leaf;
            leaf.entries.remove(idx);

            // A leaf whose own entries are gone may still own an overflow
            // chain; pull the chain's head back into the leaf rather than
            // stranding it.
            if let Some(overflow_id) = leaf.overflow.filter(|_| leaf.entries.is_empty()) {
                let overflow = self.read_leaf(overflow_id).await?;
                leaf.entries = overflow.entries;
                leaf.overflow = overflow.overflow;
                self.store.write_page(leaf_id, &Page::Leaf(leaf)).await?;
                self.store.free_page(overflow_id).await?;
                return Ok(true);
            }

            let now_empty = leaf.entries.is_empty() && leaf.overflow.is_none();
            self.store.write_page(leaf_id, &Page::Leaf(leaf)).await?;

            if now_empty {
                self.reclaim_empty_leaf(&path).await?;
            }

            return Ok(true);
        }

        // Walk the overflow chain looking for the entry, tracking the page
        // that points at each link so an emptied page can be unlinked.
        let mut previous_id = leaf_id;
        let mut next_overflow = leaf.overflow;
        while let Some(overflow_id) = next_overflow {
            let mut overflow = self.read_leaf(overflow_id).await?;

            if let Some(idx) = overflow
                .entries
                .iter()
                .position(|e| e.key == key && e.row_path == row_path)
            {
                overflow.entries.remove(idx);

                if overflow.entries.is_empty() {
                    // Splice this page out of the chain and reclaim it.
                    let mut previous = self.read_leaf(previous_id).await?;
                    previous.overflow = overflow.overflow;
                    self.store
                        .write_page(previous_id, &Page::Leaf(previous))
                        .await?;
                    self.store.free_page(overflow_id).await?;
                } else {
                    self.store
                        .write_page(overflow_id, &Page::Leaf(overflow))
                        .await?;
                }

                return Ok(true);
            }

            previous_id = overflow_id;
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
            let next_leaf = self.read_leaf(current).await?.next_leaf;

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

    /// Assert the index is structurally sound: no live page is also on the
    /// free-list, the leaf chain terminates, and the tree contains no
    /// pointer to a freed page.
    async fn assert_index_is_consistent(idx: &PageBackedBTreeIndex) {
        // Collect the free-list.
        let mut free = std::collections::HashSet::new();
        let mut next = idx.store.free_list_head_for_test();
        while let Some(id) = next {
            assert!(free.insert(id), "free-list contains a cycle at page {}", id);
            match idx.store.read_page(id).await.unwrap() {
                Page::Free(link) => next = link,
                other => panic!("page {} on the free-list is not free: {:?}", id, other),
            }
        }

        // Walk the live tree from the root, checking nothing points at a
        // freed page.
        let mut live = std::collections::HashSet::new();
        if let Some(root) = idx.store.root_page_id().await.unwrap() {
            let mut stack = vec![root];
            while let Some(id) = stack.pop() {
                assert!(
                    !free.contains(&id),
                    "live tree references freed page {}",
                    id
                );
                assert!(live.insert(id), "tree contains page {} twice", id);

                match idx.store.read_page(id).await.unwrap() {
                    Page::Internal(internal) => {
                        assert_eq!(
                            internal.keys.len() + 1,
                            internal.children.len(),
                            "internal page {} violates keys+1 == children",
                            id
                        );
                        stack.extend(internal.children);
                    }
                    Page::Leaf(leaf) => {
                        let mut overflow = leaf.overflow;
                        while let Some(oid) = overflow {
                            assert!(
                                !free.contains(&oid),
                                "leaf {} points at freed overflow page {}",
                                id,
                                oid
                            );
                            match idx.store.read_page(oid).await.unwrap() {
                                Page::Leaf(o) => {
                                    assert!(
                                        !o.entries.is_empty(),
                                        "overflow page {} is empty but still linked",
                                        oid
                                    );
                                    overflow = o.overflow;
                                }
                                other => panic!("overflow page {} is not a leaf: {:?}", oid, other),
                            }
                        }
                    }
                    Page::Free(_) => panic!("tree descent reached freed page {}", id),
                }
            }
        }

        // The leaf chain must terminate and only visit live leaves.
        let mut seen = std::collections::HashSet::new();
        let mut cursor = idx.find_leftmost_leaf().await.unwrap();
        while let Some(id) = cursor {
            assert!(!free.contains(&id), "leaf chain reaches freed page {}", id);
            assert!(
                seen.insert(id),
                "leaf chain contains a cycle at page {}",
                id
            );
            cursor = idx.read_leaf(id).await.unwrap().next_leaf;
        }
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
        assert_eq!(
            idx.get(&int_key(750)).await.unwrap(),
            vec!["/r/750".to_string()]
        );
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
            LeafEntry {
                key: "a".to_string(),
                row_path: "1".to_string(),
            },
            LeafEntry {
                key: "b".to_string(),
                row_path: "1".to_string(),
            },
            LeafEntry {
                key: "b".to_string(),
                row_path: "2".to_string(),
            },
            LeafEntry {
                key: "b".to_string(),
                row_path: "3".to_string(),
            },
            LeafEntry {
                key: "c".to_string(),
                row_path: "1".to_string(),
            },
        ];
        let split = find_split_point(&entries).unwrap();
        assert_ne!(entries[split - 1].key, entries[split].key);
    }

    // ── Free-list reuse and empty-page reclamation (issue #232) ──

    /// Emptying the sole (root) leaf collapses the tree back to the empty
    /// state and reclaims its page.
    #[tokio::test]
    async fn removing_the_last_entry_reclaims_the_root_leaf() {
        let path = temp_path("reclaim_root_leaf.idx");
        let idx = PageBackedBTreeIndex::create(&path, "id".to_string(), false)
            .await
            .unwrap();

        idx.insert("I:001".to_string(), "/r/1".to_string())
            .await
            .unwrap();
        assert_eq!(idx.free_page_count().await.unwrap(), 0);

        assert!(idx.remove("I:001", "/r/1").await.unwrap());

        assert_eq!(idx.free_page_count().await.unwrap(), 1);
        assert_eq!(idx.scan_all().await.unwrap().len(), 0);

        // The index is still usable afterwards, and reuses the freed slot.
        let before = idx.allocated_page_count().await.unwrap();
        idx.insert("I:002".to_string(), "/r/2".to_string())
            .await
            .unwrap();
        assert_eq!(idx.allocated_page_count().await.unwrap(), before);
        assert_eq!(idx.free_page_count().await.unwrap(), 0);
        assert_eq!(idx.get("I:002").await.unwrap(), vec!["/r/2".to_string()]);
        assert_index_is_consistent(&idx).await;
    }

    /// Deleting every entry of one leaf in a multi-leaf tree frees that
    /// leaf while leaving the remaining data and the scan chain intact.
    #[tokio::test]
    async fn emptying_one_leaf_frees_it_and_preserves_range_scans() {
        let path = temp_path("reclaim_one_leaf.idx");
        let idx = PageBackedBTreeIndex::create(&path, "id".to_string(), false)
            .await
            .unwrap();

        // Enough entries to build several leaves.
        let total = 400;
        for i in 0..total {
            idx.insert(format!("I:{:05}", i), format!("/r/{}", i))
                .await
                .unwrap();
        }
        assert!(
            idx.allocated_page_count().await.unwrap() > 1,
            "expected the tree to have split into multiple pages"
        );

        // Delete a contiguous middle block wide enough to fully drain at
        // least one leaf (leaves hold ~70 entries at this page size).
        let (hole_start, hole_end) = (100, 250);
        for i in hole_start..hole_end {
            assert!(
                idx.remove(&format!("I:{:05}", i), &format!("/r/{}", i))
                    .await
                    .unwrap()
            );
        }

        assert!(
            idx.free_page_count().await.unwrap() > 0,
            "deleting a contiguous block should have emptied and freed at least one leaf"
        );

        // Surviving entries are all still reachable, in order, via the leaf
        // chain that the reclaimed leaf was spliced out of.
        let all = idx.scan_all().await.unwrap();
        assert_eq!(all.len(), (total - (hole_end - hole_start)) as usize);

        let keys: Vec<String> = all.iter().map(|e| e.key.clone()).collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(
            keys, sorted,
            "scan order must stay sorted after reclamation"
        );

        for i in 0..total {
            let expected: Vec<String> = if (hole_start..hole_end).contains(&i) {
                vec![]
            } else {
                vec![format!("/r/{}", i)]
            };
            assert_eq!(
                idx.get(&format!("I:{:05}", i)).await.unwrap(),
                expected,
                "unexpected lookup result for key {}",
                i
            );
        }

        // A bounded range scan spanning the hole also stays correct:
        // [50, 100) survives, [100, 250) was deleted, [250, 300) survives.
        let range = idx.range(Some("I:00050"), Some("I:00300")).await.unwrap();
        assert_eq!(range.len(), 100);
        assert_index_is_consistent(&idx).await;
    }

    /// Freed pages are handed back out by later inserts instead of growing
    /// the file (the core acceptance criterion of issue #232).
    #[tokio::test]
    async fn freed_pages_are_reused_by_later_inserts() {
        let path = temp_path("reuse_freed_pages.idx");
        let idx = PageBackedBTreeIndex::create(&path, "id".to_string(), false)
            .await
            .unwrap();

        for i in 0..400 {
            idx.insert(format!("I:{:05}", i), format!("/r/{}", i))
                .await
                .unwrap();
        }
        let allocated_after_fill = idx.allocated_page_count().await.unwrap();

        for i in 0..400 {
            assert!(
                idx.remove(&format!("I:{:05}", i), &format!("/r/{}", i))
                    .await
                    .unwrap()
            );
        }
        assert_eq!(idx.scan_all().await.unwrap().len(), 0);

        let freed = idx.free_page_count().await.unwrap();
        assert!(freed > 0, "clearing the index should free pages");

        // Refilling consumes the free-list before extending the file.
        for i in 0..400 {
            idx.insert(format!("J:{:05}", i), format!("/p/{}", i))
                .await
                .unwrap();
        }

        let allocated_after_refill = idx.allocated_page_count().await.unwrap();
        assert!(
            allocated_after_refill <= allocated_after_fill,
            "refilling should reuse freed pages instead of growing the file: {} -> {}",
            allocated_after_fill,
            allocated_after_refill
        );
        assert_eq!(idx.scan_all().await.unwrap().len(), 400);
        assert_index_is_consistent(&idx).await;
    }

    /// A delete-heavy churn loop must not grow the index file without bound.
    #[tokio::test]
    async fn repeated_insert_delete_churn_does_not_grow_the_file_indefinitely() {
        let path = temp_path("churn_stable.idx");
        let idx = PageBackedBTreeIndex::create(&path, "id".to_string(), false)
            .await
            .unwrap();

        let mut sizes = Vec::new();
        for round in 0..5 {
            for i in 0..200 {
                idx.insert(format!("I:{:05}", i), format!("/r/{}", i))
                    .await
                    .unwrap();
            }
            for i in 0..200 {
                assert!(
                    idx.remove(&format!("I:{:05}", i), &format!("/r/{}", i))
                        .await
                        .unwrap(),
                    "round {} failed to remove key {}",
                    round,
                    i
                );
            }
            sizes.push(idx.allocated_page_count().await.unwrap());
        }

        assert_eq!(idx.scan_all().await.unwrap().len(), 0);
        // After the first round primes the free-list, the file must stop
        // growing.
        assert_eq!(
            sizes.last().unwrap(),
            sizes.get(1).unwrap(),
            "index file kept growing across churn rounds: {:?}",
            sizes
        );
        assert_index_is_consistent(&idx).await;
    }

    /// Emptying an overflow page splices it out of the duplicate-key chain
    /// and reclaims it, without losing the rest of the group.
    #[tokio::test]
    async fn emptying_an_overflow_page_reclaims_it_and_keeps_the_group() {
        let path = temp_path("reclaim_overflow.idx");
        let idx = PageBackedBTreeIndex::create(&path, "id".to_string(), false)
            .await
            .unwrap();

        // One key with enough duplicates to spill into overflow pages.
        let dups = 400;
        for i in 0..dups {
            idx.insert("K:dup".to_string(), format!("/r/{}", i))
                .await
                .unwrap();
        }
        assert_eq!(idx.get("K:dup").await.unwrap().len(), dups as usize);

        let freed_before = idx.free_page_count().await.unwrap();

        // Remove most of the group; overflow pages should drain and be freed.
        for i in 0..(dups - 10) {
            assert!(idx.remove("K:dup", &format!("/r/{}", i)).await.unwrap());
        }

        assert!(
            idx.free_page_count().await.unwrap() > freed_before,
            "draining a duplicate group should reclaim overflow pages"
        );

        // The surviving members of the group are all still reachable.
        let remaining = idx.get("K:dup").await.unwrap();
        assert_eq!(remaining.len(), 10);
        for i in (dups - 10)..dups {
            assert!(
                remaining.contains(&format!("/r/{}", i)),
                "lost /r/{} from the duplicate group",
                i
            );
        }

        // Range scans see the same surviving entries.
        assert_eq!(idx.range(Some("K:dup"), None).await.unwrap().len(), 10);
        assert_index_is_consistent(&idx).await;
    }

    /// Reclamation must survive a reopen: the free-list lives in the
    /// superblock, so a reopened index keeps reusing freed slots.
    #[tokio::test]
    async fn free_list_persists_across_reopen() {
        let path = temp_path("free_list_reopen.idx");

        let allocated;
        {
            let idx = PageBackedBTreeIndex::create(&path, "id".to_string(), false)
                .await
                .unwrap();
            for i in 0..300 {
                idx.insert(format!("I:{:05}", i), format!("/r/{}", i))
                    .await
                    .unwrap();
            }
            for i in 0..300 {
                idx.remove(&format!("I:{:05}", i), &format!("/r/{}", i))
                    .await
                    .unwrap();
            }
            assert!(idx.free_page_count().await.unwrap() > 0);
            allocated = idx.allocated_page_count().await.unwrap();
        }

        let idx = PageBackedBTreeIndex::open(&path, "id".to_string(), false)
            .await
            .unwrap();
        assert!(
            idx.free_page_count().await.unwrap() > 0,
            "free-list must survive reopen"
        );

        for i in 0..300 {
            idx.insert(format!("J:{:05}", i), format!("/p/{}", i))
                .await
                .unwrap();
        }
        assert!(
            idx.allocated_page_count().await.unwrap() <= allocated,
            "reopened index should reuse the persisted free-list"
        );
        assert_eq!(idx.scan_all().await.unwrap().len(), 300);
        assert_index_is_consistent(&idx).await;
    }
}

#[cfg(test)]
mod invariant_integration {
    use super::*;
    use crate::engine::index::page::{InternalPage, Page};
    use crate::engine::index::page_store::PageStore;

    /// #258 end to end: a corrupt root used to panic the task in
    /// `find_leftmost_leaf`. Asserted through a real file and the public API,
    /// because the value of the decode check is that callers see an error.
    #[tokio::test]
    async fn scan_reports_a_corrupt_root_instead_of_panicking() {
        let dir = std::path::PathBuf::from("target/test_invariant_integration");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("corrupt_root.idx");
        let _ = tokio::fs::remove_file(&path).await;

        {
            let store = PageStore::create(&path, INDEX_PAGE_SIZE).await.unwrap();
            let id = store.allocate_page().await.unwrap();
            store
                .write_page(
                    id,
                    &Page::Internal(InternalPage {
                        keys: vec![],
                        children: vec![],
                    }),
                )
                .await
                .unwrap();
            store.set_root_page_id(Some(id)).await.unwrap();
        }

        let index = PageBackedBTreeIndex::open(&path, "c".to_string(), false)
            .await
            .unwrap();
        let error = index
            .scan_all()
            .await
            .expect_err("a corrupt root must surface as an error");
        assert!(
            error.to_string().contains("corrupt internal page"),
            "got: {}",
            error
        );

        let _ = tokio::fs::remove_file(&path).await;
    }
}
