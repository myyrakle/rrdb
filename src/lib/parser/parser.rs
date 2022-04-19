use std::{collections::VecDeque, error::Error};

use crate::lib::{
    Column, ColumnBuilder, CreateTableQuery, FloatExpression, IExpression, IntegerExpression,
    ParsingError, SQLStatement, Table, Token, Tokenizer,
};

#[derive(Debug)]
pub struct Parser {
    pub current_token: Token,
    pub tokens: VecDeque<Token>,
}

impl Parser {
    pub fn new(text: String) -> Self {
        Self {
            current_token: Token::EOF,
            tokens: VecDeque::from(Tokenizer::string_to_tokens(text)),
        }
    }

    pub fn get_next_token(&mut self) -> Token {
        self.tokens.pop_front().unwrap()
    }

    pub fn unget_next_token(&mut self, token: Token) {
        self.tokens.push_front(token)
    }

    pub fn has_next_token(&self) -> bool {
        self.tokens.len() != 0 && !self.tokens.front().unwrap().is_eof()
    }

    fn _parse_integer(value: i64) -> Box<dyn IExpression> {
        Box::new(IntegerExpression::new(value))
    }

    fn _parse_float(value: f64) -> Box<dyn IExpression> {
        Box::new(FloatExpression::new(value))
    }

    // CREATE...로 시작되는 쿼리 분석
    fn handle_create_query(&mut self) -> Result<Box<dyn SQLStatement>, Box<dyn Error>> {
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

    // CREATE TABLE 쿼리 분석
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
                    return Err(ParsingError::boxed(format!(
                        "expected keyword is 'exists'. but your input word is '{:?}'",
                        current_token
                    )));
                }
            } else {
                return Err(ParsingError::boxed(format!(
                    "expected keyword is 'not'. but your input word is '{:?}'",
                    current_token
                )));
            }
        }

        // 테이블명 획득 로직
        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        // 첫번째로 오는 이름은 테이블명으로 추정
        let current_token = self.get_next_token();
        let mut table_name;
        let mut database_name = None;

        if let Token::Identifier(name) = current_token {
            table_name = name;
        } else {
            return Err(ParsingError::boxed(format!(
                "expected identifier. but your input word is '{:?}'",
                current_token
            )));
        }

        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        let current_token = self.get_next_token();

        // .가 있을 경우 "데이터베이스명"."테이블명"의 형태로 추정
        if current_token == Token::Period {
            if !self.has_next_token() {
                return Err(ParsingError::boxed("need more tokens"));
            }

            let current_token = self.get_next_token();

            if let Token::Identifier(name) = current_token {
                database_name = Some(table_name);
                table_name = name;
            } else {
                return Err(ParsingError::boxed(format!(
                    "expected identifier. but your input word is '{:?}'",
                    current_token
                )));
            }
        }

        // 테이블명 설정
        query_builder.set_table(Table::new(database_name, table_name));

        // 여는 괄호 체크
        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        let current_token = self.get_next_token();

        if Token::LeftParentheses != current_token {
            return Err(ParsingError::boxed(format!(
                "expected '('. but your input word is '{:?}'",
                current_token
            )));
        }

        // 닫는 괄호 나올때까지 행 파싱 반복
        loop {
            if !self.has_next_token() {
                return Err(ParsingError::boxed("need more tokens"));
            }

            let current_token = self.get_next_token();

            match current_token {
                Token::RightParentheses => {
                    self.unget_next_token(current_token);
                    break;
                }
                _ => {
                    let column = self.parse_table_column()?;
                    query_builder.add_column(column);
                }
            }
        }

        // 닫는 괄호 체크
        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        let current_token = self.get_next_token();

        if Token::RightParentheses != current_token {
            return Err(ParsingError::boxed(format!(
                "expected ')'. but your input word is '{:?}'",
                current_token
            )));
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

    fn parse_table_column(&mut self) -> Result<Column, Box<dyn Error>> {
        let mut builder = Column::builder();

        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        let current_token = self.get_next_token();

        if let Token::Identifier(name) = current_token {
            builder.set_name(name);
        } else {
            return Err(ParsingError::boxed(format!(
                "expected identifier. but your input word is '{:?}'",
                current_token
            )));
        }

        Ok(builder.build())
    }

    fn handle_alter_query(&mut self) -> Result<Box<dyn SQLStatement>, Box<dyn Error>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        let _current_token = self.get_next_token();

        let query_builder = CreateTableQuery::builder();
        // TODO: impl

        Ok(query_builder.build())
    }

    fn handle_drop_query(&mut self) -> Result<Box<dyn SQLStatement>, Box<dyn Error>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        let _current_token = self.get_next_token();

        let query_builder = CreateTableQuery::builder();
        // TODO: impl

        Ok(query_builder.build())
    }

    fn handle_select_query(&mut self) -> Result<Box<dyn SQLStatement>, Box<dyn Error>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        let _current_token = self.get_next_token();

        let query_builder = CreateTableQuery::builder();
        // TODO: impl

        Ok(query_builder.build())
    }

    fn handle_update_query(&mut self) -> Result<Box<dyn SQLStatement>, Box<dyn Error>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        let _current_token = self.get_next_token();

        let query_builder = CreateTableQuery::builder();
        // TODO: impl

        Ok(query_builder.build())
    }

    fn handle_delete_query(&mut self) -> Result<Box<dyn SQLStatement>, Box<dyn Error>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        let _current_token = self.get_next_token();

        let query_builder = CreateTableQuery::builder();
        // TODO: impl

        Ok(query_builder.build())
    }

    fn handle_insert_query(&mut self) -> Result<Box<dyn SQLStatement>, Box<dyn Error>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        let _current_token = self.get_next_token();

        let query_builder = CreateTableQuery::builder();
        // TODO: impl

        Ok(query_builder.build())
    }

    pub fn parse(&mut self) -> Result<Vec<Box<dyn SQLStatement>>, Box<dyn Error>> {
        let mut statements: Vec<Box<dyn SQLStatement>> = vec![];

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
                    Token::Select => statements.push(self.handle_select_query()?),
                    Token::Update => statements.push(self.handle_update_query()?),
                    Token::Insert => statements.push(self.handle_insert_query()?),
                    Token::Delete => statements.push(self.handle_delete_query()?),
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
