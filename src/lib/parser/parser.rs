use std::error::Error;

use crate::lib::{IExpression, IntegerExpression, ParsingError, Token, Tokenizer};

pub struct Parser {
    pub current_token: Token,
    pub tokenizer: Tokenizer,
}

impl Parser {
    pub fn new(text: String) -> Self {
        Self {
            current_token: Token::EOF,
            tokenizer: Tokenizer::new(text),
        }
    }

    pub fn get_next_token(&mut self) -> Token {
        self.current_token = self.tokenizer.get_token();
        self.current_token.to_owned()
    }

    pub fn has_next_token(&self) -> bool {
        !self.tokenizer.is_eof()
    }

    fn parse_integer(value: i64) -> Box<dyn IExpression> {
        Box::new(IntegerExpression::new(value))
    }

    fn handle_create_query(&mut self) -> Result<(), Box<dyn Error>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("possible commands: (create table)"));
        }

        let current_token = self.get_next_token();

        match current_token {
            Token::Table => {}
            _ => {
                return Err(ParsingError::boxed(
                    "not supported command. possible commands: (create table)",
                ));
            }
        }

        Ok(())
    }

    fn handle_alter_query(&mut self) {}

    fn handle_drop_query(&mut self) {}

    fn handle_select_query(&mut self) {}

    fn handle_update_query(&mut self) {}

    fn handle_delete_query(&mut self) {}

    fn handle_insert_query(&mut self) {}

    pub fn parse(&mut self) -> Result<(), Box<dyn Error>> {
        // Top-Level Parser Loop
        loop {
            if self.has_next_token() {
                let current_token = self.get_next_token();

                match current_token {
                    Token::EOF => {
                        // 루프 종료
                        break;
                    }
                    Token::SemiColon => {
                        // top-level 세미콜론 무시
                        continue;
                    }
                    Token::Create => self.handle_create_query()?,
                    Token::Alter => self.handle_alter_query(),
                    Token::Drop => self.handle_drop_query(),
                    Token::Select => self.handle_select_query(),
                    Token::Update => self.handle_update_query(),
                    Token::Insert => self.handle_insert_query(),
                    Token::Delete => self.handle_delete_query(),
                    _ => (),
                }
            } else {
                break;
            }
        }

        Ok(())
    }
}
