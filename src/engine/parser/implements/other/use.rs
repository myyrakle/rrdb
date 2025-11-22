use crate::engine::ast::SQLStatement;
use crate::engine::ast::other::use_database::UseDatabaseQuery;
use crate::engine::lexer::predule::Token;
use crate::engine::parser::predule::{Parser, ParserContext};
use crate::errors::{self, Errors, ErrorKind};
use crate::errors::parsing_error::ParsingError;

impl Parser {
    pub(crate) fn parse_use_query(
        &mut self,
        _context: ParserContext,
    ) -> errors::Result<SQLStatement> {
        if !self.has_next_token() {
            return Err(Errors::new(ErrorKind::ParsingError("need more tokens".to_string())));
        }

        let current_token = self.get_next_token();

        match current_token {
            Token::Identifier(identifier) => Ok(UseDatabaseQuery {
                database_name: identifier,
            }
            .into()),
            _ => Err(ParsingError::wrap(format!(
                "unexpected token '{:?}'",
                current_token
            ))),
        }
    }
}
