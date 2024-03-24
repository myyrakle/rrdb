use std::error::Error;

use crate::ast::other::show_databases::ShowDatabasesQuery;
use crate::ast::other::show_tables::ShowTablesQuery;
use crate::ast::SQLStatement;
use crate::errors::predule::ParsingError;
use crate::lexer::predule::Token;
use crate::parser::predule::{Parser, ParserContext};

impl Parser {
    pub(crate) fn parse_show_query(
        &mut self,
        context: ParserContext,
    ) -> Result<SQLStatement, RRDBError> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0701 need more tokens"));
        }

        let current_token = self.get_next_token();

        match current_token {
            Token::Databases => Ok(ShowDatabasesQuery {}.into()),
            Token::Tables => Ok(ShowTablesQuery {
                database: context.default_database.unwrap_or_else(|| "None".into()),
            }
            .into()),
            _ => Err(ParsingError::boxed(format!(
                "E0702: unexpected token '{:?}'",
                current_token
            ))),
        }
    }
}
