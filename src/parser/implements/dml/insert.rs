use std::error::Error;

use crate::ast::predule::{InsertQuery, InsertValue};
use crate::errors::predule::ParsingError;
use crate::lexer::predule::Token;
use crate::parser::predule::{Parser, ParserContext};

impl Parser {
    pub(crate) fn handle_insert_query(
        &mut self,
        context: ParserContext,
    ) -> Result<InsertQuery, Box<dyn Error + Send>> {
        let mut query_builder = InsertQuery::builder();

        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0401 need more tokens"));
        }

        // INSERT 토큰 삼키기
        let current_token = self.get_next_token();
        if current_token != Token::Insert {
            return Err(ParsingError::boxed("E0402 expected INSERT"));
        }

        // INTO 토큰 삼키기
        let current_token = self.get_next_token();
        if current_token != Token::Into {
            return Err(ParsingError::boxed("E0403 expected INTO"));
        }

        // 테이블명 파싱
        let table_name = self.parse_table_name(context.clone())?;
        query_builder = query_builder.set_into_table(table_name);

        // 컬럼명 지정 파싱
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0404 need more tokens"));
        }

        let current_token = self.get_next_token();

        if current_token != Token::LeftParentheses {
            return Err(ParsingError::boxed(format!(
                "expected '('. but your input word is '{:?}'",
                current_token
            )));
        }

        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0405 need more tokens"));
        }

        // 컬럼명 지정 파싱
        let columns = self.parse_insert_columns(context.clone())?;
        query_builder = query_builder.set_columns(columns.clone());

        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0413 need more tokens"));
        }

        let current_token = self.get_next_token();

        match current_token {
            Token::Values => {
                self.unget_next_token(current_token);
                let values = self.parse_insert_values(context)?;

                if values.iter().any(|e| e.list.len() != columns.len()) {
                    return Err(ParsingError::boxed(
                        "E0415 The number of values in insert and the number of columns do not match.",
                    ));
                }

                query_builder = query_builder.set_values(values);
            }
            Token::Select => {
                self.unget_next_token(current_token);
                let select = self.handle_select_query(context)?;

                if select.select_items.len() != columns.len() {
                    return Err(ParsingError::boxed(
                        "E0416 The number of values in insert and the number of columns do not match.",
                    ));
                }

                query_builder = query_builder.set_select(select);
            }
            _ => {
                return Err(ParsingError::boxed(format!(
                    "E0414 expected 'Values'. but your input word is '{:?}'",
                    current_token
                )))
            }
        }

        // TODO: On Conflict 절 파싱

        // TODO: Returning 절 파싱

        Ok(query_builder.build())
    }

    // INSERT의 컬럼명 지정 부분 파싱
    // INSERT INTO (A, B, C) Values (1, 2, 3);
    //              ^^^^^^^
    pub(crate) fn parse_insert_columns(
        &mut self,
        _context: ParserContext,
    ) -> Result<Vec<String>, Box<dyn Error + Send>> {
        let mut names = vec![];
        loop {
            if !self.has_next_token() {
                return Err(ParsingError::boxed("E0406 need more tokens"));
            }

            let current_token = self.get_next_token();

            match current_token {
                Token::Identifier(identifier) => {
                    names.push(identifier);
                    continue;
                }
                Token::Comma => {
                    continue;
                }
                Token::RightParentheses => {
                    self.unget_next_token(current_token);
                    break;
                }
                _ => {
                    return Err(ParsingError::boxed(format!(
                        "E0407 unexpected input word '{:?}'",
                        current_token
                    )));
                }
            }
        }

        let current_token = self.get_next_token();

        if current_token != Token::RightParentheses {
            return Err(ParsingError::boxed(format!(
                "E0408 expected ')'. but your input word is '{:?}'",
                current_token
            )));
        }

        Ok(names)
    }

    // Values 절 파싱
    // INSERT INTO (A, B, C) Values(1, 2, 3);
    //                       ^^^^^^^^^^^^^^^
    pub(crate) fn parse_insert_values(
        &mut self,
        context: ParserContext,
    ) -> Result<Vec<InsertValue>, Box<dyn Error + Send>> {
        // Values 파싱
        let mut values: Vec<InsertValue> = vec![];

        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0409 need more tokens"));
        }

        let current_token = self.get_next_token();

        if current_token != Token::Values {
            return Err(ParsingError::boxed(format!(
                "E0408 expected 'Values'. but your input word is '{:?}'",
                current_token
            )));
        }

        loop {
            let mut list = vec![];

            if !self.has_next_token() {
                break;
            }

            let current_token = self.get_next_token();

            if current_token != Token::LeftParentheses {
                self.unget_next_token(current_token);
                break;
            }

            if !self.has_next_token() {
                return Err(ParsingError::boxed("E0411 need more tokens"));
            }

            // 각 Value 절 파싱. (A, B, C, D...)
            loop {
                if !self.has_next_token() {
                    return Err(ParsingError::boxed("E0412 need more tokens"));
                }

                let current_token = self.get_next_token();

                match current_token {
                    Token::Comma => {
                        continue;
                    }
                    Token::RightParentheses => {
                        break;
                    }
                    Token::Default => {
                        list.push(None);
                        continue;
                    }
                    _ => {
                        if current_token.is_expression() {
                            self.unget_next_token(current_token);
                            let expression = self.parse_expression(context.clone())?;
                            list.push(Some(expression));
                            continue;
                        }
                    }
                }
            }

            // 쉼표가 있으면 삼키기
            if self.pick_next_token() == Token::Comma {
                self.get_next_token();
            }

            let value = InsertValue { list };

            values.push(value);
        }

        Ok(values)
    }
}
