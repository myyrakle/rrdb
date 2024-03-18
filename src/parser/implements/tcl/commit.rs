use std::error::Error;

use crate::ast::tcl::CommitQuery;
use crate::ast::SQLStatement;
use crate::parser::predule::{Parser, ParserContext};

impl Parser {
    pub(crate) fn parse_commit_query(
        &mut self,
        _context: ParserContext,
    ) -> Result<SQLStatement, Box<dyn Error + Send>> {
        Ok(CommitQuery {}.into())
    }
}
