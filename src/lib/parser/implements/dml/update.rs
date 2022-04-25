use std::error::Error;

use crate::lib::parser::Parser;

use crate::lib::{CreateTableQuery, ParsingError, SQLStatement};

impl Parser {
    pub(crate) fn handle_update_query(&mut self) -> Result<SQLStatement, Box<dyn Error>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        let _current_token = self.get_next_token();

        let query_builder = CreateTableQuery::builder();
        // TODO: impl

        Ok(query_builder.build())
    }
}
