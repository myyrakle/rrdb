#![cfg(test)]

use crate::ast::predule::{
    BinaryOperator, BinaryOperatorExpression, DeleteQuery, SQLExpression, SelectColumn, TableName,
    WhereClause,
};
use crate::parser::context::ParserContext;
use crate::parser::predule::Parser;

#[test]
pub fn delete_from_1() {
    let text = r#"
        DELETE FROM foo.bar
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = DeleteQuery::builder()
        .set_from_table(TableName {
            database_name: Some("foo".into()),
            table_name: "bar".into(),
        })
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}

#[test]
pub fn delete_from_where_1() {
    let text = r#"
        DELETE FROM foo.bar WHERE name = 'asdf'
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = DeleteQuery::builder()
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
        .build();

    assert_eq!(
        parser.parse(ParserContext::default()).unwrap(),
        vec![expected.into()],
    );
}
