use std::error::Error;

use crate::lib::ast::predule::InsertQuery;
use crate::lib::errors::predule::ParsingError;
use crate::lib::parser::predule::{Parser, ParserContext};

impl Parser {
    pub(crate) fn handle_insert_query(
        &mut self,
        _context: ParserContext,
    ) -> Result<InsertQuery, Box<dyn Error>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        let _current_token = self.get_next_token();

        // TODO: impl

        todo!();
    }
}
