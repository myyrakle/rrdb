use crate::engine::ast::SQLStatement;
use crate::engine::ast::tcl::BeginTransactionQuery;
use crate::engine::lexer::tokens::Token;
use crate::engine::parser::predule::{Parser, ParserContext};
use crate::errors::parsing_error::ParsingError;
use crate::errors;

impl Parser {
    pub(crate) fn parse_begin_query(
        &mut self,
        _context: ParserContext,
    ) -> errors::Result<SQLStatement> {
        if !self.has_next_token() {
            return Err(ParsingError::wrap("need more tokens"));
        }

        let current_token = self.get_next_token();

        if current_token != Token::Transaction {
            return Err(ParsingError::wrap(
                "Expected BEGIN".to_string(),
            ));
        }

        Ok(BeginTransactionQuery {}.into())
    }
}
