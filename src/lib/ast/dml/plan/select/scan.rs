#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectScanType {
    FullScan,
    IndexScan,
}
