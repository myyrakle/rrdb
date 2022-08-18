use std::error::Error;

use crate::lib::ast::predule::UpdateQuery;
use crate::lib::errors::predule::ParsingError;
use crate::lib::lexer::predule::Token;
use crate::lib::parser::predule::{Parser, ParserContext};

impl Parser {
    pub(crate) fn handle_update_query(
        &mut self,
        context: ParserContext,
    ) -> Result<UpdateQuery, Box<dyn Error>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0601: need more tokens"));
        }

        let current_token = self.get_next_token();

        if current_token != Token::Update {
            return Err(ParsingError::boxed(format!(
                "E0602: expected 'UPDATE'. but your input word is '{:?}'",
                current_token
            )));
        }

        let mut query_builder = UpdateQuery::builder();

        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0603: need more tokens"));
        }

        // 테이블명 파싱
        let table_name = self.parse_table_name()?;
        query_builder = query_builder.set_target_table(table_name);

        if self.next_token_is_table_alias() {
            let alias = self.parse_table_alias()?;
            query_builder = query_builder.set_target_alias(alias);
        }

        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0604: need more tokens"));
        }

        if current_token != Token::Set {
            return Err(ParsingError::boxed(format!(
                "E0605: expected 'SET'. but your input word is '{:?}'",
                current_token
            )));
        }

        Ok(query_builder.build())
    }
}
