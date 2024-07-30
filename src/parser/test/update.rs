#![cfg(test)]

use crate::ast::dml::expressions::binary::BinaryOperatorExpression;
use crate::ast::dml::expressions::operators::BinaryOperator;
use crate::ast::dml::parts::_where::WhereClause;
use crate::ast::dml::parts::update_item::UpdateItem;
use crate::ast::dml::update::UpdateQuery;
use crate::ast::types::{SQLExpression, SelectColumn, TableName};
use crate::lexer::predule::OperatorToken;
use crate::lexer::tokens::Token;
use crate::parser::predule::Parser;

#[test]
fn test_update_query() {
    struct TestCase {
        name: String,
        input: Vec<Token>,
        expected: UpdateQuery,
        want_error: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "Update foo.bar set a = 1 where a = 5".to_owned(),
            input: vec![
                Token::Update,
                Token::Identifier("foo".to_owned()),
                Token::Period,
                Token::Identifier("bar".to_owned()),
                Token::Set,
                Token::Identifier("a".to_owned()),
                Token::Operator(OperatorToken::Eq),
                Token::Integer(1),
                Token::Where,
                Token::Identifier("a".to_owned()),
                Token::Operator(OperatorToken::Eq),
                Token::Integer(5),
            ],
            expected: UpdateQuery::builder()
                .set_target_table(TableName {
                    database_name: Some("foo".into()),
                    table_name: "bar".into(),
                })
                .add_update_item(UpdateItem {
                    column: "a".into(),
                    value: SQLExpression::Integer(1),
                })
                .set_where(WhereClause {
                    expression: BinaryOperatorExpression {
                        operator: BinaryOperator::Eq,
                        lhs: SelectColumn::new(None, "a".into()).into(),
                        rhs: SQLExpression::Integer(5),
                    }
                    .into(),
                })
                .build(),
            want_error: false,
        },
        TestCase {
            name: "Update foo.bar set a = 1, b = 2".to_owned(),
            input: vec![
                Token::Update,
                Token::Identifier("foo".to_owned()),
                Token::Period,
                Token::Identifier("bar".to_owned()),
                Token::Set,
                Token::Identifier("a".to_owned()),
                Token::Operator(OperatorToken::Eq),
                Token::Integer(1),
                Token::Comma,
                Token::Identifier("b".to_owned()),
                Token::Operator(OperatorToken::Eq),
                Token::Integer(2),
            ],
            expected: UpdateQuery::builder()
                .set_target_table(TableName {
                    database_name: Some("foo".into()),
                    table_name: "bar".into(),
                })
                .add_update_item(UpdateItem {
                    column: "a".into(),
                    value: SQLExpression::Integer(1),
                })
                .add_update_item(UpdateItem {
                    column: "b".into(),
                    value: SQLExpression::Integer(2),
                })
                .build(),
            want_error: false,
        },
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got = parser.handle_update_query(Default::default());

        assert_eq!(
            got.is_err(),
            t.want_error,
            "{}: want_error: {}, error: {:?}",
            t.name,
            t.want_error,
            got.err()
        );

        if let Ok(statements) = got {
            assert_eq!(statements, t.expected, "TC: {}", t.name);
        }
    }
}
