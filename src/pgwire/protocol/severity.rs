#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Severity(pub &'static str);

impl Severity {
    pub const ERROR: Severity = Severity("ERROR");
    pub const FATAL: Severity = Severity("FATAL");
}
