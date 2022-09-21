use crate::lib::ast::predule::Index;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectScanType {
    FullScan,
    IndexScan(Index),
}
