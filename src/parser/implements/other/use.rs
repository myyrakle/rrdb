use crate::ast::other::use_database::UseDatabaseQuery;
use crate::ast::SQLStatement;
use crate::errors::predule::ParsingError;
use crate::errors::RRDBError;
use crate::lexer::predule::Token;
use crate::parser::predule::{Parser, ParserContext};

impl Parser {
    pub(crate) fn parse_use_query(
        &mut self,
        _context: ParserContext,
    ) -> Result<SQLStatement, RRDBError> {
        if !self.has_next_token() {
            return Err(ParsingError::wrap("E0901 need more tokens"));
        }

        let current_token = self.get_next_token();

        match current_token {
            Token::Identifier(identifier) => Ok(UseDatabaseQuery {
                database_name: identifier,
            }
            .into()),
            _ => Err(ParsingError::wrap(format!(
                "E0902: unexpected token '{:?}'",
                current_token
            ))),
        }
    }
}
