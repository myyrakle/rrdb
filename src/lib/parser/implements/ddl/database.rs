use crate::lib::parser::Parser;

use crate::lib::ast::ddl::CreateDatabaseQuery;
use crate::lib::{ParsingError, SQLStatement, Token};
use std::error::Error;

impl Parser {
    // CREATE TABLE 쿼리 분석
    pub(crate) fn handle_create_database_query(
        &mut self,
    ) -> Result<Box<dyn SQLStatement>, Box<dyn Error>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        let mut query_builder = CreateDatabaseQuery::builder();

        // IF NOT EXISTS 파싱
        let if_not_exists = self.has_if_not_exists()?;
        query_builder.set_if_not_exists(if_not_exists);

        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        let current_token = self.get_next_token();

        match current_token {
            Token::Identifier(identifier) => {
                query_builder.set_name(identifier);
            }
            _ => {
                return Err(ParsingError::boxed(
                    "not supported command. possible commands: (create database)",
                ));
            }
        }

        // 세미콜론 체크
        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        let current_token = self.get_next_token();

        if Token::SemiColon != current_token {
            return Err(ParsingError::boxed(format!(
                "expected ';'. but your input word is '{:?}'",
                current_token
            )));
        }

        Ok(query_builder.build())
    }
}
