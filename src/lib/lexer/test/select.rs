use crate::lib::lexer::{Token, Tokenizer};

#[test]
pub fn select_1() {
    let text = r#"SELECT 1"#.to_owned();

    let tokens = Tokenizer::string_to_tokens(text);

    assert_eq!(tokens, vec![Token::Select, Token::Integer(1)]);
}

#[test]
pub fn select_2() {
    let text = r#"  SELECT 1432"#.to_owned();

    let tokens = Tokenizer::string_to_tokens(text);

    assert_eq!(tokens, vec![Token::Select, Token::Integer(1432)]);
}

#[test]
pub fn select_3() {
    let text = r#"SELECT 3.14"#.to_owned();

    let tokens = Tokenizer::string_to_tokens(text);

    assert_eq!(tokens, vec![Token::Select, Token::Float(3.14)]);
}

#[test]
pub fn select_from_1() {
    let text = r#"SELECT name from person"#.to_owned();

    let tokens = Tokenizer::string_to_tokens(text);

    assert_eq!(
        tokens,
        vec![
            Token::Select,
            Token::Identifier("name".to_owned()),
            Token::From,
            Token::Identifier("person".to_owned())
        ]
    );
}

#[test]
pub fn select_from_where_1() {
    let text = r#"SELECT name from person where"#.to_owned();

    let tokens = Tokenizer::string_to_tokens(text);

    assert_eq!(
        tokens,
        vec![
            Token::Select,
            Token::Identifier("name".to_owned()),
            Token::From,
            Token::Identifier("person".to_owned()),
            Token::Where,
        ]
    );
}
