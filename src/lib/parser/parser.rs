use crate::lib::{IExpression, IntegerExpression, Token, Tokenizer};

pub struct Parser {
    pub current_token: Token,
    pub tokenizer: Tokenizer,
}

impl Parser {
    pub fn new(text: String) -> Self {
        Self {
            current_token: Token::EOF,
            tokenizer: Tokenizer::new(text),
        }
    }

    pub fn get_next_token(&mut self) -> Token {
        self.current_token = self.tokenizer.get_token();
        self.current_token.to_owned()
    }

    pub fn has_next_token(&mut self) -> bool {
        !self.tokenizer.is_eof()
    }

    fn parse_integer(value: i64) -> Box<dyn IExpression> {
        Box::new(IntegerExpression::new(value))
    }

    pub fn parse() {
        loop {}
    }
}
