use std::error::Error;

use crate::lib::ast::predule::{SQLStatement, ShowDatabasesQuery};
use crate::lib::errors::predule::ParsingError;
use crate::lib::lexer::predule::Token;
use crate::lib::parser::predule::{Parser, ParserContext};

impl Parser {
    pub(crate) fn parse_backslash_query(
        &mut self,
        _context: ParserContext,
    ) -> Result<SQLStatement, Box<dyn Error + Send>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0801 need more tokens"));
        }

        let current_token = self.get_next_token();

        match current_token {
            Token::Identifier(identifier) => match identifier.as_str() {
                "l" => Ok(ShowDatabasesQuery {}.into()),
                _ => Err(ParsingError::boxed(format!(
                    "E0803: unexpected identifier '{:?}'",
                    identifier
                ))),
            },
            _ => Err(ParsingError::boxed(format!(
                "E0802: unexpected token '{:?}'",
                current_token
            ))),
        }
    }
}
