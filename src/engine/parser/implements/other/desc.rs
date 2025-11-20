use crate::engine::ast::SQLStatement;
use crate::engine::ast::other::desc_table::DescTableQuery;
use crate::engine::parser::predule::{Parser, ParserContext};
use crate::errors::{Errors, ErrorKind};

impl Parser {
    pub(crate) fn parse_desc_query(
        &mut self,
        context: ParserContext,
    ) -> Result<SQLStatement, Errors> {
        if !self.has_next_token() {
            return Err(Errors::new(ErrorKind::ParsingError("E1001 need more tokens".to_string())));
        }

        let table_name = self.parse_table_name(context)?;

        Ok(DescTableQuery { table_name }.into())
    }
}
