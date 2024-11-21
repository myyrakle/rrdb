pub mod execute_error;
pub mod into_error;
pub mod lexing_error;
pub mod parsing_error;
pub mod predule;
pub mod server_error;
pub mod type_error;
pub mod wal_errors;

#[derive(Debug, PartialEq)]
pub enum RRDBError {
    ExecuteError(execute_error::ExecuteError),
    IntoError(into_error::IntoError),
    LexingError(lexing_error::LexingError),
    ParsingError(parsing_error::ParsingError),
    ServerError(server_error::ServerError),
    TypeError(type_error::TypeError),
    WALError(wal_errors::WALError),
}

impl ToString for RRDBError {
    fn to_string(&self) -> String {
        match self {
            RRDBError::ExecuteError(e) => e.to_string(),
            RRDBError::IntoError(e) => e.to_string(),
            RRDBError::LexingError(e) => e.to_string(),
            RRDBError::ParsingError(e) => e.to_string(),
            RRDBError::ServerError(e) => e.to_string(),
            RRDBError::TypeError(e) => e.to_string(),
            RRDBError::WALError(e) => e.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use predule::{ExecuteError, IntoError, LexingError, ParsingError, ServerError, TypeError, WALError};

    use super::*;

    #[test]
    fn test_rrdb_error_to_string() {
        let error = ExecuteError::wrap("test");
        assert!(error.to_string().contains("test"));

        let error = IntoError::wrap("test");
        assert!(error.to_string().contains("test"));

        let error = LexingError::wrap("test");
        assert!(error.to_string().contains("test"));

        let error = ParsingError::wrap("test");
        assert!(error.to_string().contains("test"));

        let error = ServerError::wrap("test");
        assert!(error.to_string().contains("test"));

        let error = TypeError::wrap("test");
        assert!(error.to_string().contains("test"));
    }
}
