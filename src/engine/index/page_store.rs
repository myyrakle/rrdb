//! Fixed-size page storage for the page-backed B+tree index (issue #230).
//!
//! On-disk layout: a small fixed-size superblock, followed by an array of
//! `page_size`-byte page slots addressed by `page_id` (see `page.rs` for the
//! per-page encoding). Pages are allocated from a free-list when one is
//! available (issue #232), falling back to a bump allocator otherwise.
//! Freed pages keep their file slot but are linked into a singly-linked
//! free-list whose head lives in the superblock; each freed slot stores the
//! id of the next free page (`Page::Free`).
//!
//! File IO is synchronous (`std::fs::File` guarded by a `std::sync::Mutex`)
//! run inline inside `async fn`s. This briefly blocks the executor thread on
//! each page read/write; a follow-up could move this onto
//! `tokio::task::spawn_blocking` to avoid that. Because every mutating
//! operation only reads/writes the handful of pages it touches (not the
//! whole file), this is still a large improvement over the previous
//! full-file snapshot rewrite on every mutation.

use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::errors;
use crate::errors::execute_error::ExecuteError;

use super::page::{self, Page, PageId};

const MAGIC: [u8; 4] = *b"RIDX";
/// Bumped to 2 for the free-list head added in issue #232. Version 1 files
/// decode with `free_list_head = None` (see `read_superblock`), so existing
/// index files keep working and simply start with an empty free-list.
const VERSION: u16 = 2;
/// Fixed size of the superblock region at the start of the file. Must be
/// large enough to hold the bincode-encoded `Superblock` below; checked by a
/// test.
pub const SUPERBLOCK_SIZE: usize = 64;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Superblock {
    magic: [u8; 4],
    version: u16,
    page_size: u32,
    root_page_id: Option<PageId>,
    next_page_id: PageId,
    /// Head of the free-page list, or `None` when no pages are free.
    free_list_head: Option<PageId>,
}

/// The superblock layout as written by version 1 (before the free-list in
/// issue #232). Used to migrate existing index files on open.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct SuperblockV1 {
    magic: [u8; 4],
    version: u16,
    page_size: u32,
    root_page_id: Option<PageId>,
    next_page_id: PageId,
}

/// A single index's page-backed storage file.
pub struct PageStore {
    file: Mutex<std::fs::File>,
    page_size: usize,
}

