#[cfg(test)]
use crate::lib::ast::predule::{
    BinaryOperator, BinaryOperatorExpression, SQLExpression, SelectItem, SelectQuery,
};
#[cfg(test)]
use crate::lib::parser::predule::Parser;

#[test]
pub fn expression_1() {
    let text = r#"
        SELECT 3 + 5 AS foo
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
        .add_select_item(
            SelectItem::builder()
                .set_item(SQLExpression::Binary(
                    BinaryOperatorExpression {
                        operator: BinaryOperator::Add,
                        lhs: SQLExpression::Integer(3),
                        rhs: SQLExpression::Integer(5),
                    }
                    .into(),
                ))
                .set_alias("foo".into())
                .build(),
        )
        .build();

    assert_eq!(parser.parse().unwrap(), vec![expected],);
}

#[test]
pub fn expression_2() {
    let text = r#"
        SELECT 1 + 2 + 3 AS foo
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
        .add_select_item(
            SelectItem::builder()
                .set_item(SQLExpression::Binary(
                    BinaryOperatorExpression {
                        operator: BinaryOperator::Add,
                        lhs: SQLExpression::Binary(
                            BinaryOperatorExpression {
                                operator: BinaryOperator::Add,
                                lhs: SQLExpression::Integer(1),
                                rhs: SQLExpression::Integer(2),
                            }
                            .into(),
                        ),
                        rhs: SQLExpression::Integer(3),
                    }
                    .into(),
                ))
                .set_alias("foo".into())
                .build(),
        )
        .build();

    assert_eq!(parser.parse().unwrap(), vec![expected],);
}

#[test]
pub fn expression_3() {
    let text = r#"
        SELECT 1 + 2 * 3 AS foo
    "#
    .to_owned();

    let mut parser = Parser::new(text).unwrap();

    let expected = SelectQuery::builder()
        .add_select_item(
            SelectItem::builder()
                .set_item(SQLExpression::Binary(
                    BinaryOperatorExpression {
                        operator: BinaryOperator::Add,
                        lhs: SQLExpression::Integer(1),
                        rhs: SQLExpression::Binary(
                            BinaryOperatorExpression {
                                operator: BinaryOperator::Mul,
                                lhs: SQLExpression::Integer(2),
                                rhs: SQLExpression::Integer(3),
                            }
                            .into(),
                        ),
                    }
                    .into(),
                ))
                .set_alias("foo".into())
                .build(),
        )
        .build();

    assert_eq!(parser.parse().unwrap(), vec![expected],);
}

// #[test]
// pub fn expression_2() {
//     let text = r#"
//         SELECT 2 * (3 + 5) AS foo
//     "#
//     .to_owned();

//     let mut parser = Parser::new(text).unwrap();

//     let expected = SelectQuery::builder()
//         .add_select_item(
//             SelectItem::builder()
//                 .set_item(SQLExpression::Binary(
//                     BinaryOperatorExpression {
//                         operator: BinaryOperator::Mul,
//                         lhs: SQLExpression::Integer(2),
//                         rhs: SQLExpression::Binary(
//                             BinaryOperatorExpression {
//                                 operator: BinaryOperator::Add,
//                                 lhs: SQLExpression::Integer(3),
//                                 rhs: SQLExpression::Integer(5),
//                             }
//                             .into(),
//                         ),
//                     }
//                     .into(),
//                 ))
//                 .set_alias("foo".into())
//                 .build(),
//         )
//         .build();

//     assert_eq!(parser.parse().unwrap(), vec![expected],);
// }
