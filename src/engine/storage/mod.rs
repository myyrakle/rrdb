pub mod page;

pub use page::*;

pub type PageId = u64;
pub type SlotId = u16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageError {
    NoSpace,
    InvalidSlot,
    RowTooLarge,
}