impl PageStore {
    /// Create a brand new, empty page store at `path` with the given
    /// `page_size`. Errors if a file already exists at `path`.
    pub async fn create(path: &Path, page_size: usize) -> errors::Result<Self> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| ExecuteError::wrap(format!("failed to create index file: {}", e)))?;

        let store = PageStore {
            file: Mutex::new(file),
            page_size,
        };

        store
            .write_superblock(&Superblock {
                magic: MAGIC,
                version: VERSION,
                page_size: page_size as u32,
                root_page_id: None,
                next_page_id: 0,
                free_list_head: None,
            })
            .await?;

        Ok(store)
    }

    /// Open an existing page store, reading `page_size` back from its
    /// superblock.
    pub async fn open(path: &Path) -> errors::Result<Self> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|e| ExecuteError::wrap(format!("failed to open index file: {}", e)))?;

        let store = PageStore {
            file: Mutex::new(file),
            page_size: page::INDEX_PAGE_SIZE,
        };

        let sb = store.read_superblock()?;
        if sb.magic != MAGIC {
            return Err(ExecuteError::wrap("index file has invalid magic bytes"));
        }
        // The magic only rules out files that are not index files at all. A
        // file written by a different format version is the case actually
        // worth guarding: page_size is read straight back out of the
        // superblock and drives every offset, so accepting a foreign version
        // means writing pages at the wrong place in a file that is otherwise
        // valid.
        if sb.version != VERSION {
            return Err(ExecuteError::wrap(format!(
                "index file version {} is not supported (expected {})",
                sb.version, VERSION
            )));
        }

        Ok(PageStore {
            file: store.file,
            page_size: sb.page_size as usize,
        })
    }

    pub fn page_size(&self) -> usize {
        self.page_size
    }

    fn read_superblock(&self) -> errors::Result<Superblock> {
        let mut buf = vec![0u8; SUPERBLOCK_SIZE];
        let mut file = self.file.lock().unwrap();
        file.seek(SeekFrom::Start(0))
            .map_err(|e| ExecuteError::wrap(format!("failed to seek to superblock: {}", e)))?;
        file.read_exact(&mut buf)
            .map_err(|e| ExecuteError::wrap(format!("failed to read superblock: {}", e)))?;

        // Version 1 files predate `free_list_head`; decode them with the old
        // layout and treat their free-list as empty.
        let version = u16::from_le_bytes(
            buf.get(4..6)
                .and_then(|b| b.try_into().ok())
                .ok_or_else(|| ExecuteError::wrap("superblock too small to contain a version"))?,
        );

        if version < 2 {
            let sb: SuperblockV1 = bincode::deserialize(&buf).map_err(|e| {
                ExecuteError::wrap(format!("failed to decode v1 superblock: {}", e))
            })?;

            return Ok(Superblock {
                magic: sb.magic,
                version: sb.version,
                page_size: sb.page_size,
                root_page_id: sb.root_page_id,
                next_page_id: sb.next_page_id,
                free_list_head: None,
            });
        }

        bincode::deserialize(&buf)
            .map_err(|e| ExecuteError::wrap(format!("failed to decode superblock: {}", e)))
    }

    async fn write_superblock(&self, sb: &Superblock) -> errors::Result<()> {
        let mut encoded = bincode::serialize(sb)
            .map_err(|e| ExecuteError::wrap(format!("failed to encode superblock: {}", e)))?;
        if encoded.len() > SUPERBLOCK_SIZE {
            return Err(ExecuteError::wrap("superblock exceeds reserved size"));
        }
        encoded.resize(SUPERBLOCK_SIZE, 0);

        let mut file = self.file.lock().unwrap();
        file.seek(SeekFrom::Start(0))
            .map_err(|e| ExecuteError::wrap(format!("failed to seek to superblock: {}", e)))?;
        file.write_all(&encoded)
            .map_err(|e| ExecuteError::wrap(format!("failed to write superblock: {}", e)))?;
        file.sync_data()
            .map_err(|e| ExecuteError::wrap(format!("failed to sync superblock: {}", e)))?;

        Ok(())
    }

    fn page_offset(&self, page_id: PageId) -> u64 {
        SUPERBLOCK_SIZE as u64 + (page_id as u64) * (self.page_size as u64)
    }

    /// Read a page by id. The page must have been previously written via
    /// `write_page` (or `allocate_page` + `write_page`).
    pub async fn read_page(&self, page_id: PageId) -> errors::Result<Page> {
        let mut buf = vec![0u8; self.page_size];
        {
            let mut file = self.file.lock().unwrap();
            file.seek(SeekFrom::Start(self.page_offset(page_id)))
                .map_err(|e| ExecuteError::wrap(format!("failed to seek to page: {}", e)))?;
            file.read_exact(&mut buf).map_err(|e| {
                ExecuteError::wrap(format!("failed to read page {}: {}", page_id, e))
            })?;
        }
        page::decode_page(&buf)
    }

    /// Write a page at `page_id`, overwriting only that page's fixed slot.
    pub async fn write_page(&self, page_id: PageId, page: &Page) -> errors::Result<()> {
        let encoded = page::encode_page(page, self.page_size)?;

        let mut file = self.file.lock().unwrap();
        file.seek(SeekFrom::Start(self.page_offset(page_id)))
            .map_err(|e| ExecuteError::wrap(format!("failed to seek to page: {}", e)))?;
        file.write_all(&encoded)
            .map_err(|e| ExecuteError::wrap(format!("failed to write page {}: {}", page_id, e)))?;
        file.sync_data()
            .map_err(|e| ExecuteError::wrap(format!("failed to sync page {}: {}", page_id, e)))?;

        Ok(())
    }

    /// Allocate a page id, reusing the head of the free-list when one is
    /// available and falling back to the bump allocator otherwise
    /// (issue #232).
    pub async fn allocate_page(&self) -> errors::Result<PageId> {
        let mut sb = self.read_superblock()?;

        if let Some(free_id) = sb.free_list_head {
            // Pop the head of the free-list; its slot stores the next link.
            let next_free = match self.read_page(free_id).await? {
                Page::Free(next) => next,
                _ => {
                    return Err(ExecuteError::wrap(format!(
                        "corrupt index: page {} is on the free-list but is not a free page",
                        free_id
                    )));
                }
            };

            sb.free_list_head = next_free;
            self.write_superblock(&sb).await?;
            return Ok(free_id);
        }

        let id = sb.next_page_id;
        sb.next_page_id += 1;
        self.write_superblock(&sb).await?;
        Ok(id)
    }

    /// Return `page_id` to the free-list so a later `allocate_page` can
    /// reuse it. The caller must guarantee no page still references it.
    ///
    /// The free page is written before the superblock head is updated, so a
    /// crash between the two leaves the page merely orphaned (leaked) rather
    /// than producing a free-list that points at live data.
    pub async fn free_page(&self, page_id: PageId) -> errors::Result<()> {
        let mut sb = self.read_superblock()?;

        self.write_page(page_id, &Page::Free(sb.free_list_head))
            .await?;

        sb.free_list_head = Some(page_id);
        self.write_superblock(&sb).await
    }

    /// Number of pages currently on the free-list. Test/diagnostic helper.
    pub async fn free_page_count(&self) -> errors::Result<usize> {
        let mut count = 0;
        let mut next = self.read_superblock()?.free_list_head;

        while let Some(page_id) = next {
            match self.read_page(page_id).await? {
                Page::Free(link) => {
                    count += 1;
                    next = link;
                }
                _ => {
                    return Err(ExecuteError::wrap(format!(
                        "corrupt index: page {} is on the free-list but is not a free page",
                        page_id
                    )));
                }
            }
        }

        Ok(count)
    }

    /// Highest page id ever handed out by the bump allocator (i.e. the
    /// number of page slots in the file). Test/diagnostic helper.
    pub async fn allocated_page_count(&self) -> errors::Result<PageId> {
        Ok(self.read_superblock()?.next_page_id)
    }

    /// Head of the free-list. Test helper for structural assertions.
    #[cfg(test)]
    pub(crate) fn free_list_head_for_test(&self) -> Option<PageId> {
        self.read_superblock().unwrap().free_list_head
    }

    pub async fn root_page_id(&self) -> errors::Result<Option<PageId>> {
        Ok(self.read_superblock()?.root_page_id)
    }

    pub async fn set_root_page_id(&self, root: Option<PageId>) -> errors::Result<()> {
        let mut sb = self.read_superblock()?;
        sb.root_page_id = root;
        self.write_superblock(&sb).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::index::page::{InternalPage, LeafEntry, LeafPage};

    fn temp_path(name: &str) -> std::path::PathBuf {
        let dir = std::path::PathBuf::from("target/test_page_store");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        let _ = std::fs::remove_file(&path);
        path
    }

    #[test]
    fn superblock_fits_in_its_reserved_size() {
        let sb = Superblock {
            magic: MAGIC,
            version: VERSION,
            page_size: page::INDEX_PAGE_SIZE as u32,
            root_page_id: Some(12345),
            next_page_id: 67890,
            free_list_head: Some(4242),
        };
        let encoded = bincode::serialize(&sb).unwrap();
        assert!(encoded.len() <= SUPERBLOCK_SIZE);
    }

    /// `read_superblock` locates the version field by byte offset to decide
    /// which layout to decode, so that offset must match the encoding.
    #[test]
    fn version_field_sits_at_the_expected_byte_offset() {
        let sb = Superblock {
            magic: MAGIC,
            version: VERSION,
            page_size: page::INDEX_PAGE_SIZE as u32,
            root_page_id: None,
            next_page_id: 0,
            free_list_head: None,
        };
        let encoded = bincode::serialize(&sb).unwrap();

        assert_eq!(&encoded[0..4], &MAGIC);
        assert_eq!(
            u16::from_le_bytes(encoded[4..6].try_into().unwrap()),
            VERSION
        );
    }

    /// Index files written before issue #232 must still open, with an empty
    /// free-list.
    #[tokio::test]
    async fn opens_a_version_1_superblock_with_an_empty_free_list() {
        let path = temp_path("v1_compat.idx");

        // Hand-write a v1 file: v1 superblock + one leaf page.
        {
            let mut encoded = bincode::serialize(&SuperblockV1 {
                magic: MAGIC,
                version: 1,
                page_size: 256,
                root_page_id: Some(0),
                next_page_id: 1,
            })
            .unwrap();
            encoded.resize(SUPERBLOCK_SIZE, 0);

            let leaf = page::encode_page(
                &Page::Leaf(LeafPage {
                    entries: vec![LeafEntry {
                        key: "I:001".to_string(),
                        row_path: "/r/1".to_string(),
                    }],
                    next_leaf: None,
                    overflow: None,
                }),
                256,
            )
            .unwrap();

            let mut bytes = encoded;
            bytes.extend_from_slice(&leaf);
            std::fs::write(&path, bytes).unwrap();
        }

        let store = PageStore::open(&path).await.unwrap();
        assert_eq!(store.page_size(), 256);
        assert_eq!(store.root_page_id().await.unwrap(), Some(0));
        assert_eq!(store.free_page_count().await.unwrap(), 0);

        // A fresh allocation still works and does not collide with page 0.
        assert_eq!(store.allocate_page().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn allocates_and_persists_pages_across_reopen() {
        let path = temp_path("alloc_reopen.idx");

        let leaf_id;
        {
            let store = PageStore::create(&path, 256).await.unwrap();
            assert_eq!(store.root_page_id().await.unwrap(), None);

            leaf_id = store.allocate_page().await.unwrap();
            let page = Page::Leaf(LeafPage {
                entries: vec![LeafEntry {
                    key: "I:001".to_string(),
                    row_path: "/r/1".to_string(),
                }],
                next_leaf: None,
                overflow: None,
            });
            store.write_page(leaf_id, &page).await.unwrap();
            store.set_root_page_id(Some(leaf_id)).await.unwrap();
        }

        {
            let store = PageStore::open(&path).await.unwrap();
            assert_eq!(store.page_size(), 256);
            assert_eq!(store.root_page_id().await.unwrap(), Some(leaf_id));

            let page = store.read_page(leaf_id).await.unwrap();
            match page {
                Page::Leaf(leaf) => {
                    assert_eq!(leaf.entries.len(), 1);
                    assert_eq!(leaf.entries[0].key, "I:001");
                }
                _ => panic!("expected a leaf page"),
            }
        }
    }

    #[tokio::test]
    async fn writing_one_page_does_not_disturb_its_neighbors() {
        let path = temp_path("isolated_writes.idx");
        let store = PageStore::create(&path, 256).await.unwrap();

        let a = store.allocate_page().await.unwrap();
        let b = store.allocate_page().await.unwrap();

        store
            .write_page(
                a,
                &Page::Internal(InternalPage {
                    keys: vec!["I:010".to_string()],
                    children: vec![1, 2],
                }),
            )
            .await
            .unwrap();
        store
            .write_page(
                b,
                &Page::Leaf(LeafPage {
                    entries: vec![LeafEntry {
                        key: "I:020".to_string(),
                        row_path: "/r/2".to_string(),
                    }],
                    next_leaf: None,
                    overflow: None,
                }),
            )
            .await
            .unwrap();

        match store.read_page(a).await.unwrap() {
            Page::Internal(p) => assert_eq!(p.keys, vec!["I:010".to_string()]),
            _ => panic!("expected internal page at a"),
        }
        match store.read_page(b).await.unwrap() {
            Page::Leaf(p) => assert_eq!(p.entries[0].row_path, "/r/2"),
            _ => panic!("expected leaf page at b"),
        }
    }

    #[tokio::test]
    async fn allocate_page_ids_are_monotonically_increasing() {
        let path = temp_path("monotonic_ids.idx");
        let store = PageStore::create(&path, 256).await.unwrap();

        let mut ids = Vec::new();
        for _ in 0..3 {
            ids.push(store.allocate_page().await.unwrap());
        }
        assert_eq!(ids, vec![0, 1, 2]);
    }

    /// Rewrite the superblock of an existing store, keeping everything the
    /// caller cannot see (page contents) intact.
    fn rewrite_superblock(path: &std::path::Path, mutate: impl FnOnce(&mut Superblock)) {
        let mut raw = std::fs::read(path).unwrap();
        let mut sb: Superblock = bincode::deserialize(&raw[..SUPERBLOCK_SIZE]).unwrap();
        mutate(&mut sb);
        let mut encoded = bincode::serialize(&sb).unwrap();
        encoded.resize(SUPERBLOCK_SIZE, 0);
        raw[..SUPERBLOCK_SIZE].copy_from_slice(&encoded);
        std::fs::write(path, &raw).unwrap();
    }

    #[tokio::test]
    async fn open_rejects_an_unsupported_format_version() {
        let path = temp_path("foreign_version.idx");
        let store = PageStore::create(&path, page::INDEX_PAGE_SIZE).await.unwrap();
        drop(store);

        rewrite_superblock(&path, |sb| sb.version = VERSION + 1);

        let error = match PageStore::open(&path).await {
            Ok(_) => panic!("a store written by a different format version was accepted"),
            Err(error) => error.to_string(),
        };
        assert!(
            error.contains("version"),
            "the error should name the version, got: {}",
            error
        );
    }

    /// The guard rail for the check above: rejecting more is only a fix if
    /// everything legitimate still opens. A store this build wrote must round
    /// trip, contents included.
    #[tokio::test]
    async fn open_still_accepts_a_store_written_by_this_version() {
        let path = temp_path("same_version.idx");
        let store = PageStore::create(&path, page::INDEX_PAGE_SIZE).await.unwrap();
        let id = store.allocate_page().await.unwrap();
        store
            .write_page(
                id,
                &Page::Leaf(LeafPage {
                    entries: vec![LeafEntry {
                        key: "k".to_string(),
                        row_path: "/r/1".to_string(),
                    }],
                    next_leaf: None,
                    overflow: None,
                }),
            )
            .await
            .unwrap();
        drop(store);

        let reopened = PageStore::open(&path).await.unwrap();
        assert_eq!(reopened.page_size(), page::INDEX_PAGE_SIZE);
        match reopened.read_page(id).await.unwrap() {
            Page::Leaf(leaf) => assert_eq!(leaf.entries[0].row_path, "/r/1"),
            other => panic!("expected the leaf page back, got {:?}", other),
        }
    }

    /// Why the version matters rather than being cosmetic: `page_size` comes
    /// straight out of the superblock and drives every offset. Without the
    /// check, a foreign version whose page size differs writes pages at the
    /// wrong offsets into an otherwise valid file.
    #[tokio::test]
    async fn a_foreign_version_would_write_pages_at_the_wrong_offset() {
        let path = temp_path("wrong_offset.idx");
        let store = PageStore::create(&path, page::INDEX_PAGE_SIZE).await.unwrap();
        let id = store.allocate_page().await.unwrap();
        store
            .write_page(
                id,
                &Page::Leaf(LeafPage {
                    entries: vec![LeafEntry {
                        key: "k".to_string(),
                        row_path: "/r/1".to_string(),
                    }],
                    next_leaf: None,
                    overflow: None,
                }),
            )
            .await
            .unwrap();
        drop(store);

        rewrite_superblock(&path, |sb| {
            sb.version = VERSION + 1;
            sb.page_size = (page::INDEX_PAGE_SIZE * 2) as u32;
        });

        assert!(
            PageStore::open(&path).await.is_err(),
            "the version check has to catch this before the page size is trusted"
        );
    }
}
