#![cfg(test)]

use crate::ast::dml::delete::DeleteQuery;
use crate::ast::dml::expressions::binary::BinaryOperatorExpression;
use crate::ast::dml::expressions::operators::BinaryOperator;
use crate::ast::dml::parts::_where::WhereClause;
use crate::ast::types::{SQLExpression, SelectColumn, TableName};
use crate::parser::context::ParserContext;
use crate::parser::predule::Parser;

#[test]
pub fn test_delete_query() {
    struct TestCase {
        name: String,
        input: String,
        expected: DeleteQuery,
        want_error: bool,
    }

    let test_cases = vec![
        TestCase {
            name: "where 없는 delete".into(),
            input: "DELETE FROM foo.bar".to_owned(),
            expected: DeleteQuery::builder()
                .set_from_table(TableName {
                    database_name: Some("foo".into()),
                    table_name: "bar".into(),
                })
                .build(),
            want_error: false,
        },
        TestCase {
            name: "where 있는 delete".into(),
            input: "DELETE FROM foo.bar WHERE name = 'asdf'".to_owned(),
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
        let mut parser = Parser::new(t.input).unwrap();

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
