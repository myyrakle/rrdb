use std::{collections::VecDeque, error::Error};

use crate::ast::predule::SQLStatement;
use crate::lexer::predule::{Token, Tokenizer};
use crate::parser::predule::ParserContext;

#[derive(Debug)]
pub struct Parser {
    pub current_token: Token,
    pub tokens: VecDeque<Token>,
}

impl Parser {
    // 파서 객체 생성
    pub fn new(text: String) -> Result<Self, Box<dyn Error + Send>> {
        Ok(Self {
            current_token: Token::EOF,
            tokens: VecDeque::from(Tokenizer::string_to_tokens(text)?),
        })
    }

    // 파서 객체 생성
    pub fn with_tokens(tokens: VecDeque<Token>) -> Self {
        Self {
            current_token: Token::EOF,
            tokens,
        }
    }

    pub fn parse(
        &mut self,
        context: ParserContext,
    ) -> Result<Vec<SQLStatement>, Box<dyn Error + Send>> {
        let mut statements: Vec<SQLStatement> = vec![];

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
                    Token::Create => statements.push(self.handle_create_query(context.clone())?),
                    Token::Alter => statements.push(self.handle_alter_query(context.clone())?),
                    Token::Drop => statements.push(self.handle_drop_query(context.clone())?),
                    Token::Select => {
                        self.unget_next_token(current_token);
                        let query = self.handle_select_query(context.clone())?;
                        statements.push(query.into());
                    }
                    Token::Update => {
                        self.unget_next_token(current_token);
                        let query = self.handle_update_query(context.clone())?;
                        statements.push(query.into());
                    }
                    Token::Insert => {
                        self.unget_next_token(current_token);
                        let query = self.handle_insert_query(context.clone())?;
                        statements.push(query.into());
                    }
                    Token::Delete => {
                        self.unget_next_token(current_token);
                        let query = self.handle_delete_query(context.clone())?;
                        statements.push(query.into());
                    }
                    Token::Backslash => {
                        let query = self.parse_backslash_query(context.clone())?;
                        statements.push(query);
                    }
                    Token::Show => {
                        let query = self.parse_show_query(context.clone())?;
                        statements.push(query);
                    }
                    Token::Use => {
                        let query = self.parse_use_query(context.clone())?;
                        statements.push(query);
                    }
                    Token::Desc => {
                        let query = self.parse_desc_query(context.clone())?;
                        statements.push(query);
                    }
                    _ => {
                        break;
                    }
                }
            } else {
                break;
            }
        }

        Ok(statements)
    }
}
