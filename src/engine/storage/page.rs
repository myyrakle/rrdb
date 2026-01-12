use crate::engine::storage::{PageError, PageId, SlotId};

pub const PAGE_SIZE: usize = 8 * 1024;

const HEADER_SIZE: usize = 16;
const SLOT_SIZE: usize = 8;
const MAX_SLOTS: usize = (PAGE_SIZE - HEADER_SIZE) / SLOT_SIZE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageHeader {
    pub page_id: PageId,
    pub slot_count: u16,
    pub free_start: u16,
    pub free_end: u16,
}

impl PageHeader {
    pub fn new(page_id: PageId) -> Self {
        Self {
            page_id,
            slot_count: 0,
            free_start: HEADER_SIZE as u16,
            free_end: PAGE_SIZE as u16,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Slot {
    offset: u16,
    len: u16,
    live: bool,
}

const EMPTY_SLOT: Slot = Slot {
    offset: 0,
    len: 0,
    live: false,
};

pub struct Page {
    header: PageHeader,
    slots: [Slot; MAX_SLOTS],
    data: Vec<u8>,
}

impl Page {
    pub fn new(page_id: PageId) -> Self {
        Self {
            header: PageHeader::new(page_id),
            slots: [EMPTY_SLOT; MAX_SLOTS],
            data: vec![0; PAGE_SIZE],
        }
    }

    pub fn insert(&mut self, payload: &[u8]) -> Result<SlotId, PageError> {
        if payload.len() > u16::MAX as usize {
            return Err(PageError::RowTooLarge);
        }
        if self.header.slot_count as usize >= MAX_SLOTS {
            return Err(PageError::NoSpace);
        }

        let needed = SLOT_SIZE + payload.len();
        if needed > self.free_space() {
            return Err(PageError::NoSpace);
        }

        let payload_len = payload.len() as u16;
        let new_free_end = self
            .header
            .free_end
            .checked_sub(payload_len)
            .ok_or(PageError::NoSpace)?;
        let start = new_free_end as usize;
        let end = self.header.free_end as usize;
        self.data[start..end].copy_from_slice(payload);

        let slot_id = self.header.slot_count;
        self.slots[slot_id as usize] = Slot {
            offset: new_free_end,
            len: payload_len,
            live: true,
        };

        self.header.slot_count += 1;
        self.header.free_start += SLOT_SIZE as u16;
        self.header.free_end = new_free_end;

        Ok(slot_id)
    }

    pub fn read(&self, slot_id: SlotId) -> Result<Option<&[u8]>, PageError> {
        if slot_id >= self.header.slot_count {
            return Err(PageError::InvalidSlot);
        }
        let slot = self.slots[slot_id as usize];
        if !slot.live {
            return Ok(None);
        }

        let start = slot.offset as usize;
        let end = start + slot.len as usize;
        Ok(Some(&self.data[start..end]))
    }

    pub fn delete(&mut self, slot_id: SlotId) -> Result<(), PageError> {
        if slot_id >= self.header.slot_count {
            return Err(PageError::InvalidSlot);
        }
        self.slots[slot_id as usize].live = false;
        Ok(())
    }

    fn free_space(&self) -> usize {
        self.header.free_end.saturating_sub(self.header.free_start) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_read_delete_roundtrip() {
        let mut page = Page::new(1);
        let slot = page.insert(b"hello").expect("insert failed");
        assert_eq!(page.read(slot).unwrap().unwrap(), b"hello");
        page.delete(slot).expect("delete failed");
        assert!(page.read(slot).unwrap().is_none());
    }
}
