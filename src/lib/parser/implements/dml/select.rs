use std::error::Error;

use crate::lib::lexer::Token;
use crate::lib::parser::Parser;
use crate::lib::{ParsingError, SQLStatement, SelectQuery};

impl Parser {
    pub(crate) fn handle_select_query(&mut self) -> Result<SQLStatement, Box<dyn Error>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("need more tokens"));
        }

        let query_builder = SelectQuery::builder();
        // TODO: impl

        // FROM 절이나 세미콜론이 나오기 전까지 select 절 파싱
        loop {
            let current_token = self.get_next_token();

            match current_token {
                Token::From => {
                    self.unget_next_token(current_token);
                    break;
                }
                Token::SemiColon => {
                    return Ok(query_builder.build());
                }
                _ => {}
            }
        }

        // FROM 절 파싱
        let current_token = self.get_next_token();

        match current_token {
            Token::From => {}
            _ => {
                return Err(ParsingError::boxed(format!(
                    "expected 'FROM'. but your input word is '{:?}'",
                    current_token
                )));
            }
        }

        Ok(query_builder.build())
    }
}
