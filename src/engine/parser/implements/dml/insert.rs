use crate::engine::ast::dml::insert::InsertQuery;
use crate::engine::ast::dml::parts::insert_values::InsertValue;
use crate::engine::lexer::predule::Token;
use crate::engine::parser::predule::{Parser, ParserContext};
use crate::errors;
use crate::errors::parsing_error::ParsingError;

impl Parser {
    pub(crate) fn handle_insert_query(
        &mut self,
        context: ParserContext,
    ) -> errors::Result<InsertQuery> {
        let mut query_builder = InsertQuery::builder();

        if !self.has_next_token() {
            return Err(ParsingError::wrap("need more tokens"));
        }

        // INSERT 토큰 삼키기
        let current_token = self.get_next_token();
        if current_token != Token::Insert {
            return Err(ParsingError::wrap("expected INSERT".to_string()));
        }

        // INTO 토큰 삼키기
        if !self.has_next_token() {
            return Err(ParsingError::wrap("need more tokens"));
        }

        let current_token = self.get_next_token();
        if current_token != Token::Into {
            return Err(ParsingError::wrap("expected INTO".to_string()));
        }

        // 테이블명 파싱
        let table_name = self.parse_table_name(context.clone())?;
        query_builder = query_builder.set_into_table(table_name);

        // 컬럼명 지정 파싱
        if !self.has_next_token() {
            return Err(ParsingError::wrap("need more tokens"));
        }

        let current_token = self.get_next_token();

        if current_token != Token::LeftParentheses {
            return Err(ParsingError::wrap(format!(
                "expected '('. but your input word is '{:?}'",
                current_token
            )));
        }

        if !self.has_next_token() {
            return Err(ParsingError::wrap("need more tokens"));
        }

        // 컬럼명 지정 파싱
        let columns = self.parse_insert_columns(context.clone())?;
        query_builder = query_builder.set_columns(columns.clone());

        if !self.has_next_token() {
            return Err(ParsingError::wrap("need more tokens"));
        }

        let current_token = self.get_next_token();

        match current_token {
            Token::Values => {
                self.unget_next_token(current_token);
                let values = self.parse_insert_values(context)?;

                if values.iter().any(|e| e.list.len() != columns.len()) {
                    return Err(ParsingError::wrap(
                        "The number of values in insert and the number of columns do not match.",
                    ));
                }

                query_builder = query_builder.set_values(values);
            }
            Token::Select => {
                self.unget_next_token(current_token);
                let select = self.handle_select_query(context)?;

                if select.select_items.len() != columns.len() {
                    return Err(ParsingError::wrap(
                        "The number of values in insert and the number of columns do not match.",
                    ));
                }

                query_builder = query_builder.set_select(select);
            }
            _ => {
                return Err(ParsingError::wrap(format!(
                    "expected 'Values'. but your input word is '{:?}'",
                    current_token
                )));
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
    ) -> errors::Result<Vec<String>> {
        let mut names = vec![];
        loop {
            if !self.has_next_token() {
                return Err(ParsingError::wrap("need more tokens"));
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
                    return Ok(names);
                }
                _ => {
                    return Err(ParsingError::wrap(format!(
                        "unexpected input word '{:?}'",
                        current_token
                    )));
                }
            }
        }
    }

    // Values 절 파싱
    // INSERT INTO (A, B, C) Values(1, 2, 3);
    //                       ^^^^^^^^^^^^^^^
    pub(crate) fn parse_insert_values(
        &mut self,
        context: ParserContext,
    ) -> errors::Result<Vec<InsertValue>> {
        // Values 파싱
        let mut values: Vec<InsertValue> = vec![];

        if !self.has_next_token() {
            return Err(ParsingError::wrap("need more tokens"));
        }

        let current_token = self.get_next_token();

        if current_token != Token::Values {
            return Err(ParsingError::wrap(format!(
                "expected 'Values'. but your input word is '{:?}'",
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
                return Err(ParsingError::wrap(format!(
                    "expected '('. but your input word is '{:?}'",
                    current_token
                )));
            }

            if !self.has_next_token() {
                return Err(ParsingError::wrap("need more tokens"));
            }

            // 각 Value 절 파싱. (A, B, C, D...)
            loop {
                if !self.has_next_token() {
                    return Err(ParsingError::wrap("need more tokens"));
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
            if self.has_next_token() && self.pick_next_token() == Token::Comma {
                self.get_next_token();
            }

            let value = InsertValue { list };

            values.push(value);
        }

        Ok(values)
    }
}
