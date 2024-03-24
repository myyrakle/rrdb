pub mod execute_error;
pub mod into_error;
pub mod lexing_error;
pub mod parsing_error;
pub mod predule;
pub mod server_error;
pub mod type_error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RRDBError {
    ExecuteError(execute_error::ExecuteError),
    IntoError(into_error::IntoError),
    LexingError(lexing_error::LexingError),
    ParsingError(parsing_error::ParsingError),
    ServerError(server_error::ServerError),
    TypeError(type_error::TypeError),
}
