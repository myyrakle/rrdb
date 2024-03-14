use crate::ast::ddl::{AlterDatabaseAction, AlterDatabaseQuery, AlterDatabaseRenameTo};
use crate::parser::predule::Parser;

use crate::ast::predule::{CreateDatabaseQuery, DropDatabaseQuery, SQLStatement};
use crate::errors::predule::ParsingError;
use crate::lexer::predule::Token;
use std::error::Error;

impl Parser {
    // CREATE DATABASE 쿼리 분석
    pub(crate) fn handle_create_database_query(
        &mut self,
    ) -> Result<SQLStatement, Box<dyn Error + Send>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0101 need more tokens"));
        }

        let mut query_builder = CreateDatabaseQuery::builder();

        // IF NOT EXISTS 파싱
        let if_not_exists = self.has_if_not_exists()?;
        query_builder = query_builder.set_if_not_exists(if_not_exists);

        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0102 need more tokens"));
        }

        let current_token = self.get_next_token();

        match current_token {
            Token::Identifier(identifier) => {
                query_builder = query_builder.set_name(identifier);
            }
            _ => {
                return Err(ParsingError::boxed(
                    "not supported command. possible commands: (create database)",
                ));
            }
        }

        if !self.has_next_token() {
            return Ok(query_builder.build());
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

    // DROP DATABASE 쿼리 분석
    pub(crate) fn handle_drop_database_query(
        &mut self,
    ) -> Result<SQLStatement, Box<dyn Error + Send>> {
        let mut query_builder = DropDatabaseQuery::builder();

        // IF EXISTS 파싱
        let if_exists = self.has_if_exists()?;
        query_builder = query_builder.set_if_exists(if_exists);

        // 테이블명 획득 로직
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0104 need more tokens"));
        }

        let current_token = self.get_next_token();

        match current_token {
            Token::Identifier(identifier) => {
                query_builder = query_builder.set_name(identifier);
            }
            _ => {
                return Err(ParsingError::boxed(
                    "not supported command. possible commands: (create database)",
                ));
            }
        }

        // 세미콜론 체크
        if !self.has_next_token() {
            return Ok(query_builder.build());
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

    // ALTER DATABASE 쿼리 분석
    pub(crate) fn handle_alter_database_query(
        &mut self,
    ) -> Result<SQLStatement, Box<dyn Error + Send>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0105 need more tokens"));
        }

        let mut query_builder = AlterDatabaseQuery::builder();

        let current_token = self.get_next_token();

        match current_token {
            Token::Identifier(identifier) => {
                query_builder = query_builder.set_name(identifier);
            }
            _ => {
                return Err(ParsingError::boxed(
                    "not supported command. possible commands: (alter database)",
                ));
            }
        }

        if !self.has_next_token() {
            return Ok(query_builder.build());
        }

        let current_token = self.get_next_token();

        match current_token {
            Token::Rename => {
                if !self.has_next_token() {
                    return Err(ParsingError::boxed(
                        "E106: expected 'TO'. but no more token",
                    ));
                }

                let current_token = self.get_next_token();

                if current_token != Token::To {
                    return Err(ParsingError::boxed(format!(
                        "E107: expected 'TO'. but your input word is '{:?}'",
                        current_token
                    )));
                }

                if !self.has_next_token() {
                    return Err(ParsingError::boxed(
                        "E108: expected identifier. but no more token",
                    ));
                }

                let current_token = self.get_next_token();

                match current_token {
                    Token::Identifier(identifier) => {
                        query_builder = query_builder.set_action(AlterDatabaseAction::RenameTo(
                            AlterDatabaseRenameTo { name: identifier },
                        ));
                    }
                    _ => {
                        return Err(ParsingError::boxed(
                            "E109: not supported command. possible commands: (alter database)",
                        ));
                    }
                }
            }
            Token::SemiColon => {}
            _ => {
                return Err(ParsingError::boxed(format!(
                    "E107: not supported syntax'{:?}'",
                    current_token
                )));
            }
        }

        Ok(query_builder.build())
    }
}
