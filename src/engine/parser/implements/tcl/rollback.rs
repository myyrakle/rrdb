use crate::engine::ast::tcl::RollbackQuery;
use crate::engine::ast::SQLStatement;
use crate::errors::RRDBError;
use crate::engine::parser::predule::{Parser, ParserContext};

impl Parser {
    pub(crate) fn parse_rollback_query(
        &mut self,
        _context: ParserContext,
    ) -> Result<SQLStatement, RRDBError> {
        Ok(RollbackQuery {}.into())
    }
}
