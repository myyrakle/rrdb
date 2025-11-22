use crate::engine::ast::SQLStatement;
use crate::engine::ast::tcl::RollbackQuery;
use crate::engine::parser::predule::{Parser, ParserContext};
use crate::errors;

impl Parser {
    pub(crate) fn parse_rollback_query(
        &mut self,
        _context: ParserContext,
    ) -> errors::Result<SQLStatement> {
        Ok(RollbackQuery {}.into())
    }
}
