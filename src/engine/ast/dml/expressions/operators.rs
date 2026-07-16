use serde::{Deserialize, Serialize};

// 2항연산자
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum BinaryOperator {
    Add,     // A + B
    Sub,     // A - B
    Mul,     // A * B
    Div,     // A / B
    And,     // A AND B
    Or,      // A OR B
    Lt,      // A < B
    Gt,      // A > B
    Lte,     // A <= B
    Gte,     // A >= B
    Eq,      // A = B
    Neq,     // A != B, A <> B
    Like,    // A LIKE B
    NotLike, // A NOT LIKE B
    In,      // A In B
    NotIn,   // A Not In B
    Is,      // A Is B
    IsNot,   // A Is Not B
}

// 단항연산자
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum UnaryOperator {
    Pos, // +A
    Neg, // -A
    Not, // Not A
}

impl BinaryOperator {
    // 2항연산자 우선순위 획득
    // 표준 SQL 우선순위: OR < AND < 비교 연산자 < 덧셈/뺄셈 < 곱셈/나눗셈
    pub fn get_precedence(&self) -> i32 {
        match self {
            BinaryOperator::Or => 1,
            BinaryOperator::And => 2,
            BinaryOperator::Lt => 5,
            BinaryOperator::Gt => 5,
            BinaryOperator::Lte => 5,
            BinaryOperator::Gte => 5,
            BinaryOperator::Eq => 5,
            BinaryOperator::Neq => 5,
            BinaryOperator::Like => 5,
            BinaryOperator::NotLike => 5,
            BinaryOperator::In => 5,
            BinaryOperator::NotIn => 5,
            BinaryOperator::Is => 5,
            BinaryOperator::IsNot => 5,
            BinaryOperator::Add => 10,
            BinaryOperator::Sub => 10,
            BinaryOperator::Mul => 40,
            BinaryOperator::Div => 40,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BinaryOperator;
    use crate::engine::parser::predule::Parser;

    #[test]
    fn test_get_precedence() {
        assert_eq!(BinaryOperator::Add.get_precedence(), 10);
        assert_eq!(BinaryOperator::Sub.get_precedence(), 10);
        assert_eq!(BinaryOperator::Mul.get_precedence(), 40);
        assert_eq!(BinaryOperator::Div.get_precedence(), 40);
        assert_eq!(BinaryOperator::And.get_precedence(), 2);
        assert_eq!(BinaryOperator::Or.get_precedence(), 1);
        assert_eq!(BinaryOperator::Lt.get_precedence(), 5);
        assert_eq!(BinaryOperator::Gt.get_precedence(), 5);
        assert_eq!(BinaryOperator::Lte.get_precedence(), 5);
        assert_eq!(BinaryOperator::Gte.get_precedence(), 5);
        assert_eq!(BinaryOperator::Eq.get_precedence(), 5);
        assert_eq!(BinaryOperator::Neq.get_precedence(), 5);
        assert_eq!(BinaryOperator::Like.get_precedence(), 5);
        assert_eq!(BinaryOperator::NotLike.get_precedence(), 5);
        assert_eq!(BinaryOperator::In.get_precedence(), 5);
        assert_eq!(BinaryOperator::NotIn.get_precedence(), 5);
        assert_eq!(BinaryOperator::Is.get_precedence(), 5);
        assert_eq!(BinaryOperator::IsNot.get_precedence(), 5);
    }

    /// 회귀 테스트: AND/OR/비교 연산자가 섞인 표현식이 올바른 결합 순서로 파싱되는지 검증합니다.
    /// BinaryOperator::get_precedence의 값 변경 시 이 테스트가 깨져야 합니다.
    mod operator_precedence_parsing {
        use crate::engine::ast::dml::expressions::operators::BinaryOperator;
        use crate::engine::ast::types::SQLExpression;
        use crate::engine::parser::predule::{Parser, ParserContext};

        fn parse_where(sql: &str) -> SQLExpression {
            let full_sql = format!("select * from t where {}", sql);
            let mut parser = Parser::with_string(full_sql).unwrap();
            let stmts = parser.parse(ParserContext::default()).unwrap();
            let select = match stmts.into_iter().next().unwrap() {
                crate::engine::ast::SQLStatement::DML(
                    crate::engine::ast::DMLStatement::SelectQuery(q),
                ) => q,
                _ => panic!("expected select"),
            };
            select.where_clause.unwrap().expression
        }

        fn assert_binary(expr: &SQLExpression, op: BinaryOperator) -> bool {
            matches!(expr, SQLExpression::Binary(b) if b.operator == op)
        }

        #[test]
        fn and_binds_tighter_than_or() {
            // a OR b AND c  →  a OR (b AND c)
            let expr = parse_where("a = 1 or b = 2 and c = 3");
            // 최상위는 OR, 오른쪽 피연산자는 AND여야 함
            assert!(
                assert_binary(&expr, BinaryOperator::Or),
                "expected OR at top level"
            );

            if let SQLExpression::Binary(b) = &expr {
                assert!(
                    assert_binary(&b.lhs, BinaryOperator::Eq),
                    "left side of OR should be comparison"
                );
                assert!(
                    assert_binary(&b.rhs, BinaryOperator::And),
                    "right side of OR should be AND (b = 2 AND c = 3)"
                );
            }
        }

        #[test]
        fn comparisons_bind_tighter_than_and() {
            // a > b AND c = d  →  (a > b) AND (c = d)
            let expr = parse_where("a > b and c = d");
            assert!(
                assert_binary(&expr, BinaryOperator::And),
                "expected AND at top level"
            );

            if let SQLExpression::Binary(b) = &expr {
                assert!(
                    assert_binary(&b.lhs, BinaryOperator::Gt),
                    "left side of AND should be > comparison"
                );
                assert!(
                    assert_binary(&b.rhs, BinaryOperator::Eq),
                    "right side of AND should be = comparison"
                );
            }
        }

        #[test]
        fn arithmetic_binds_tighter_than_comparison() {
            // a + b > c  →  (a + b) > c
            let expr = parse_where("a + b > c");
            assert!(
                assert_binary(&expr, BinaryOperator::Gt),
                "expected > at top level for arithmetic+comparison"
            );
        }
    }
}
