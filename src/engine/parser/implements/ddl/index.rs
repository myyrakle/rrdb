use crate::engine::ast::SQLStatement;
use crate::engine::ast::ddl::create_index::CreateIndexQuery;
use crate::engine::ast::ddl::drop_index::DropIndexQuery;
use crate::engine::lexer::predule::Token;
use crate::engine::parser::context::ParserContext;
use crate::engine::parser::predule::Parser;
use crate::errors;
use crate::errors::parsing_error::ParsingError;

impl Parser {
    // CREATE [UNIQUE] INDEX 쿼리 분석
    // 진입 시점에는 INDEX 토큰까지 소비된 상태입니다.
    pub(crate) fn handle_create_index_query(
        &mut self,
        context: ParserContext,
        is_unique: bool,
    ) -> errors::Result<SQLStatement> {
        if !self.has_next_token() {
            return Err(ParsingError::wrap("need more tokens".to_string()));
        }

        let mut query_builder = CreateIndexQuery::builder().set_unique(is_unique);

        // IF NOT EXISTS 파싱
        let if_not_exists = self.has_if_not_exists()?;
        query_builder = query_builder.set_if_not_exists(if_not_exists);

        // 인덱스명 파싱
        if !self.has_next_token() {
            return Err(ParsingError::wrap("need more tokens".to_string()));
        }

        let current_token = self.get_next_token();

        let index_name = if let Token::Identifier(name) = current_token {
            name
        } else {
            return Err(ParsingError::wrap(format!(
                "expected index name. but your input word is '{:?}'",
                current_token
            )));
        };

        query_builder = query_builder.set_index_name(index_name);

        // ON 토큰 체크
        if !self.has_next_token() {
            return Err(ParsingError::wrap("need more tokens".to_string()));
        }

        let current_token = self.get_next_token();

        if Token::On != current_token {
            return Err(ParsingError::wrap(format!(
                "expected 'ON'. but your input word is '{:?}'",
                current_token
            )));
        }

        // 테이블명 파싱
        let table = self.parse_table_name(context)?;
        query_builder = query_builder.set_table(table);

        // 여는 괄호 체크
        if !self.has_next_token() {
            return Err(ParsingError::wrap("need more tokens".to_string()));
        }

        let current_token = self.get_next_token();

        if Token::LeftParentheses != current_token {
            return Err(ParsingError::wrap(format!(
                "expected '('. but your input word is '{:?}'",
                current_token
            )));
        }

        // 닫는 괄호 나올때까지 컬럼명 파싱 반복
        loop {
            if !self.has_next_token() {
                return Err(ParsingError::wrap("need more tokens".to_string()));
            }

            let current_token = self.get_next_token();

            match current_token {
                Token::RightParentheses => break,
                Token::Comma => continue,
                Token::Identifier(column_name) => {
                    query_builder = query_builder.add_column(column_name);
                }
                _ => {
                    return Err(ParsingError::wrap(format!(
                        "expected column name. but your input word is '{:?}'",
                        current_token
                    )));
                }
            }
        }

        if !self.has_next_token() {
            return Ok(query_builder.build());
        }

        let current_token = self.get_next_token();

        if Token::SemiColon != current_token {
            return Err(ParsingError::wrap(format!(
                "expected ';'. but your input word is '{:?}'",
                current_token
            )));
        }

        Ok(query_builder.build())
    }

    // DROP INDEX 쿼리 분석
    // 진입 시점에는 INDEX 토큰까지 소비된 상태입니다.
    pub(crate) fn handle_drop_index_query(
        &mut self,
        context: ParserContext,
    ) -> errors::Result<SQLStatement> {
        if !self.has_next_token() {
            return Err(ParsingError::wrap("need more tokens".to_string()));
        }

        let mut query_builder =
            DropIndexQuery::builder().set_database_name(context.default_database.clone());

        // IF EXISTS 파싱
        let if_exists = self.has_if_exists()?;
        query_builder = query_builder.set_if_exists(if_exists);

        // 인덱스명 파싱
        if !self.has_next_token() {
            return Err(ParsingError::wrap("need more tokens".to_string()));
        }

        let current_token = self.get_next_token();

        let index_name = if let Token::Identifier(name) = current_token {
            name
        } else {
            return Err(ParsingError::wrap(format!(
                "expected index name. but your input word is '{:?}'",
                current_token
            )));
        };

        query_builder = query_builder.set_index_name(index_name);

        if !self.has_next_token() {
            return Ok(query_builder.build());
        }

        let current_token = self.get_next_token();

        match current_token {
            // 선택적 ON [database_name.]table_name 절
            Token::On => {
                let table = self.parse_table_name(context)?;
                query_builder = query_builder.set_table(table);

                if !self.has_next_token() {
                    return Ok(query_builder.build());
                }

                let current_token = self.get_next_token();

                if Token::SemiColon != current_token {
                    return Err(ParsingError::wrap(format!(
                        "expected ';'. but your input word is '{:?}'",
                        current_token
                    )));
                }

                Ok(query_builder.build())
            }
            Token::SemiColon => Ok(query_builder.build()),
            _ => Err(ParsingError::wrap(format!(
                "expected ';' or 'ON'. but your input word is '{:?}'",
                current_token
            ))),
        }
    }
}
