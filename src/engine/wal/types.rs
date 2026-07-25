use serde::{Deserialize, Serialize};

use crate::engine::ast::dml::insert::InsertQuery;

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
pub struct WALEntry {
    pub entry_type: EntryType,
    pub data: Option<Vec<u8>>,
    pub timestamp: u128,
    pub transaction_id: Option<u64>,
    pub is_continuation: bool,
}

impl WALEntry {
    pub fn size(&self) -> usize {
        let data_size = self.data.as_ref().map_or(0, |data| data.len());

        size_of::<EntryType>()
            + size_of::<u128>()
            + size_of::<Option<u64>>()
            + size_of::<bool>()
            + data_size
    }
}

/// Payload for `EntryType::Insert`.
///
/// The query alone is not enough to replay an INSERT idempotently. Rows can
/// become durable before the WAL checkpoint boundary advances, so a crash can
/// leave the rows on disk while the WAL entry is still replay-pending; replay
/// would then append them a second time. Deduplicating by row value is not an
/// option because duplicate rows are legal on tables without a unique index.
///
/// `start_row_index` records where this statement's rows were placed, giving
/// replay a stable identity: if the table already holds rows at that position,
/// this INSERT is already durable and must be skipped (see issue #236).
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct InsertWALPayload {
    pub query: InsertQuery,
    pub start_row_index: usize,
    pub row_count: usize,
}

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
pub enum EntryType {
    #[default]
    Insert,
    Set,
    Delete,
    Checkpoint,

    CreateIndex,
    DropIndex,

    TransactionBegin,
    TransactionCommit,
    TransactionRollback,
}
