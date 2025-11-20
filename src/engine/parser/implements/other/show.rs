use crate::engine::ast::SQLStatement;
use crate::engine::ast::other::show_databases::ShowDatabasesQuery;
use crate::engine::ast::other::show_tables::ShowTablesQuery;
use crate::engine::lexer::predule::Token;
use crate::engine::parser::predule::{Parser, ParserContext};
use crate::errors::{Errors, ErrorKind};
use crate::errors::parsing_error::ParsingError;

impl Parser {
    pub(crate) fn parse_show_query(
        &mut self,
        context: ParserContext,
    ) -> Result<SQLStatement, Errors> {
        if !self.has_next_token() {
            return Err(Errors::new(ErrorKind::ParsingError("E0701 need more tokens".to_string())));
        }

        let current_token = self.get_next_token();

        match current_token {
            Token::Databases => Ok(ShowDatabasesQuery {}.into()),
            Token::Tables => Ok(ShowTablesQuery {
                database: context.default_database.unwrap_or_else(|| "None".into()),
            }
            .into()),
            _ => Err(ParsingError::wrap(format!(
                "E0702: unexpected token '{:?}'",
                current_token
            ))),
        }
    }
}
