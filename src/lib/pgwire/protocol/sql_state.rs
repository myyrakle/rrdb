#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqlState(pub &'static str);

impl SqlState {
    pub const SUCCESSFUL_COMPLETION: SqlState = SqlState("00000");
    pub const FEATURE_NOT_SUPPORTED: SqlState = SqlState("0A000");
    pub const INVALID_CURSOR_NAME: SqlState = SqlState("34000");
    pub const CONNECTION_EXCEPTION: SqlState = SqlState("08000");
    pub const INVALID_SQL_STATEMENT_NAME: SqlState = SqlState("26000");
    pub const DATA_EXCEPTION: SqlState = SqlState("22000");
    pub const PROTOCOL_VIOLATION: SqlState = SqlState("08P01");
    pub const SYNTAX_ERROR: SqlState = SqlState("42601");
    pub const INVALID_DATETIME_FORMAT: SqlState = SqlState("22007");
}
