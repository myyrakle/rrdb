use crate::lib::parser::predule::Parser;

use crate::lib::lexer::predule::Token;

impl Parser {
    // 다음 토큰 획득
    pub(crate) fn get_next_token(&mut self) -> Token {
        self.tokens.pop_front().unwrap()
    }

    // 토큰 획득 롤백
    pub(crate) fn unget_next_token(&mut self, token: Token) {
        self.tokens.push_front(token)
    }

    // 다음 토큰 유무 확인
    pub(crate) fn has_next_token(&self) -> bool {
        self.tokens.len() != 0 && !self.tokens.front().unwrap().is_eof()
    }
}
