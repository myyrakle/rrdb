pub mod page;
pub mod table_heap;

pub type PageId = u64;
pub type SlotId = u16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageError {
    NoSpace,
    InvalidSlot,
    RowTooLarge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RowId {
    pub page_id: PageId,
    pub slot_id: SlotId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeapError {
    InvalidPage,
    Page(PageError),
}

impl From<PageError> for HeapError {
    fn from(error: PageError) -> Self {
        Self::Page(error)
    }
}

pub use page::*;
pub use table_heap::*;
