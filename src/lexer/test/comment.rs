#[cfg(test)]
use crate::lexer::predule::{Token, Tokenizer};

#[test]
pub fn comment_1() {
    let text = r#"SELECT 1 -- asdf"#.to_owned();

    let tokens = Tokenizer::string_to_tokens(text).unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::Select,
            Token::Integer(1),
            Token::CodeComment(" asdf".to_owned())
        ]
    );
}

#[test]
pub fn comment_2() {
    let text = r#"SELECT /*asdf*/1"#.to_owned();

    let tokens = Tokenizer::string_to_tokens(text).unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::Select,
            Token::CodeComment("asdf".to_owned()),
            Token::Integer(1),
        ]
    );
}
