use std::error::Error;

use crate::lib::ast::predule::UpdateQuery;
use crate::lib::errors::predule::ParsingError;
use crate::lib::lexer::predule::Token;
use crate::lib::parser::predule::{Parser, ParserContext};

impl Parser {
    pub(crate) fn handle_update_query(
        &mut self,
        context: ParserContext,
    ) -> Result<UpdateQuery, Box<dyn Error>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0601: need more tokens"));
        }

        let current_token = self.get_next_token();

        if current_token != Token::Update {
            return Err(ParsingError::boxed(format!(
                "E0602: expected 'UPDATE'. but your input word is '{:?}'",
                current_token
            )));
        }

        let mut query_builder = UpdateQuery::builder();

        // TODO: IMPL

        Ok(query_builder.build())
    }
}
