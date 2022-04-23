use crate::lib::parser::Parser;

use crate::lib::{CreateTableQuery, DropTableQuery, ParsingError, SQLStatement, Token};
use std::error::Error;

impl Parser {
    // CREATE...로 시작되는 쿼리 분석
    pub(crate) fn handle_create_query(&mut self) -> Result<Box<dyn SQLStatement>, Box<dyn Error>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        let current_token = self.get_next_token();

        match current_token {
            Token::Table => {
                return self.handle_create_table_query();
            }
            _ => {
                return Err(ParsingError::boxed(
                    "not supported command. possible commands: (create table)",
                ));
            }
        }
    }

    // ALTER TABLE...
    pub(crate) fn handle_alter_query(&mut self) -> Result<Box<dyn SQLStatement>, Box<dyn Error>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        let current_token = self.get_next_token();

        match current_token {
            Token::Table => {
                return self.handle_alter_table_query();
            }
            _ => {
                return Err(ParsingError::boxed(
                    "not supported command. possible commands: (alter table)",
                ));
            }
        }
    }

    pub(crate) fn handle_drop_query(&mut self) -> Result<Box<dyn SQLStatement>, Box<dyn Error>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        let current_token = self.get_next_token();

        match current_token {
            Token::Table => {
                return self.handle_drop_table_query();
            }
            _ => {
                return Err(ParsingError::boxed(
                    "not supported command. possible commands: (create table)",
                ));
            }
        }
    }
}
