use crate::ast::other::show_databases::ShowDatabasesQuery;
use crate::ast::SQLStatement;
use crate::errors::predule::ParsingError;
use crate::errors::RRDBError;
use crate::lexer::predule::Token;
use crate::parser::predule::{Parser, ParserContext};

impl Parser {
    pub(crate) fn parse_backslash_query(
        &mut self,
        _context: ParserContext,
    ) -> Result<SQLStatement, RRDBError> {
        if !self.has_next_token() {
            return Err(ParsingError::new("E0801 need more tokens"));
        }

        let current_token = self.get_next_token();

        match current_token {
            Token::Identifier(identifier) => match identifier.as_str() {
                "l" => Ok(ShowDatabasesQuery {}.into()),
                _ => Err(ParsingError::new(format!(
                    "E0803: unexpected identifier '{:?}'",
                    identifier
                ))),
            },
            _ => Err(ParsingError::new(format!(
                "E0802: unexpected token '{:?}'",
                current_token
            ))),
        }
    }
}
