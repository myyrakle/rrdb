use std::error::Error;

use crate::lib::{IExpression, IntegerExpression, ParsingError, Token, Tokenizer, SQLStatement, CreateTableQuery};

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

    fn _parse_integer(value: i64) -> Box<dyn IExpression> {
        Box::new(IntegerExpression::new(value))
    }

    // CREATE...로 시작되는 쿼리 분석
    fn handle_create_query(&mut self) -> Result<(), Box<dyn Error>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        let current_token = self.get_next_token();

        match current_token {
            Token::Table => {
                self.handle_create_table_query()?;
            }
            _ => {
                return Err(ParsingError::boxed(
                    "not supported command. possible commands: (create table)",
                ));
            }
        }

        Ok(())
    }

    // CREATE table 쿼리 분석
    fn handle_create_table_query(&mut self) -> Result<Box<dyn SQLStatement>, Box<dyn Error>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        let current_token = self.get_next_token();

        let mut query_builder = CreateTableQuery::builder();

        // [IF NOT EXISTS] 체크 로직
        if Token::If == current_token {
            if !self.has_next_token() {
                return Err(ParsingError::boxed("need more tokens"));
            }

            let current_token = self.get_next_token();

            if Token::Not == current_token {
                if !self.has_next_token() {
                    return Err(ParsingError::boxed("need more tokens"));
                }
    
                let current_token = self.get_next_token();

                if Token::Exists == current_token {
                    query_builder.set_if_not_exists(true);
                } else {
                    return Err(ParsingError::boxed("need more tokens"));
                }
            } else {
                return Err(ParsingError::boxed("need more tokens"));
            }
        }

        Ok(query_builder.build())
    }

    fn handle_alter_query(&mut self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    fn handle_drop_query(&mut self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    fn handle_select_query(&mut self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    fn handle_update_query(&mut self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    fn handle_delete_query(&mut self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    fn handle_insert_query(&mut self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

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
                    Token::Alter => self.handle_alter_query()?,
                    Token::Drop => self.handle_drop_query()?,
                    Token::Select => self.handle_select_query()?,
                    Token::Update => self.handle_update_query()?,
                    Token::Insert => self.handle_insert_query()?,
                    Token::Delete => self.handle_delete_query()?,
                    _ => (),
                }
            } else {
                break;
            }
        }

        Ok(())
    }
}
