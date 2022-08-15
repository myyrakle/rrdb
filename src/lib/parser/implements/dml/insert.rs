use std::error::Error;

use crate::lib::ast::predule::InsertQuery;
use crate::lib::errors::predule::ParsingError;
use crate::lib::lexer::predule::Token;
use crate::lib::parser::predule::{Parser, ParserContext};

impl Parser {
    pub(crate) fn handle_insert_query(
        &mut self,
        _context: ParserContext,
    ) -> Result<InsertQuery, Box<dyn Error>> {
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
        let table_name = self.parse_table_name()?;
        query_builder = query_builder.set_into_table(table_name);

        // TODO: 컬럼명 지정

        // TODO: Values 파싱

        // TODO: On Conflict 절 파싱

        // TODO: Returning 절 파싱

        Ok(query_builder.build())
    }
}
