use std::{collections::VecDeque, error::Error};

use crate::lib::ast::predule::SQLStatement;
use crate::lib::lexer::predule::{OperatorToken, Token, Tokenizer};
use crate::lib::parser::predule::ParserContext;

#[derive(Debug)]
pub struct Parser {
    pub current_token: Token,
    pub tokens: VecDeque<Token>,
}

impl Parser {
    // 파서 객체 생성
    pub fn new(text: String) -> Result<Self, Box<dyn Error>> {
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

    pub fn parse(&mut self) -> Result<Vec<SQLStatement>, Box<dyn Error>> {
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
                    Token::Create => statements.push(self.handle_create_query()?),
                    Token::Alter => statements.push(self.handle_alter_query()?),
                    Token::Drop => statements.push(self.handle_drop_query()?),
                    Token::Select => {
                        self.unget_next_token(current_token);
                        let query = self.handle_select_query(ParserContext::default())?;
                        statements.push(query);
                    }
                    Token::Update => statements.push(self.handle_update_query()?),
                    Token::Insert => statements.push(self.handle_insert_query()?),
                    Token::Delete => statements.push(self.handle_delete_query()?),
                    Token::Operator(operator) if operator == OperatorToken::Slash => {
                        // TODO: 추후 구현 필요. \c, \d 등...
                        continue;
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
