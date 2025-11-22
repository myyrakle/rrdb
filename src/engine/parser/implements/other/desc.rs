use crate::engine::ast::SQLStatement;
use crate::engine::ast::other::desc_table::DescTableQuery;
use crate::engine::parser::predule::{Parser, ParserContext};
use crate::errors::parsing_error::ParsingError;
use crate::errors::{self};

impl Parser {
    pub(crate) fn parse_desc_query(
        &mut self,
        context: ParserContext,
    ) -> errors::Result<SQLStatement> {
        if !self.has_next_token() {
            return Err(ParsingError::wrap("need more tokens"));
        }

        let table_name = self.parse_table_name(context)?;

        Ok(DescTableQuery { table_name }.into())
    }
}
