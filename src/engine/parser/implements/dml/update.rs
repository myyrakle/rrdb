use crate::engine::ast::dml::parts::_where::WhereClause;
use crate::engine::ast::dml::parts::update_item::UpdateItem;
use crate::engine::ast::dml::update::UpdateQuery;
use crate::engine::lexer::predule::{OperatorToken, Token};
use crate::engine::parser::predule::{Parser, ParserContext};
use crate::errors::{self, Errors, ErrorKind};
use crate::errors::parsing_error::ParsingError;

impl Parser {
    pub(crate) fn handle_update_query(
        &mut self,
        context: ParserContext,
    ) -> errors::Result<UpdateQuery> {
        if !self.has_next_token() {
            return Err(Errors::new(ErrorKind::ParsingError("need more tokens".to_string())));
        }

        let current_token = self.get_next_token();

        if current_token != Token::Update {
            return Err(ParsingError::wrap(format!(
                "expected 'UPDATE'. but your input word is '{:?}'",
                current_token
            )));
        }

        let mut query_builder = UpdateQuery::builder();

        if !self.has_next_token() {
            return Err(Errors::new(ErrorKind::ParsingError("need more tokens".to_string())));
        }

        // 테이블명 파싱
        let table_name = self.parse_table_name(context.clone())?;
        query_builder = query_builder.set_target_table(table_name);

        if self.next_token_is_table_alias() {
            let alias = self.parse_table_alias()?;
            query_builder = query_builder.set_target_alias(alias);
        }

        if !self.has_next_token() {
            return Err(Errors::new(ErrorKind::ParsingError("need more tokens".to_string())));
        }

        let current_token = self.get_next_token();

        if current_token != Token::Set {
            return Err(ParsingError::wrap(format!(
                "expected 'SET'. but your input word is '{:?}'",
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
                        return Err(Errors::new(ErrorKind::ParsingError("need more tokens".to_string())));
                    }

                    let current_token = self.get_next_token();

                    if current_token != Token::Operator(OperatorToken::Eq) {
                        return Err(ParsingError::wrap(format!(
                            "expected '='. but your input word is '{:?}'",
                            current_token
                        )));
                    }

                    if !self.has_next_token() {
                        return Err(Errors::new(ErrorKind::ParsingError("need more tokens".to_string())));
                    }

                    let expression = self.parse_expression(context.clone())?;

                    let update_item = UpdateItem {
                        column: identifier,
                        value: expression,
                    };

                    query_builder = query_builder.add_update_item(update_item)
                }
                _ => {
                    return Err(ParsingError::wrap(format!(
                        "unexpected input word: '{:?}'",
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
