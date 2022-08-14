use std::error::Error;

use crate::lib::ast::predule::UpdateQuery;
use crate::lib::errors::predule::ParsingError;
use crate::lib::parser::predule::Parser;

impl Parser {
    pub(crate) fn handle_update_query(&mut self) -> Result<UpdateQuery, Box<dyn Error>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        let _current_token = self.get_next_token();

        // TODO: impl

        todo!();
    }
}
