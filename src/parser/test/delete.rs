#![cfg(test)]

use crate::ast::dml::delete::DeleteQuery;
use crate::ast::dml::expressions::binary::BinaryOperatorExpression;
use crate::ast::dml::expressions::operators::BinaryOperator;
use crate::ast::dml::parts::_where::WhereClause;
use crate::ast::types::{SQLExpression, SelectColumn, TableName};
use crate::lexer::predule::OperatorToken;
use crate::lexer::tokens::Token;
use crate::parser::predule::Parser;

#[test]
pub fn test_delete_query() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: DeleteQuery,
        want_error: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "성공: delete from foo.bar".into(),
            input: vec![
                Token::Delete,
                Token::From,
                Token::Identifier("foo".into()),
                Token::Period,
                Token::Identifier("bar".into()),
            ],
            expected: DeleteQuery::builder()
                .set_from_table(TableName {
                    database_name: Some("foo".into()),
                    table_name: "bar".into(),
                })
                .build(),
            want_error: false,
        },
        TestCase {
            name: "성공: DELETE FROM foo.bar WHERE name = 'asdf'".into(),
            input: vec![
                Token::Delete,
                Token::From,
                Token::Identifier("foo".into()),
                Token::Period,
                Token::Identifier("bar".into()),
                Token::Where,
                Token::Identifier("name".into()),
                Token::Operator(OperatorToken::Eq),
                Token::String("asdf".into()),
            ],
            expected: DeleteQuery::builder()
                .set_from_table(TableName {
                    database_name: Some("foo".into()),
                    table_name: "bar".into(),
                })
                .set_where(WhereClause {
                    expression: BinaryOperatorExpression {
                        operator: BinaryOperator::Eq,
                        lhs: SelectColumn::new(None, "name".into()).into(),
                        rhs: SQLExpression::String("asdf".into()),
                    }
                    .into(),
                })
                .build(),
            want_error: false,
        },
        TestCase {
            name: "성공: DELETE FROM foo.bar as f".into(),
            input: vec![
                Token::Delete,
                Token::From,
                Token::Identifier("foo".into()),
                Token::Period,
                Token::Identifier("bar".into()),
                Token::As,
                Token::Identifier("f".into()),
            ],
            expected: DeleteQuery::builder()
                .set_from_table(TableName {
                    database_name: Some("foo".into()),
                    table_name: "bar".into(),
                })
                .set_from_alias("f".into())
                .build(),
            want_error: false,
        },
        TestCase {
            name: "실패: 토큰이 하나도 없음".into(),
            input: vec![],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "실패: DELETE가 아님".into(),
            input: vec![Token::Select],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "실패: DELETE밖에 없음".into(),
            input: vec![Token::Delete],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "실패: DELETE INTO".into(),
            input: vec![Token::Delete, Token::Into],
            expected: Default::default(),
            want_error: true,
        },
        TestCase {
            name: "실패: DELETE FROM".into(),
            input: vec![Token::Delete, Token::From],
            expected: Default::default(),
            want_error: true,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got = parser.handle_delete_query(Default::default());

        assert_eq!(
            got.is_err(),
            t.want_error,
            "{}: want_error: {}, error: {:?}",
            t.name,
            t.want_error,
            got.err()
        );

        if let Ok(statements) = got {
            assert_eq!(statements, t.expected.into(), "TC: {}", t.name);
        }
    }
}
