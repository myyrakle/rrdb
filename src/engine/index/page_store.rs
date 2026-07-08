//! Fixed-size page storage for the page-backed B+tree index (issue #230).
//!
//! On-disk layout: a small fixed-size superblock, followed by an array of
//! `page_size`-byte page slots addressed by `page_id` (see `page.rs` for the
//! per-page encoding). Pages are allocated with a simple bump allocator --
//! there is no free-list reuse of dropped pages in this MVP.
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
const VERSION: u16 = 1;
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
        file.flush()
            .map_err(|e| ExecuteError::wrap(format!("failed to flush superblock: {}", e)))?;

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
            file.read_exact(&mut buf)
                .map_err(|e| ExecuteError::wrap(format!("failed to read page {}: {}", page_id, e)))?;
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
        file.flush()
            .map_err(|e| ExecuteError::wrap(format!("failed to flush page {}: {}", page_id, e)))?;

        Ok(())
    }

    /// Allocate a fresh page id (bump allocator; no reuse of freed pages).
    pub async fn allocate_page(&self) -> errors::Result<PageId> {
        let mut sb = self.read_superblock()?;
        let id = sb.next_page_id;
        sb.next_page_id += 1;
        self.write_superblock(&sb).await?;
        Ok(id)
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
        };
        let encoded = bincode::serialize(&sb).unwrap();
        assert!(encoded.len() <= SUPERBLOCK_SIZE);
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
}
