use crate::engine::ast::SQLStatement;
use crate::errors::predule::ParsingError;
use crate::errors::RRDBError;
use crate::engine::lexer::predule::Token;
use crate::engine::parser::context::ParserContext;
use crate::engine::parser::predule::Parser;

impl Parser {
    // CREATE...로 시작되는 쿼리 분석
    pub(crate) fn handle_create_query(
        &mut self,
        context: ParserContext,
    ) -> Result<SQLStatement, RRDBError> {
        if !self.has_next_token() {
            return Err(ParsingError::wrap("E1101 need more tokens"));
        }

        let current_token = self.get_next_token();

        match current_token {
            Token::Table => self.handle_create_table_query(context),
            Token::Database => self.handle_create_database_query(),
            _ => Err(ParsingError::wrap(format!(
                "E1102 not supported command. possible commands: (create table). but your input is {:?}",
                current_token
            ))),
        }
    }

    // ALTER TABLE...
    pub(crate) fn handle_alter_query(
        &mut self,
        context: ParserContext,
    ) -> Result<SQLStatement, RRDBError> {
        if !self.has_next_token() {
            return Err(ParsingError::wrap("E1103 need more tokens"));
        }

        let current_token = self.get_next_token();

        match current_token {
            Token::Table => self.handle_alter_table_query(context),
            Token::Database => self.handle_alter_database_query(),
            _ => Err(ParsingError::wrap(
                "E1104 not supported command. possible commands: (alter table)",
            )),
        }
    }

    pub(crate) fn handle_drop_query(
        &mut self,
        context: ParserContext,
    ) -> Result<SQLStatement, RRDBError> {
        if !self.has_next_token() {
            return Err(ParsingError::wrap("E1105 need more tokens"));
        }

        let current_token = self.get_next_token();

        match current_token {
            Token::Table => self.handle_drop_table_query(context),
            Token::Database => self.handle_drop_database_query(),
            _ => Err(ParsingError::wrap(
                "E1106 not supported command. possible commands: (create table)",
            )),
        }
    }
}
