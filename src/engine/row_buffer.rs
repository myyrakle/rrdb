use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::engine::encoder::schema_encoder::StorageEncoder;
use crate::engine::schema::row::TableDataRow;
use crate::errors;
use crate::errors::execute_error::ExecuteError;

pub(crate) const ROW_FRAME_LIVE: u8 = 0;
pub(crate) const ROW_FRAME_TOMBSTONE: u8 = 1;

#[derive(Default)]
pub(crate) struct RowBufferPool {
    segments: HashMap<PathBuf, RowSegmentBuffer>,
    unsynced_segments: HashSet<PathBuf>,
}

#[derive(Default)]
struct RowSegmentBuffer {
    persisted_rows: Option<Vec<Option<TableDataRow>>>,
    persisted_row_count: Option<usize>,
    pending_append_rows: Vec<TableDataRow>,
    pending_append_bytes: Vec<u8>,
    rewrite_required: bool,
}

pub(crate) struct RowBufferWrite {
    pub(crate) segment_path: PathBuf,
    pub(crate) content: Vec<u8>,
    pub(crate) replace_existing: bool,
    pub(crate) next_row_index: usize,
    kind: RowBufferWriteKind,
}

enum RowBufferWriteKind {
    Append { rows: Vec<TableDataRow> },
    Rewrite { rows: Vec<Option<TableDataRow>> },
}

impl RowBufferPool {
    pub(crate) fn seed_row_count(&mut self, segment_path: PathBuf, row_count: usize) {
        let segment = self.segments.entry(segment_path).or_default();
        segment.persisted_row_count.get_or_insert(row_count);
    }

    pub(crate) fn append_rows(
        &mut self,
        segment_path: PathBuf,
        rows: &[TableDataRow],
        encoded_rows: Vec<u8>,
    ) -> usize {
        let segment = self.segments.entry(segment_path).or_default();
        segment.pending_append_rows.extend_from_slice(rows);
        segment.pending_append_bytes.extend_from_slice(&encoded_rows);
        self.dirty_bytes()
    }

    pub(crate) fn read_rows(
        &mut self,
        segment_path: PathBuf,
        load_disk_rows: impl FnOnce() -> Vec<Option<TableDataRow>>,
    ) -> Vec<Option<TableDataRow>> {
        let segment = self.segments.entry(segment_path).or_default();
        let persisted_rows = segment.persisted_rows.get_or_insert_with(load_disk_rows);
        segment.persisted_row_count = Some(persisted_rows.len());
        let mut rows = persisted_rows.clone();
        rows.extend(
            segment
                .pending_append_rows
                .iter()
                .cloned()
                .map(Some),
        );
        rows
    }

    pub(crate) fn cached_rows(&self, segment_path: &PathBuf) -> Option<Vec<Option<TableDataRow>>> {
        let segment = self.segments.get(segment_path)?;
        let persisted_rows = segment.persisted_rows.as_ref()?;
        let mut rows = persisted_rows.clone();
        rows.extend(segment.pending_append_rows.iter().cloned().map(Some));
        Some(rows)
    }

    pub(crate) fn cached_row_count(&self, segment_path: &PathBuf) -> Option<usize> {
        let segment = self.segments.get(segment_path)?;
        let persisted_len = segment.persisted_row_count?;
        Some(persisted_len + segment.pending_append_rows.len())
    }

    pub(crate) fn replace_rows(&mut self, segment_path: PathBuf, rows: Vec<Option<TableDataRow>>) {
        let segment = self.segments.entry(segment_path).or_default();
        segment.persisted_row_count = Some(rows.len());
        segment.persisted_rows = Some(rows);
        segment.pending_append_rows.clear();
        segment.pending_append_bytes.clear();
        segment.rewrite_required = true;
    }

    pub(crate) fn drain_writes(&mut self) -> errors::Result<Vec<RowBufferWrite>> {
        let mut writes = Vec::new();

        for (segment_path, segment) in self.segments.iter_mut() {
            if segment.rewrite_required {
                let mut rows = segment.persisted_rows.clone().unwrap_or_default();
                rows.extend(segment.pending_append_rows.iter().cloned().map(Some));
                let content = encode_row_frames(&rows)?;
                segment.rewrite_required = false;
                segment.pending_append_rows.clear();
                segment.pending_append_bytes.clear();
                writes.push(RowBufferWrite {
                    segment_path: segment_path.clone(),
                    content,
                    replace_existing: true,
                    next_row_index: rows.len(),
                    kind: RowBufferWriteKind::Rewrite { rows },
                });
                continue;
            }

            if !segment.pending_append_bytes.is_empty() {
                let rows = std::mem::take(&mut segment.pending_append_rows);
                let content = std::mem::take(&mut segment.pending_append_bytes);
                let base_count = segment.persisted_row_count.unwrap_or_default();
                writes.push(RowBufferWrite {
                    segment_path: segment_path.clone(),
                    content,
                    replace_existing: false,
                    next_row_index: base_count + rows.len(),
                    kind: RowBufferWriteKind::Append { rows },
                });
            }
        }

        Ok(writes)
    }

