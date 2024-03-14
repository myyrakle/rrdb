use crate::parser::predule::Parser;

use crate::lexer::predule::Token;

impl Parser {
    // 다음 토큰 획득
    pub(crate) fn get_next_token(&mut self) -> Token {
        self.tokens.pop_front().unwrap()
    }

    // 다음 토큰 미리보기
    pub(crate) fn pick_next_token(&mut self) -> Token {
        self.tokens.front().unwrap().to_owned()
    }

    // 토큰 획득 롤백
    pub(crate) fn unget_next_token(&mut self, token: Token) {
        self.tokens.push_front(token)
    }

    // 다음 토큰 유무 확인
    pub(crate) fn has_next_token(&self) -> bool {
        !self.tokens.is_empty() && !self.tokens.front().unwrap().is_eof()
    }
}
