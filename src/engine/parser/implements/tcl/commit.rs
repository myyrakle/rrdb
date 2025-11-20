use crate::engine::ast::SQLStatement;
use crate::engine::ast::tcl::CommitQuery;
use crate::engine::parser::predule::{Parser, ParserContext};
use crate::errors::Errors;

impl Parser {
    pub(crate) fn parse_commit_query(
        &mut self,
        _context: ParserContext,
    ) -> Result<SQLStatement, Errors> {
        Ok(CommitQuery {}.into())
    }
}
