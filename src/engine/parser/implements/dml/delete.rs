use crate::engine::ast::dml::delete::DeleteQuery;
use crate::engine::lexer::predule::Token;
use crate::engine::parser::predule::{Parser, ParserContext};
use crate::errors::parsing_error::ParsingError;
use crate::errors::{self, ErrorKind, Errors};

impl Parser {
    pub(crate) fn handle_delete_query(
        &mut self,
        context: ParserContext,
    ) -> errors::Result<DeleteQuery> {
        if !self.has_next_token() {
            return Err(Errors::new(ErrorKind::ParsingError(
                "need more tokens".to_string(),
            )));
        }

        // DELETE 토큰 삼키기
        let current_token = self.get_next_token();

        if current_token != Token::Delete {
            return Err(ParsingError::wrap(format!(
                "expected 'DELETE'. but your input word is '{:?}'",
                current_token
            )));
        }

        if !self.has_next_token() {
            return Err(Errors::new(ErrorKind::ParsingError(
                "need more tokens".to_string(),
            )));
        }

        // FROM 토큰 삼키기
        let current_token = self.get_next_token();

        if current_token != Token::From {
            return Err(ParsingError::wrap(format!(
                "expected 'FROM'. but your input word is '{:?}'",
                current_token
            )));
        }

        let mut query_builder = DeleteQuery::builder();

        if !self.has_next_token() {
            return Err(Errors::new(ErrorKind::ParsingError(
                "need more tokens".to_string(),
            )));
        }

        // 테이블명 파싱
        let table_name = self.parse_table_name(context.clone())?;
        query_builder = query_builder.set_from_table(table_name);

        // 테이블 alias 파싱
        if self.next_token_is_table_alias() {
            let alias = self.parse_table_alias()?;
            query_builder = query_builder.set_from_alias(alias);
        }

        // WHERE 절 파싱
        if self.next_token_is_where() {
            let where_clause = self.parse_where(context)?;
            query_builder = query_builder.set_where(where_clause);
        }

        Ok(query_builder.build())
    }
}
