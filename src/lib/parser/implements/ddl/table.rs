use crate::lib::ast::ddl::AlterTableQuery;
use crate::lib::ast::predule::{CreateTableQuery, DropTableQuery, SQLStatement};
use crate::lib::errors::predule::ParsingError;
use crate::lib::lexer::predule::Token;
use crate::lib::parser::context::ParserContext;
use crate::lib::parser::predule::Parser;
use std::error::Error;

impl Parser {
    // CREATE TABLE 쿼리 분석
    pub(crate) fn handle_create_table_query(
        &mut self,
        context: ParserContext,
    ) -> Result<SQLStatement, Box<dyn Error>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E1205 need more tokens"));
        }

        let mut query_builder = CreateTableQuery::builder();

        // IF NOT EXISTS 파싱
        let if_not_exists = self.has_if_not_exists()?;
        query_builder = query_builder.set_if_not_exists(if_not_exists);

        // 테이블명 설정
        let table = self.parse_table_name(context)?;
        query_builder = query_builder.set_table(table);

        // 여는 괄호 체크
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E1206 need more tokens"));
        }

        let current_token = self.get_next_token();

        if Token::LeftParentheses != current_token {
            return Err(ParsingError::boxed(format!(
                "E1207 expected '('. but your input word is '{:?}'",
                current_token
            )));
        }

        // 닫는 괄호 나올때까지 행 파싱 반복
        loop {
            if !self.has_next_token() {
                return Err(ParsingError::boxed("E1208 need more tokens"));
            }

            let current_token = self.get_next_token();

            match current_token {
                Token::RightParentheses => {
                    self.unget_next_token(current_token);
                    break;
                }
                _ => {
                    self.unget_next_token(current_token);
                    let column = self.parse_table_column()?;
                    query_builder = query_builder.add_column(column);
                }
            }
        }

        // 닫는 괄호 체크
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E1209 need more tokens"));
        }

        let current_token = self.get_next_token();

        if Token::RightParentheses != current_token {
            return Err(ParsingError::boxed(format!(
                "E1210 expected ')'. but your input word is '{:?}'",
                current_token
            )));
        }

        if !self.has_next_token() {
            return Ok(query_builder.build());
        }

        let current_token = self.get_next_token();

        if Token::SemiColon != current_token {
            return Err(ParsingError::boxed(format!(
                "E1211 expected ';'. but your input word is '{:?}'",
                current_token
            )));
        }

        Ok(query_builder.build())
    }

    // ALTER TABLE 쿼리 분석
    pub(crate) fn handle_alter_table_query(&mut self) -> Result<SQLStatement, Box<dyn Error>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E1201 need more tokens"));
        }

        let _current_token = self.get_next_token();

        let query_builder = AlterTableQuery::builder();

        if !self.has_next_token() {
            return Ok(query_builder.build());
        }

        let current_token = self.get_next_token();

        if Token::SemiColon != current_token {
            return Err(ParsingError::boxed(format!(
                "E1202 expected ';'. but your input word is '{:?}'",
                current_token
            )));
        }

        Ok(query_builder.build())
    }

    // DROP TABLE 쿼리 분석
    pub(crate) fn handle_drop_table_query(
        &mut self,
        context: ParserContext,
    ) -> Result<SQLStatement, Box<dyn Error>> {
        let mut query_builder = DropTableQuery::builder();

        // IF EXISTS 파싱
        let if_exists = self.has_if_exists()?;
        query_builder = query_builder.set_if_exists(if_exists);

        // 테이블명 획득 로직
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E1203 need more tokens"));
        }

        let table = self.parse_table_name(context)?;

        // 테이블명 설정
        query_builder = query_builder.set_table(table);

        if !self.has_next_token() {
            return Ok(query_builder.build());
        }

        let current_token = self.get_next_token();

        if Token::SemiColon != current_token {
            return Err(ParsingError::boxed(format!(
                "E1204 expected ';'. but your input word is '{:?}'",
                current_token
            )));
        }

        Ok(query_builder.build())
    }
}
