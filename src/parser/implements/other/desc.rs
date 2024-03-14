use std::error::Error;

use crate::ast::other::DescTableQuery;
use crate::ast::predule::SQLStatement;
use crate::errors::predule::ParsingError;
use crate::parser::predule::{Parser, ParserContext};

impl Parser {
    pub(crate) fn parse_desc_query(
        &mut self,
        context: ParserContext,
    ) -> Result<SQLStatement, Box<dyn Error + Send>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E1001 need more tokens"));
        }

        let table_name = self.parse_table_name(context)?;

        Ok(DescTableQuery { table_name }.into())
    }
}
