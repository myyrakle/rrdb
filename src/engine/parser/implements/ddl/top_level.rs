use crate::engine::ast::SQLStatement;
use crate::engine::lexer::predule::Token;
use crate::engine::parser::context::ParserContext;
use crate::engine::parser::predule::Parser;
use crate::errors;
use crate::errors::parsing_error::ParsingError;

impl Parser {
    // CREATE...로 시작되는 쿼리 분석
    pub(crate) fn handle_create_query(
        &mut self,
        context: ParserContext,
    ) -> errors::Result<SQLStatement> {
        if !self.has_next_token() {
            return Err(ParsingError::wrap("need more tokens".to_string()));
        }

        let current_token = self.get_next_token();

        match current_token {
            Token::Table => self.handle_create_table_query(context),
            Token::Database => self.handle_create_database_query(),
            _ => Err(ParsingError::wrap(format!(
                "not supported command. possible commands: (create table, create database). but your input is {:?}",
                current_token
            ))),
        }
    }

    // ALTER TABLE...
    pub(crate) fn handle_alter_query(
        &mut self,
        context: ParserContext,
    ) -> errors::Result<SQLStatement> {
        if !self.has_next_token() {
            return Err(ParsingError::wrap("need more tokens".to_string()));
        }

        let current_token = self.get_next_token();

        match current_token {
            Token::Table => self.handle_alter_table_query(context),
            Token::Database => self.handle_alter_database_query(),
            _ => Err(ParsingError::wrap(
                "not supported command. possible commands: (alter table, alter database)",
            )),
        }
    }

    pub(crate) fn handle_drop_query(
        &mut self,
        context: ParserContext,
    ) -> errors::Result<SQLStatement> {
        if !self.has_next_token() {
            return Err(ParsingError::wrap("need more tokens".to_string()));
        }

        let current_token = self.get_next_token();

        match current_token {
            Token::Table => self.handle_drop_table_query(context),
            Token::Database => self.handle_drop_database_query(),
            _ => Err(ParsingError::wrap(
                "not supported command. possible commands: (drop table, drop database)",
            )),
        }
    }
}
