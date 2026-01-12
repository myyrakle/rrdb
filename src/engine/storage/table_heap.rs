use super::{HeapError, Page, PageError, PageId, RowId};

pub struct TableHeap {
    pages: Vec<Page>,
}

impl TableHeap {
    pub fn new() -> Self {
        Self { pages: Vec::new() }
    }

    pub fn insert(&mut self, payload: &[u8]) -> Result<RowId, HeapError> {
        for (index, page) in self.pages.iter_mut().enumerate() {
            match page.insert(payload) {
                Ok(slot_id) => {
                    return Ok(RowId {
                        page_id: index as PageId,
                        slot_id,
                    });
                }
                Err(PageError::NoSpace) => continue,
                Err(error) => return Err(error.into()),
            }
        }

        let page_id = self.pages.len() as PageId;
        let mut page = Page::new(page_id);
        let slot_id = page.insert(payload)?;
        self.pages.push(page);

        Ok(RowId { page_id, slot_id })
    }

    pub fn read(&self, row_id: RowId) -> Result<Option<&[u8]>, HeapError> {
        let page = self
            .pages
            .get(row_id.page_id as usize)
            .ok_or(HeapError::InvalidPage)?;
        Ok(page.read(row_id.slot_id)?)
    }

    pub fn delete(&mut self, row_id: RowId) -> Result<(), HeapError> {
        let page = self
            .pages
            .get_mut(row_id.page_id as usize)
            .ok_or(HeapError::InvalidPage)?;
        page.delete(row_id.slot_id)?;
        Ok(())
    }

    pub fn scan(&self) -> Result<Vec<(RowId, Vec<u8>)>, HeapError> {
        let mut rows = Vec::new();
        for (page_index, page) in self.pages.iter().enumerate() {
            let slot_count = page.slot_count();
            for slot_id in 0..slot_count {
                if let Some(data) = page.read(slot_id)? {
                    rows.push((
                        RowId {
                            page_id: page_index as PageId,
                            slot_id,
                        },
                        data.to_vec(),
                    ));
                }
            }
        }
        Ok(rows)
    }
}

impl Default for TableHeap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_read_delete_roundtrip() {
        let mut heap = TableHeap::new();
        let row_id = heap.insert(b"foo").expect("insert failed");
        assert_eq!(heap.read(row_id).unwrap().unwrap(), b"foo");
        heap.delete(row_id).expect("delete failed");
        assert!(heap.read(row_id).unwrap().is_none());
    }
}
