//! Rebuildable row-index -> frame-body directory. The row/WAL formats are unchanged.
use crate::engine::query_memory::QueryMemoryTracker;
use crate::engine::row_buffer::ROW_FRAME_LIVE;
use crate::errors;
use crate::errors::execute_error::ExecuteError;

pub(crate) const FRAME_HEADER_LEN: u64 = 5;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn offset_cache_cap_is_not_a_row_count_limit() {
        let mut directory = FrameDirectory::default();
        for index in 0..MAX_DIRECTORY_BYTES / size_of::<FrameOffset>() + 2 {
            directory
                .push(
                    FrameOffset {
                        body: index as u64 * 5 + 5,
                        len: 0,
                        live: false,
                    },
                    None,
                )
                .unwrap();
        }
        assert!(directory.frames.capacity() * size_of::<FrameOffset>() <= MAX_DIRECTORY_BYTES);
    }
    #[test]
    fn offset_header_bounds_do_not_overflow() {
        assert!(FrameOffset::from_header([0; 5], u64::MAX, u64::MAX).is_err());
        assert!(
            FrameOffset::from_header([0, 0xff, 0xff, 0xff, 0xff], u64::MAX - 5, u64::MAX).is_err()
        );
    }
}
/// One directory retained by the buffer pool, including Vec capacity slack.
/// This bound applies even outside a query (e.g. recovery / internal callers).
pub(crate) const MAX_DIRECTORY_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug)]
pub(crate) struct FrameOffset {
    pub(crate) body: u64,
    pub(crate) len: u32,
    pub(crate) live: bool,
}

impl FrameOffset {
    pub(crate) fn from_header(header: [u8; 5], offset: u64, file_len: u64) -> errors::Result<Self> {
        let body = offset
            .checked_add(FRAME_HEADER_LEN)
            .filter(|end| *end <= file_len)
            .ok_or_else(|| ExecuteError::wrap("truncated row segment frame header"))?;
        let len = u32::from_le_bytes(header[1..].try_into().unwrap());
        body.checked_add(u64::from(len))
            .filter(|end| *end <= file_len)
            .ok_or_else(|| ExecuteError::wrap("truncated row segment frame body"))?;
        // Match the existing reader: every nonzero flag is a tombstone.
        Ok(Self {
            body,
            len,
            live: header[0] == ROW_FRAME_LIVE,
        })
    }

    pub(crate) fn end(self) -> u64 {
        self.body + u64::from(self.len)
    }
}

#[derive(Default)]
pub(crate) struct FrameDirectory {
    pub(crate) frames: Vec<FrameOffset>,
    pub(crate) file_len: u64,
    pub(crate) live_count: usize,
    pub(crate) row_count: usize,
}

impl FrameDirectory {
    pub(crate) fn push(
        &mut self,
        frame: FrameOffset,
        tracker: Option<&QueryMemoryTracker>,
    ) -> errors::Result<()> {
        // The cap limits cached offsets, not table size. Retain a prefix;
        // callers can header-walk the uncached suffix without allocations.
        // Never fill a hole in the prefix with a later row's ordinal.
        if self.frames.len() != self.row_count
            || self.row_count >= MAX_DIRECTORY_BYTES / size_of::<FrameOffset>()
        {
            self.live_count += usize::from(frame.live);
            self.row_count += 1;
            self.file_len = frame.end();
            return Ok(());
        }
        if self.frames.len() == self.frames.capacity() {
            let max = MAX_DIRECTORY_BYTES / size_of::<FrameOffset>();
            let capacity = (self.frames.capacity().max(128) * 2).min(max);
            if capacity <= self.frames.len() {
                return Err(ExecuteError::wrap("row offset directory limit exceeded"));
            }
            let additional = capacity - self.frames.len();
            if let Some(tracker) = tracker {
                tracker.reserve((additional * size_of::<FrameOffset>()) as u64)?;
            }
            self.frames.try_reserve_exact(additional).map_err(|error| {
                ExecuteError::wrap(format!("row offset allocation failed: {error}"))
            })?;
        }
        self.live_count += usize::from(frame.live);
        self.file_len = frame.end();
        self.frames.push(frame);
        self.row_count += 1;
        Ok(())
    }

    /// Only inspect new encoded frames, never rescan/clone the old directory.
    /// Failure evicts this optional cache; it must not turn a successful flush
    /// into a retry of data that has already been written.
    pub(crate) fn extend(&mut self, content: &[u8]) -> errors::Result<()> {
        let base = self.file_len;
        let file_len = base
            .checked_add(content.len() as u64)
            .ok_or_else(|| ExecuteError::wrap("row segment offset overflow"))?;
        let mut offset = 0;
        while offset < content.len() {
            let header = content
                .get(offset..offset + 5)
                .ok_or_else(|| ExecuteError::wrap("truncated row segment frame header"))?;
            let frame = FrameOffset::from_header(
                header.try_into().unwrap(),
                base + offset as u64,
                file_len,
            )?;
            offset = (frame.end() - base) as usize;
            self.push(frame, None)?;
        }
        Ok(())
    }
}
