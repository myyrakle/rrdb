use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::engine::encoder::schema_encoder::StorageEncoder;
use crate::engine::schema::row::TableDataRow;
use crate::errors;
use crate::errors::execute_error::ExecuteError;

#[derive(Default)]
pub(crate) struct RowBufferPool {
    segments: HashMap<PathBuf, RowSegmentBuffer>,
    unsynced_segments: HashSet<PathBuf>,
}

#[derive(Default)]
struct RowSegmentBuffer {
    persisted_rows: Option<Vec<TableDataRow>>,
    pending_append_rows: Vec<TableDataRow>,
    pending_append_bytes: Vec<u8>,
    rewrite_required: bool,
}

pub(crate) struct RowBufferWrite {
    pub(crate) segment_path: PathBuf,
    pub(crate) content: Vec<u8>,
    pub(crate) replace_existing: bool,
    kind: RowBufferWriteKind,
}

enum RowBufferWriteKind {
    Append { rows: Vec<TableDataRow> },
    Rewrite { rows: Vec<TableDataRow> },
}

impl RowBufferPool {
    pub(crate) fn append_rows(
        &mut self,
        segment_path: PathBuf,
        rows: &[TableDataRow],
        encoded_rows: Vec<u8>,
    ) -> usize {
        let segment = self.segments.entry(segment_path).or_default();
        segment.pending_append_rows.extend_from_slice(rows);
        segment
            .pending_append_bytes
            .extend_from_slice(&encoded_rows);
        self.dirty_bytes()
    }

    pub(crate) fn read_rows(
        &mut self,
        segment_path: PathBuf,
        disk_rows: Vec<TableDataRow>,
    ) -> Vec<TableDataRow> {
        let segment = self.segments.entry(segment_path).or_default();
        let persisted_rows = segment.persisted_rows.get_or_insert(disk_rows);
        let mut rows = persisted_rows.clone();
        rows.extend_from_slice(&segment.pending_append_rows);
        rows
    }

    pub(crate) fn cached_rows(&self, segment_path: &PathBuf) -> Option<Vec<TableDataRow>> {
        let segment = self.segments.get(segment_path)?;
        let persisted_rows = segment.persisted_rows.as_ref()?;
        let mut rows = persisted_rows.clone();
        rows.extend_from_slice(&segment.pending_append_rows);
        Some(rows)
    }

    pub(crate) fn replace_rows(&mut self, segment_path: PathBuf, rows: Vec<TableDataRow>) {
        let segment = self.segments.entry(segment_path).or_default();
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
                rows.extend_from_slice(&segment.pending_append_rows);
                let content = encode_row_frames(&rows)?;
                segment.rewrite_required = false;
                segment.pending_append_rows.clear();
                segment.pending_append_bytes.clear();
                writes.push(RowBufferWrite {
                    segment_path: segment_path.clone(),
                    content,
                    replace_existing: true,
                    kind: RowBufferWriteKind::Rewrite { rows },
                });
                continue;
            }

            if !segment.pending_append_bytes.is_empty() {
                let rows = std::mem::take(&mut segment.pending_append_rows);
                let content = std::mem::take(&mut segment.pending_append_bytes);
                writes.push(RowBufferWrite {
                    segment_path: segment_path.clone(),
                    content,
                    replace_existing: false,
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
                    persisted_rows.extend(rows);
                }
            }
            RowBufferWriteKind::Rewrite { rows } => {
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

pub(crate) fn encode_row_frames(rows: &[TableDataRow]) -> errors::Result<Vec<u8>> {
    let encoder = StorageEncoder::new();
    let mut content = Vec::new();

    for row in rows {
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

    Ok(content)
}
