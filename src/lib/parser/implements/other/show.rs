use std::error::Error;

use crate::lib::ast::other::ShowTablesQuery;
use crate::lib::ast::predule::{SQLStatement, ShowDatabasesQuery};
use crate::lib::errors::predule::ParsingError;
use crate::lib::lexer::predule::Token;
use crate::lib::parser::predule::{Parser, ParserContext};

impl Parser {
    pub(crate) fn parse_show_query(
        &mut self,
        context: ParserContext,
    ) -> Result<SQLStatement, Box<dyn Error>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0701 need more tokens"));
        }

        let current_token = self.get_next_token();

        match current_token {
            Token::Databases => Ok(ShowDatabasesQuery {}.into()),
            Token::Tables => Ok(ShowTablesQuery {
                database: context.default_database.unwrap_or("None".into()),
            }
            .into()),
            _ => Err(ParsingError::boxed(format!(
                "E0702: unexpected token '{:?}'",
                current_token
            ))),
        }
    }
}
