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

    pub fn has_next_token(&self) -> bool {
        !self.tokenizer.is_eof()
    }

    fn parse_integer(value: i64) -> Box<dyn IExpression> {
        Box::new(IntegerExpression::new(value))
    }

    pub fn parse(&mut self) {
        // Top-Level Parser Loop
        loop {
            if self.has_next_token() {
                let current_token = self.get_next_token();

                match current_token {
                    Token::EOF => {
                        // 루프 종료
                        break;
                    }
                    Token::SemiColon => {
                        // top-level 세미콜론 무시
                        continue;
                    }
                    Token::Create => {}
                    Token::Alter => {}
                    Token::Drop => {}
                    Token::Select => {}
                    Token::Update => {}
                    Token::Insert => {}
                    Token::Delete => {}
                    _ => (),
                }
            } else {
                break;
            }
        }
    }
}
