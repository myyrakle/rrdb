//! Fixed-size disk pages for the page-backed B+tree index (issue #230).
//!
//! Each page is encoded into a fixed-size byte buffer so that pages can be
//! addressed on disk by `offset = superblock_size + page_id * page_size`
//! (see `page_store.rs`). The encoding is `[tag:u8][len:u32 LE][payload][zero padding]`.

use serde::{Deserialize, Serialize};

use crate::errors;
use crate::errors::execute_error::ExecuteError;

/// Default page size for index files, in bytes.
pub const INDEX_PAGE_SIZE: usize = 4096;

/// Page identifier: an offset slot within an index file.
pub type PageId = u32;

const TAG_LEN: usize = 1;
const LEN_PREFIX_LEN: usize = 4;
/// Bytes consumed by the tag+length header before the actual payload.
pub const PAGE_ENCODING_OVERHEAD: usize = TAG_LEN + LEN_PREFIX_LEN;

const TAG_LEAF: u8 = 1;
const TAG_INTERNAL: u8 = 2;

/// A single key -> row_path mapping stored in a leaf page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeafEntry {
    pub key: String,
    pub row_path: String,
}

/// A leaf page: sorted entries plus a pointer to the next leaf (for range
/// scans) and an optional overflow page holding additional entries for the
/// last duplicate-key group when it does not fit in this page (see
/// `page_btree.rs` for the overflow-chain policy).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct LeafPage {
    pub entries: Vec<LeafEntry>,
    pub next_leaf: Option<PageId>,
    pub overflow: Option<PageId>,
}

/// An internal (directory) page: separator keys and child page ids.
/// Invariant: `keys.len() + 1 == children.len()`.
/// Child `children[i]` holds keys in `[keys[i-1], keys[i])`, with
/// `children[0]` holding everything below `keys[0]` and `children[last]`
/// holding everything from `keys[last-1]` onward.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct InternalPage {
    pub keys: Vec<String>,
    pub children: Vec<PageId>,
}

/// A decoded on-disk page.
#[derive(Debug, Clone, PartialEq)]
pub enum Page {
    Leaf(LeafPage),
    Internal(InternalPage),
}

/// Encode `page` into a `page_size`-byte buffer.
///
/// Returns an error if the serialized payload (plus the tag+length header)
/// does not fit within `page_size` -- callers are expected to split the page
/// before this happens; see `page_btree.rs`.
pub fn encode_page(page: &Page, page_size: usize) -> errors::Result<Vec<u8>> {
    let (tag, payload) = match page {
        Page::Leaf(leaf) => (
            TAG_LEAF,
            bincode::serialize(leaf)
                .map_err(|e| ExecuteError::wrap(format!("failed to encode leaf page: {}", e)))?,
        ),
        Page::Internal(internal) => (
            TAG_INTERNAL,
            bincode::serialize(internal).map_err(|e| {
                ExecuteError::wrap(format!("failed to encode internal page: {}", e))
            })?,
        ),
    };

    if payload.len() + PAGE_ENCODING_OVERHEAD > page_size {
        return Err(ExecuteError::wrap(format!(
            "page payload of {} bytes (+{} byte header) exceeds page size {} bytes",
            payload.len(),
            PAGE_ENCODING_OVERHEAD,
            page_size
        )));
    }

    let mut buf = Vec::with_capacity(page_size);
    buf.push(tag);
    buf.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    buf.extend_from_slice(&payload);
    buf.resize(page_size, 0);
    Ok(buf)
}

/// Decode a page previously produced by `encode_page`.
pub fn decode_page(buf: &[u8]) -> errors::Result<Page> {
    if buf.len() < PAGE_ENCODING_OVERHEAD {
        return Err(ExecuteError::wrap(
            "page buffer too small to contain a header",
        ));
    }

    let tag = buf[0];
    let len = u32::from_le_bytes(buf[TAG_LEN..PAGE_ENCODING_OVERHEAD].try_into().unwrap()) as usize;
    let payload_end = PAGE_ENCODING_OVERHEAD + len;

    if payload_end > buf.len() {
        return Err(ExecuteError::wrap(
            "page payload length exceeds buffer size",
        ));
    }

    let payload = &buf[PAGE_ENCODING_OVERHEAD..payload_end];

    match tag {
        TAG_LEAF => {
            let leaf: LeafPage = bincode::deserialize(payload)
                .map_err(|e| ExecuteError::wrap(format!("failed to decode leaf page: {}", e)))?;
            Ok(Page::Leaf(leaf))
        }
        TAG_INTERNAL => {
            let internal: InternalPage = bincode::deserialize(payload).map_err(|e| {
                ExecuteError::wrap(format!("failed to decode internal page: {}", e))
            })?;
            Ok(Page::Internal(internal))
        }
        other => Err(ExecuteError::wrap(format!("unknown page tag {}", other))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(entries: Vec<(&str, &str)>, next_leaf: Option<PageId>) -> LeafPage {
        LeafPage {
            entries: entries
                .into_iter()
                .map(|(k, v)| LeafEntry {
                    key: k.to_string(),
                    row_path: v.to_string(),
                })
                .collect(),
            next_leaf,
            overflow: None,
        }
    }

    #[test]
    fn roundtrips_a_leaf_page_preserving_order_and_next_pointer() {
        let original = leaf(
            vec![("I:0001", "/r/1"), ("I:0002", "/r/2"), ("I:0002", "/r/3")],
            Some(7),
        );

        let encoded = encode_page(&Page::Leaf(original.clone()), INDEX_PAGE_SIZE).unwrap();
        assert_eq!(encoded.len(), INDEX_PAGE_SIZE);

        let decoded = decode_page(&encoded).unwrap();
        assert_eq!(decoded, Page::Leaf(original));
    }

    #[test]
    fn roundtrips_an_internal_page_preserving_keys_and_children() {
        let original = InternalPage {
            keys: vec!["I:0010".to_string(), "I:0020".to_string()],
            children: vec![1, 2, 3],
        };

        let encoded = encode_page(&Page::Internal(original.clone()), INDEX_PAGE_SIZE).unwrap();
        assert_eq!(encoded.len(), INDEX_PAGE_SIZE);

        let decoded = decode_page(&encoded).unwrap();
        assert_eq!(decoded, Page::Internal(original));
    }

    #[test]
    fn rejects_a_page_whose_payload_does_not_fit_in_the_page_size() {
        let huge = leaf(
            (0..1000)
                .map(|i| {
                    (
                        Box::leak(format!("I:{:016}", i).into_boxed_str()) as &str,
                        "/r/x",
                    )
                })
                .collect(),
            None,
        );

        let result = encode_page(&Page::Leaf(huge), INDEX_PAGE_SIZE);
        assert!(result.is_err());
    }

    #[test]
    fn accepts_a_small_page_size_when_payload_fits() {
        let small = leaf(vec![("I:01", "/r/1")], None);
        let encoded = encode_page(&Page::Leaf(small.clone()), 128).unwrap();
        assert_eq!(encoded.len(), 128);
        assert_eq!(decode_page(&encoded).unwrap(), Page::Leaf(small));
    }
}
