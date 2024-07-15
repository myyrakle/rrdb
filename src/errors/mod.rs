pub mod execute_error;
pub mod into_error;
pub mod lexing_error;
pub mod parsing_error;
pub mod predule;
pub mod server_error;
pub mod type_error;

#[derive(Debug, PartialEq)]
pub enum RRDBError {
    ExecuteError(execute_error::ExecuteError),
    IntoError(into_error::IntoError),
    LexingError(lexing_error::LexingError),
    ParsingError(parsing_error::ParsingError),
    ServerError(server_error::ServerError),
    TypeError(type_error::TypeError),
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
        }
    }
}
