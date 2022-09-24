use crate::lib::ast::predule::Index;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScanType {
    FullScan,
    IndexScan(Index),
}
