use crate::engine::ast::types::Index;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScanType {
    FullScan,
    IndexScan(Index),
}
