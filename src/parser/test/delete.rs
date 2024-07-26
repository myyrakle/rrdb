#![cfg(test)]

use crate::ast::dml::delete::DeleteQuery;
use crate::ast::dml::expressions::binary::BinaryOperatorExpression;
use crate::ast::dml::expressions::operators::BinaryOperator;
use crate::ast::dml::parts::_where::WhereClause;
use crate::ast::types::{SQLExpression, SelectColumn, TableName};
use crate::lexer::predule::OperatorToken;
use crate::lexer::tokens::Token;
use crate::parser::context::ParserContext;
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
    ];

    for t in test_cases {
        let mut parser = Parser::new(t.input);

        let got = parser.parse(ParserContext::default());

        assert_eq!(
            got.is_err(),
            t.want_error,
            "{}: want_error: {}, error: {:?}",
            t.name,
            t.want_error,
            got.err()
        );

        if let Ok(statements) = got {
            assert_eq!(statements, vec![t.expected.into()], "TC: {}", t.name);
        }
    }
}
