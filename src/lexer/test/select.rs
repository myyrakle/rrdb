#[cfg(test)]
use crate::lexer::predule::{Token, Tokenizer};

#[test]
pub fn select_1() {
    let text = r#"SELECT 1"#.to_owned();

    let tokens = Tokenizer::string_to_tokens(text).unwrap();

    assert_eq!(tokens, vec![Token::Select, Token::Integer(1)]);
}

#[test]
pub fn select_2() {
    let text = r#"  SELECT 1432"#.to_owned();

    let tokens = Tokenizer::string_to_tokens(text).unwrap();

    assert_eq!(tokens, vec![Token::Select, Token::Integer(1432)]);
}

#[test]
pub fn select_3() {
    let text = r#"SELECT 3.14"#.to_owned();

    let tokens = Tokenizer::string_to_tokens(text).unwrap();

    assert_eq!(tokens, vec![Token::Select, Token::Float(3.14)]);
}

#[test]
pub fn select_4() {
    let text = r#"SELECT 'I''m Sam'"#.to_owned();

    let tokens = Tokenizer::string_to_tokens(text).unwrap();

    assert_eq!(
        tokens,
        vec![Token::Select, Token::String("I'm Sam".to_owned())]
    );
}

#[test]
pub fn select_from_1() {
    let text = r#"SELECT name from person"#.to_owned();

    let tokens = Tokenizer::string_to_tokens(text).unwrap();

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
pub fn select_from_2() {
    let text = r#"SELECT 1 from "boom""#.to_owned();

    let tokens = Tokenizer::string_to_tokens(text).unwrap();

    assert_eq!(
        tokens,
        vec![
            Token::Select,
            Token::Integer(1),
            Token::From,
            Token::Identifier("boom".to_owned())
        ]
    );
}

#[test]
pub fn select_from_where_1() {
    let text = r#"SELECT name from person where"#.to_owned();

    let tokens = Tokenizer::string_to_tokens(text).unwrap();

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

// #[test]
// pub fn inner_join() {
//     let text = r#"
//         SELECT
//             p.name as name,
//             s.name as schoolName
//         from person as p
//         inner join school as s
//         on 1=1
//             and p.school_id = s.id
//     "#
//     .to_owned();

//     let tokens = Tokenizer::string_to_tokens(text);

//     assert_eq!(
//         tokens,
//         vec![
//             Token::Select,
//             Token::Identifier("p.name".to_owned()),
//             Token::As,
//             Token::Identifier("name".to_owned()),
//             Token::From,
//             Token::Identifier("person".to_owned()),
//             Token::Where,
//         ]
//     );
// }
