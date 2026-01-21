use super::{PageError, PageId, SlotId};

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

    pub fn slot_count(&self) -> u16 {
        self.header.slot_count
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

    pub fn update(&mut self, slot_id: SlotId, payload: &[u8]) -> Result<(), PageError> {
        if payload.len() > u16::MAX as usize {
            return Err(PageError::RowTooLarge);
        }
        if slot_id >= self.header.slot_count {
            return Err(PageError::InvalidSlot);
        }

        let mut slot = self.slots[slot_id as usize];
        if !slot.live {
            return Err(PageError::InvalidSlot);
        }
        if payload.len() > slot.len as usize {
            return Err(PageError::NoSpace);
        }

        let start = slot.offset as usize;
        let end = start + payload.len();
        self.data[start..end].copy_from_slice(payload);
        slot.len = payload.len() as u16;
        self.slots[slot_id as usize] = slot;
        Ok(())
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

pub struct PageCodec;

impl PageCodec {
    pub fn encode(page: &Page) -> [u8; PAGE_SIZE] {
        let mut bytes = [0u8; PAGE_SIZE];
        if page.data.len() == PAGE_SIZE {
            bytes.copy_from_slice(&page.data);
        }

        bytes[0..8].copy_from_slice(&page.header.page_id.to_le_bytes());
        bytes[8..10].copy_from_slice(&page.header.slot_count.to_le_bytes());
        bytes[10..12].copy_from_slice(&page.header.free_start.to_le_bytes());
        bytes[12..14].copy_from_slice(&page.header.free_end.to_le_bytes());

        // 패딩 비트
        bytes[14..16].copy_from_slice(&0u16.to_le_bytes());

        let slot_count = page.header.slot_count as usize;
        for index in 0..slot_count {
            let slot = page.slots[index];
            let base = HEADER_SIZE + index * SLOT_SIZE;
            bytes[base..base + 2].copy_from_slice(&slot.offset.to_le_bytes());
            bytes[base + 2..base + 4].copy_from_slice(&slot.len.to_le_bytes());
            bytes[base + 4..base + 6].copy_from_slice(&(slot.live as u16).to_le_bytes());

            // 패딩 비트
            bytes[base + 6..base + 8].copy_from_slice(&0u16.to_le_bytes());
        }

        bytes
    }

    pub fn decode(bytes: &[u8; PAGE_SIZE]) -> Page {
        let page_id = u64::from_le_bytes(bytes[0..8].try_into().unwrap());
        let slot_count = u16::from_le_bytes(bytes[8..10].try_into().unwrap());
        let free_start = u16::from_le_bytes(bytes[10..12].try_into().unwrap());
        let free_end = u16::from_le_bytes(bytes[12..14].try_into().unwrap());
        let header = PageHeader {
            page_id,
            slot_count,
            free_start,
            free_end,
        };

        let mut slots = [EMPTY_SLOT; MAX_SLOTS];
        let max = usize::min(slot_count as usize, MAX_SLOTS);
        for index in 0..max {
            let base = HEADER_SIZE + index * SLOT_SIZE;
            let offset = u16::from_le_bytes(bytes[base..base + 2].try_into().unwrap());
            let len = u16::from_le_bytes(bytes[base + 2..base + 4].try_into().unwrap());
            let live = u16::from_le_bytes(bytes[base + 4..base + 6].try_into().unwrap()) != 0;
            slots[index] = Slot { offset, len, live };
        }

        Page {
            header,
            slots,
            data: bytes.to_vec(),
        }
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

    #[test]
    fn encode_decode_roundtrip() {
        let mut page = Page::new(7);
        let slot = page.insert(b"hello").expect("insert failed");

        let bytes = PageCodec::encode(&page);
        let decoded = PageCodec::decode(&bytes);

        assert_eq!(decoded.header.page_id, page.header.page_id);
        assert_eq!(decoded.read(slot).unwrap().unwrap(), b"hello");
    }
}
