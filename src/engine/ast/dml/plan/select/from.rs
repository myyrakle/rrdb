use crate::engine::ast::types::TableName;

use super::scan::ScanType;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectFromPlan {
    pub table_name: TableName,
    pub alias: Option<String>,
    pub scan: ScanType,
    /// Upper bound on rows the scan needs to produce, when the query shape
    /// makes that safe (#51). `None` means "scan everything", which is the
    /// only correct answer whenever a later stage can reorder or drop rows —
    /// ORDER BY, GROUP BY, aggregates, joins and WHERE all qualify.
    ///
    /// Carries OFFSET + LIMIT, not LIMIT: the rows skipped by OFFSET still
    /// have to be read before the ones that are returned.
    pub scan_limit: Option<usize>,
}
