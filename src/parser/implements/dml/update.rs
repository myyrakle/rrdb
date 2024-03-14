use std::error::Error;

use crate::ast::predule::{UpdateItem, UpdateQuery, WhereClause};
use crate::errors::predule::ParsingError;
use crate::lexer::predule::{OperatorToken, Token};
use crate::parser::predule::{Parser, ParserContext};

impl Parser {
    pub(crate) fn handle_update_query(
        &mut self,
        context: ParserContext,
    ) -> Result<UpdateQuery, Box<dyn Error + Send>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0601: need more tokens"));
        }

        let current_token = self.get_next_token();

        if current_token != Token::Update {
            return Err(ParsingError::boxed(format!(
                "E0602: expected 'UPDATE'. but your input word is '{:?}'",
                current_token
            )));
        }

        let mut query_builder = UpdateQuery::builder();

        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0603: need more tokens"));
        }

        // 테이블명 파싱
        let table_name = self.parse_table_name(context.clone())?;
        query_builder = query_builder.set_target_table(table_name);

        if self.next_token_is_table_alias() {
            let alias = self.parse_table_alias()?;
            query_builder = query_builder.set_target_alias(alias);
        }

        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0604: need more tokens"));
        }

        let current_token = self.get_next_token();

        if current_token != Token::Set {
            return Err(ParsingError::boxed(format!(
                "E0605: expected 'SET'. but your input word is '{:?}'",
                current_token
            )));
        }

        loop {
            if !self.has_next_token() {
                break;
            }

            let current_token = self.get_next_token();

            match current_token {
                Token::Comma => continue,
                Token::Where => {
                    self.unget_next_token(current_token);
                    break;
                }
                Token::SemiColon => {
                    return Ok(query_builder.build());
                }
                Token::Identifier(identifier) => {
                    if !self.has_next_token() {
                        return Err(ParsingError::boxed("E0606: need more tokens"));
                    }

                    let current_token = self.get_next_token();

                    if current_token != Token::Operator(OperatorToken::Eq) {
                        return Err(ParsingError::boxed(format!(
                            "E0607: expected '='. but your input word is '{:?}'",
                            current_token
                        )));
                    }

                    if !self.has_next_token() {
                        return Err(ParsingError::boxed("E0608: need more tokens"));
                    }

                    let expression = self.parse_expression(context.clone())?;

                    let update_item = UpdateItem {
                        column: identifier,
                        value: expression,
                    };

                    query_builder = query_builder.add_update_item(update_item)
                }
                _ => {
                    return Err(ParsingError::boxed(format!(
                        "E0609: unexpected input word: '{:?}'",
                        current_token
                    )));
                }
            }
        }

        // Where 절 파싱
        if self.next_token_is_where() {
            self.get_next_token(); // where 토큰 삼키기

            let expression = self.parse_expression(context)?;
            query_builder = query_builder.set_where(WhereClause { expression });
        }

        Ok(query_builder.build())
    }
}
