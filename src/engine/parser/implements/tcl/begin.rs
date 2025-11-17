use crate::engine::ast::tcl::BeginTransactionQuery;
use crate::engine::ast::SQLStatement;
use crate::errors::predule::ParsingError;
use crate::errors::RRDBError;
use crate::engine::lexer::tokens::Token;
use crate::engine::parser::predule::{Parser, ParserContext};

impl Parser {
    pub(crate) fn parse_begin_query(
        &mut self,
        _context: ParserContext,
    ) -> Result<SQLStatement, RRDBError> {
        if !self.has_next_token() {
            return Err(ParsingError::wrap("E2001 need more tokens"));
        }

        let current_token = self.get_next_token();

        if current_token != Token::Transaction {
            return Err(ParsingError::wrap("E2002 Expected BEGIN"));
        }

        Ok(BeginTransactionQuery {}.into())
    }
}
