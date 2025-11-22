use crate::engine::ast::SQLStatement;
use crate::engine::ast::other::show_databases::ShowDatabasesQuery;
use crate::engine::lexer::predule::Token;
use crate::engine::parser::predule::{Parser, ParserContext};
use crate::errors::parsing_error::ParsingError;
use crate::errors::{self};

impl Parser {
    pub(crate) fn parse_backslash_query(
        &mut self,
        _context: ParserContext,
    ) -> errors::Result<SQLStatement> {
        if !self.has_next_token() {
            return Err(ParsingError::wrap("need more tokens"));
        }

        let current_token = self.get_next_token();

        match current_token {
            Token::Identifier(identifier) => match identifier.as_str() {
                "l" => Ok(ShowDatabasesQuery {}.into()),
                _ => Err(ParsingError::wrap(format!(
                    "unexpected identifier '{:?}'",
                    identifier
                ))),
            },
            _ => Err(ParsingError::wrap(format!(
                "unexpected token '{:?}'",
                current_token
            ))),
        }
    }
}
