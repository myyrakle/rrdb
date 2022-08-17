use std::error::Error;

use crate::lib::parser::predule::{Parser, ParserContext};

use crate::lib::ast::predule::DeleteQuery;
use crate::lib::errors::predule::ParsingError;
use crate::lib::lexer::predule::Token;

impl Parser {
    pub(crate) fn handle_delete_query(
        &mut self,
        context: ParserContext,
    ) -> Result<DeleteQuery, Box<dyn Error>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0501 need more tokens"));
        }

        // DELETE 토큰 삼키기
        let current_token = self.get_next_token();

        if current_token != Token::Delete {
            return Err(ParsingError::boxed(format!(
                "E0502: expected 'DELETE'. but your input word is '{:?}'",
                current_token
            )));
        }

        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0503 need more tokens"));
        }

        // FROM 토큰 삼키기
        let current_token = self.get_next_token();

        if current_token != Token::From {
            return Err(ParsingError::boxed(format!(
                "E0504: expected 'FROM'. but your input word is '{:?}'",
                current_token
            )));
        }

        let mut query_builder = DeleteQuery::builder();

        Ok(query_builder.build())
    }
}