    pub(crate) fn complete_write(&mut self, write: RowBufferWrite, durable: bool) {
        let segment_path = write.segment_path;
        let segment = self.segments.entry(segment_path.clone()).or_default();
        match write.kind {
            RowBufferWriteKind::Append { rows } => {
                if let Some(persisted_rows) = &mut segment.persisted_rows {
                    persisted_rows.extend(rows.into_iter().map(Some));
                }
                segment.persisted_row_count = Some(write.next_row_index);
            }
            RowBufferWriteKind::Rewrite { rows } => {
                segment.persisted_row_count = Some(rows.len());
                segment.persisted_rows = Some(rows);
            }
        }

        if durable {
            self.unsynced_segments.remove(&segment_path);
        } else {
            self.unsynced_segments.insert(segment_path);
        }
    }

    pub(crate) fn restore_write(&mut self, write: RowBufferWrite) {
        let segment = self.segments.entry(write.segment_path).or_default();
        match write.kind {
            RowBufferWriteKind::Append { rows } => {
                let mut restored_bytes = write.content;
                restored_bytes.extend_from_slice(&segment.pending_append_bytes);
                segment.pending_append_bytes = restored_bytes;

                let mut restored_rows = rows;
                restored_rows.extend_from_slice(&segment.pending_append_rows);
                segment.pending_append_rows = restored_rows;
            }
            RowBufferWriteKind::Rewrite { rows } => {
                segment.persisted_row_count = Some(rows.len());
                segment.persisted_rows = Some(rows);
                segment.pending_append_rows.clear();
                segment.pending_append_bytes.clear();
                segment.rewrite_required = true;
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn is_dirty_empty(&self) -> bool {
        self.segments.values().all(|segment| {
            !segment.rewrite_required
                && segment.pending_append_rows.is_empty()
                && segment.pending_append_bytes.is_empty()
        })
    }

    pub(crate) fn drain_unsynced_segments(&mut self) -> Vec<PathBuf> {
        self.unsynced_segments.drain().collect()
    }

    pub(crate) fn mark_unsynced_segment(&mut self, segment_path: PathBuf) {
        self.unsynced_segments.insert(segment_path);
    }

    #[cfg(test)]
    pub(crate) fn is_unsynced_empty(&self) -> bool {
        self.unsynced_segments.is_empty()
    }

    fn dirty_bytes(&self) -> usize {
        self.segments
            .values()
            .map(|segment| {
                segment.pending_append_bytes.len()
                    + if segment.rewrite_required {
                        segment
                            .persisted_rows
                            .as_ref()
                            .map(|rows| rows.len() * size_of::<TableDataRow>())
                            .unwrap_or_default()
                    } else {
                        0
                    }
            })
            .sum()
    }
}

pub(crate) fn encode_live_row_frames(rows: &[TableDataRow]) -> errors::Result<Vec<u8>> {
    let rows = rows.iter().cloned().map(Some).collect::<Vec<_>>();
    encode_row_frames(&rows)
}

pub(crate) fn encode_row_frames(rows: &[Option<TableDataRow>]) -> errors::Result<Vec<u8>> {
    let encoder = StorageEncoder::new();
    let mut content = Vec::new();

    for row in rows {
        match row {
            Some(row) => {
                content.push(ROW_FRAME_LIVE);
                let len_offset = content.len();
                content.extend_from_slice(&0u32.to_le_bytes());
                encoder
                    .encode_into(&mut content, row)
                    .map_err(|error| ExecuteError::wrap(error.to_string()))?;
                let frame_len = u32::try_from(content.len() - len_offset - size_of::<u32>())
                    .map_err(|_| ExecuteError::wrap("row frame is too large".to_string()))?;
                content[len_offset..len_offset + size_of::<u32>()]
                    .copy_from_slice(&frame_len.to_le_bytes());
            }
            None => {
                content.push(ROW_FRAME_TOMBSTONE);
                content.extend_from_slice(&0u32.to_le_bytes());
            }
        }
    }

    Ok(content)
}
