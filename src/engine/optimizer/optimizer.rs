use std::collections::HashMap;

use crate::engine::ast::dml::delete::DeleteQuery;
use crate::engine::ast::dml::expressions::operators::{BinaryOperator, UnaryOperator};
use crate::engine::ast::dml::parts::from::FromTarget;
use crate::engine::ast::dml::plan::delete::delete_plan::DeletePlan;
use crate::engine::ast::dml::plan::delete::from::DeleteFromPlan;
use crate::engine::ast::dml::plan::select::filter::FilterPlan;
use crate::engine::ast::dml::plan::select::from::SelectFromPlan;
use crate::engine::ast::dml::plan::select::limit_offset::LimitOffsetPlan;
use crate::engine::ast::dml::plan::select::scan::{IndexScanPlan, ScanType};
use crate::engine::ast::dml::plan::select::select_plan::{SelectPlan, SelectPlanItem};
use crate::engine::ast::dml::plan::update::from::UpdateFromPlan;
use crate::engine::ast::dml::plan::update::update_plan::UpdatePlan;
use crate::engine::ast::dml::select::SelectQuery;
use crate::engine::ast::dml::update::UpdateQuery;
use crate::engine::ast::types::{SQLExpression, SelectColumn, TableName};
use crate::engine::index::{IndexMeta, field_to_key};
use crate::engine::optimizer::cost;
use crate::engine::optimizer::statistics::TableStatistics;
use crate::engine::schema::row::TableDataFieldType;
use crate::errors;

/// 옵티마이저가 계획을 세울 때 참조하는 컨텍스트 (#195)
///
/// FROM 대상 테이블의 인덱스 목록과 통계 정보를 담습니다.
/// 컨텍스트가 비어 있으면 항상 FullScan으로 계획합니다.
#[derive(Debug, Default)]
pub struct OptimizerContext {
    pub indexes: Vec<IndexMeta>,
    pub statistics: Option<TableStatistics>,
}

/// WHERE 절 분석으로 얻은 컬럼별 키 경계
#[derive(Debug, Default, Clone)]
struct ColumnBounds {
    eq_key: Option<String>,
    /// 시작 키 (포함)
    start_key: Option<String>,
    /// 끝 키 (제외)
    end_key: Option<String>,
}

pub struct Optimizer {
    context: OptimizerContext,
}

impl Optimizer {
    pub fn new() -> Self {
        Self {
            context: OptimizerContext::default(),
        }
    }

    pub fn with_context(context: OptimizerContext) -> Self {
        Self { context }
    }

    pub async fn optimize_select(&self, query: SelectQuery) -> errors::Result<SelectPlan> {
        let mut has_from = false;
        let mut plan = SelectPlan { list: vec![] };

        // 스캔 방식 결정 (조인 쿼리는 아직 FullScan만 지원)
        // TODO(#195): 조인 실행기가 구현되면 조인 순서 최적화 추가
        let scan = if query.join_clause.is_empty() {
            match &query.from_table {
                Some(from_clause) => match &from_clause.from {
                    FromTarget::Table(table_name) => self.choose_scan(
                        table_name,
                        from_clause.alias.as_ref(),
                        query.where_clause.as_ref().map(|w| &w.expression),
                    ),
                    FromTarget::Subquery(_) => ScanType::FullScan,
                },
                None => ScanType::FullScan,
            }
        } else {
            ScanType::FullScan
        };

        // FROM 절 분석
        if let Some(from_clause) = query.from_table {
            has_from = true;
            let alias = from_clause.alias;

            match from_clause.from {
                FromTarget::Table(table_name) => plan.list.push(
                    SelectFromPlan {
                        table_name,
                        alias,
                        scan,
                    }
                    .into(),
                ),
                FromTarget::Subquery(_subquery) => {}
            }
        }

        if has_from {
            // JOIN 절 구성
            if !query.join_clause.is_empty() {
                // TODO
            }

            // WHERE 절 필터링 구성
            // 인덱스 스캔이 선택되어도 필터는 유지됩니다 (잔여 조건 처리 및 정합성 보장)
            if let Some(where_clause) = query.where_clause {
                let expression = where_clause.expression;

                plan.list.push(FilterPlan { expression }.into());
            }

            // GROUP BY 절 구성
            if let Some(group_by_clause) = query.group_by_clause {
                plan.list.push(group_by_clause.into());

                // HAVING 절 구성
                if let Some(having_clause) = query.having_clause {
                    plan.list.push(
                        FilterPlan {
                            expression: *having_clause.expression,
                        }
                        .into(),
                    );
                }
            } else if query.has_aggregate {
                plan.list.push(SelectPlanItem::GroupAll);
            }

            // ORDER BY 절 구성
            if let Some(order_by_clause) = query.order_by_clause {
                plan.list.push(order_by_clause.into());
            }

            // LIMIT OFFSET 절 구성
            if query.limit.is_some() || query.offset.is_some() {
                plan.list.push(
                    LimitOffsetPlan {
                        limit: query.limit,
                        offset: query.offset,
                    }
                    .into(),
                );
            }
        }

        Ok(plan)
    }

    pub async fn optimize_update(&self, query: UpdateQuery) -> errors::Result<UpdatePlan> {
        let mut plan = UpdatePlan { list: vec![] };

        let target_table = query.target_table.clone().unwrap();

        let scan = self.choose_scan(
            &target_table.table,
            target_table.alias.as_ref(),
            query.where_clause.as_ref().map(|w| &w.expression),
        );

        plan.list.push(
            UpdateFromPlan {
                table_name: target_table.table.clone(),
                alias: target_table.alias,
                scan,
            }
            .into(),
        );

        // WHERE 절 분석
        if let Some(where_clause) = query.where_clause {
            // WHERE 절 필터링 구성

            let expression = where_clause.expression;

            plan.list.push(FilterPlan { expression }.into());
        }

        Ok(plan)
    }

    pub async fn optimize_delete(&self, query: DeleteQuery) -> errors::Result<DeletePlan> {
        let mut plan = DeletePlan { list: vec![] };

        let target_table = query.from_table.clone().unwrap();

        let scan = self.choose_scan(
            &target_table.table,
            target_table.alias.as_ref(),
            query.where_clause.as_ref().map(|w| &w.expression),
        );

        plan.list.push(
            DeleteFromPlan {
                table_name: target_table.table.clone(),
                alias: target_table.alias,
                scan,
            }
            .into(),
        );

        // WHERE 절 분석
        if let Some(where_clause) = query.where_clause {
            // WHERE 절 필터링 구성

            let expression = where_clause.expression;

            plan.list.push(FilterPlan { expression }.into());
        }

        Ok(plan)
    }

    /// 비용 기반으로 FullScan / IndexScan 중 하나를 선택합니다. (#195)
    ///
    /// 1. WHERE 절을 AND 단위로 분해 (heuristic predicate pushdown)
    /// 2. 인덱스 컬럼에 대한 sargable 조건(=, >, >=, <, <=, BETWEEN)을 추출
    /// 3. 인덱스별 선택도/비용을 계산해 풀 스캔 비용보다 저렴한 최적 인덱스를 선택
    fn choose_scan(
        &self,
        table_name: &TableName,
        alias: Option<&String>,
        where_expression: Option<&SQLExpression>,
    ) -> ScanType {
        let statistics = match &self.context.statistics {
            Some(statistics) => statistics,
            None => return ScanType::FullScan,
        };

        let where_expression = match where_expression {
            Some(expression) => expression,
            None => return ScanType::FullScan,
        };

        if self.context.indexes.is_empty() {
            return ScanType::FullScan;
        }

        // WHERE 절을 AND 단위 조건으로 분해
        let mut conjuncts = vec![];
        collect_conjuncts(where_expression, &mut conjuncts);

        // 컬럼별 키 경계 수집
        let mut bounds_per_column: HashMap<String, ColumnBounds> = HashMap::new();

        for conjunct in conjuncts {
            if let Some((column_name, bounds)) =
                extract_sargable_bounds(conjunct, table_name, alias)
            {
                merge_bounds(bounds_per_column.entry(column_name).or_default(), bounds);
            }
        }

        if bounds_per_column.is_empty() {
            return ScanType::FullScan;
        }

        let full_cost = cost::full_scan_cost(statistics.row_count, statistics.block_count);
        let mut best: Option<(f64, IndexScanPlan)> = None;

        for index in &self.context.indexes {
            let bounds = match bounds_per_column.get(&index.column_name) {
                Some(bounds) => bounds,
                None => continue,
            };

            let selectivity = if bounds.eq_key.is_some() {
                if index.is_unique {
                    1.0 / statistics.row_count.max(1) as f64
                } else {
                    cost::eq_selectivity(
                        statistics.distinct_values.get(&index.column_name).copied(),
                    )
                }
            } else if bounds.start_key.is_some() || bounds.end_key.is_some() {
                cost::DEFAULT_RANGE_SELECTIVITY
            } else {
                continue;
            };

            let scan_cost = cost::index_scan_cost(statistics.row_count, selectivity);

            if scan_cost < full_cost && best.as_ref().map(|(c, _)| scan_cost < *c).unwrap_or(true) {
                best = Some((
                    scan_cost,
                    IndexScanPlan {
                        index_name: index.index_name.clone(),
                        column_name: index.column_name.clone(),
                        eq_key: bounds.eq_key.clone(),
                        start_key: if bounds.eq_key.is_some() {
                            None
                        } else {
                            bounds.start_key.clone()
                        },
                        end_key: if bounds.eq_key.is_some() {
                            None
                        } else {
                            bounds.end_key.clone()
                        },
                    },
                ));
            }
        }

        match best {
            Some((_, plan)) => ScanType::IndexScan(plan),
            None => ScanType::FullScan,
        }
    }
}

impl Default for Optimizer {
    fn default() -> Self {
        Self::new()
    }
}

/// AND로 연결된 표현식을 개별 조건으로 분해합니다.
fn collect_conjuncts<'a>(expression: &'a SQLExpression, out: &mut Vec<&'a SQLExpression>) {
    match expression {
        SQLExpression::Binary(binary) if binary.operator == BinaryOperator::And => {
            collect_conjuncts(&binary.lhs, out);
            collect_conjuncts(&binary.rhs, out);
        }
        SQLExpression::Parentheses(parentheses) => {
            collect_conjuncts(&parentheses.expression, out);
        }
        _ => out.push(expression),
    }
}

/// 리터럴 표현식을 인덱스 키로 변환 가능한 값으로 평가합니다.
fn literal_to_field(expression: &SQLExpression) -> Option<TableDataFieldType> {
    match expression {
        SQLExpression::Integer(value) => Some(TableDataFieldType::Integer(*value)),
        SQLExpression::Float(value) => Some(TableDataFieldType::Float((*value).into())),
        SQLExpression::Boolean(value) => Some(TableDataFieldType::Boolean(*value)),
        SQLExpression::String(value) => Some(TableDataFieldType::String(value.clone())),
        SQLExpression::Unary(unary) => match (&unary.operator, &unary.operand) {
            (UnaryOperator::Neg, SQLExpression::Integer(value)) => {
                Some(TableDataFieldType::Integer(-value))
            }
            (UnaryOperator::Neg, SQLExpression::Float(value)) => {
                Some(TableDataFieldType::Float((-value).into()))
            }
            (UnaryOperator::Pos, operand) => literal_to_field(operand),
            _ => None,
        },
        SQLExpression::Parentheses(parentheses) => literal_to_field(&parentheses.expression),
        _ => None,
    }
}

/// 컬럼 참조가 대상 테이블(또는 별칭)을 가리키는지 확인합니다.
fn column_matches(column: &SelectColumn, table_name: &TableName, alias: Option<&String>) -> bool {
    match &column.table_name {
        None => true,
        Some(name) => name == &table_name.table_name || Some(name) == alias,
    }
}

/// 키 문자열 바로 다음으로 정렬되는 배타 경계 키를 만듭니다.
fn exclusive_after(key: &str) -> String {
    format!("{}\u{0}", key)
}

/// 단일 조건에서 (컬럼명, 키 경계)를 추출합니다. sargable하지 않으면 None.
fn extract_sargable_bounds(
    expression: &SQLExpression,
    table_name: &TableName,
    alias: Option<&String>,
) -> Option<(String, ColumnBounds)> {
    match expression {
        SQLExpression::Binary(binary) => {
            let (column, literal, operator) = match (&binary.lhs, &binary.rhs) {
                (SQLExpression::SelectColumn(column), rhs) => {
                    (column, literal_to_field(rhs)?, binary.operator.clone())
                }
                (lhs, SQLExpression::SelectColumn(column)) => (
                    column,
                    literal_to_field(lhs)?,
                    flip_operator(&binary.operator)?,
                ),
                _ => return None,
            };

            if !column_matches(column, table_name, alias) {
                return None;
            }

            let key = field_to_key(&literal);

            let bounds = match operator {
                BinaryOperator::Eq => ColumnBounds {
                    eq_key: Some(key),
                    ..Default::default()
                },
                BinaryOperator::Gt => ColumnBounds {
                    start_key: Some(exclusive_after(&key)),
                    ..Default::default()
                },
                BinaryOperator::Gte => ColumnBounds {
                    start_key: Some(key),
                    ..Default::default()
                },
                BinaryOperator::Lt => ColumnBounds {
                    end_key: Some(key),
                    ..Default::default()
                },
                BinaryOperator::Lte => ColumnBounds {
                    end_key: Some(exclusive_after(&key)),
                    ..Default::default()
                },
                _ => return None,
            };

            Some((column.column_name.clone(), bounds))
        }
        SQLExpression::Between(between) => {
            let column = match &between.a {
                SQLExpression::SelectColumn(column) => column,
                _ => return None,
            };

            if !column_matches(column, table_name, alias) {
                return None;
            }

            let start = literal_to_field(&between.x)?;
            let end = literal_to_field(&between.y)?;

            Some((
                column.column_name.clone(),
                ColumnBounds {
                    eq_key: None,
                    start_key: Some(field_to_key(&start)),
                    end_key: Some(exclusive_after(&field_to_key(&end))),
                },
            ))
        }
        _ => None,
    }
}

/// 리터럴이 좌변에 있는 조건(5 < id 등)의 연산자를 뒤집습니다.
fn flip_operator(operator: &BinaryOperator) -> Option<BinaryOperator> {
    match operator {
        BinaryOperator::Eq => Some(BinaryOperator::Eq),
        BinaryOperator::Gt => Some(BinaryOperator::Lt),
        BinaryOperator::Gte => Some(BinaryOperator::Lte),
        BinaryOperator::Lt => Some(BinaryOperator::Gt),
        BinaryOperator::Lte => Some(BinaryOperator::Gte),
        _ => None,
    }
}

/// 같은 컬럼에 대한 여러 조건의 경계를 병합합니다 (start=max, end=min).
fn merge_bounds(existing: &mut ColumnBounds, new: ColumnBounds) {
    if existing.eq_key.is_none() {
        existing.eq_key = new.eq_key;
    }

    existing.start_key = match (existing.start_key.take(), new.start_key) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    };

    existing.end_key = match (existing.end_key.take(), new.end_key) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::ast::dml::expressions::binary::BinaryOperatorExpression;
    use crate::engine::parser::predule::{Parser, ParserContext};

    fn table() -> TableName {
        TableName::new(Some("rrdb".to_string()), "users".to_string())
    }

    fn index_meta(name: &str, column: &str, unique: bool) -> IndexMeta {
        IndexMeta::new(
            format!("rrdb.{}", name),
            table(),
            column.to_string(),
            unique,
        )
    }

    fn statistics(row_count: usize) -> TableStatistics {
        TableStatistics {
            row_count,
            block_count: row_count / 100 + 1,
            distinct_values: HashMap::from([("id".to_string(), row_count)]),
        }
    }

    fn context(row_count: usize, unique: bool) -> OptimizerContext {
        OptimizerContext {
            indexes: vec![index_meta("users_pkey", "id", unique)],
            statistics: Some(statistics(row_count)),
        }
    }

    fn eq_expression(column: &str, value: i64) -> SQLExpression {
        BinaryOperatorExpression {
            operator: BinaryOperator::Eq,
            lhs: SelectColumn::new(None, column.to_string()).into(),
            rhs: SQLExpression::Integer(value),
        }
        .into()
    }

    fn integer_key(value: i64) -> String {
        field_to_key(&TableDataFieldType::Integer(value))
    }

    #[test]
    fn choose_scan_picks_index_for_eq_predicate_on_large_table() {
        let optimizer = Optimizer::with_context(context(10_000, true));
        let expression = eq_expression("id", 42);

        let scan = optimizer.choose_scan(&table(), None, Some(&expression));

        match scan {
            ScanType::IndexScan(plan) => {
                assert_eq!(plan.index_name, "rrdb.users_pkey");
                assert_eq!(plan.column_name, "id");
                assert_eq!(plan.eq_key, Some(integer_key(42)));
                assert_eq!(plan.start_key, None);
                assert_eq!(plan.end_key, None);
            }
            other => panic!("expected IndexScan, got {:?}", other),
        }
    }

    #[test]
    fn choose_scan_prefers_full_scan_on_tiny_table() {
        let optimizer = Optimizer::with_context(context(3, true));
        let expression = eq_expression("id", 42);

        let scan = optimizer.choose_scan(&table(), None, Some(&expression));

        assert_eq!(scan, ScanType::FullScan);
    }

    #[test]
    fn choose_scan_full_scan_without_index_or_statistics() {
        let expression = eq_expression("id", 42);

        // 인덱스 없음
        let optimizer = Optimizer::with_context(OptimizerContext {
            indexes: vec![],
            statistics: Some(statistics(10_000)),
        });
        assert_eq!(
            optimizer.choose_scan(&table(), None, Some(&expression)),
            ScanType::FullScan
        );

        // 통계 없음
        let optimizer = Optimizer::with_context(OptimizerContext {
            indexes: vec![index_meta("users_pkey", "id", true)],
            statistics: None,
        });
        assert_eq!(
            optimizer.choose_scan(&table(), None, Some(&expression)),
            ScanType::FullScan
        );

        // WHERE 절 없음
        let optimizer = Optimizer::with_context(context(10_000, true));
        assert_eq!(
            optimizer.choose_scan(&table(), None, None),
            ScanType::FullScan
        );
    }

    #[test]
    fn choose_scan_full_scan_for_non_indexed_column() {
        let optimizer = Optimizer::with_context(context(10_000, true));
        let expression = eq_expression("name", 42);

        assert_eq!(
            optimizer.choose_scan(&table(), None, Some(&expression)),
            ScanType::FullScan
        );
    }

    #[test]
    fn choose_scan_merges_range_bounds_from_and_conjuncts() {
        // id > 10 AND id <= 20 AND name = 'a' (name은 인덱스 없음)
        let optimizer = Optimizer::with_context(OptimizerContext {
            indexes: vec![index_meta("users_pkey", "id", true)],
            statistics: Some(TableStatistics {
                row_count: 1_000_000,
                block_count: 10_000,
                distinct_values: HashMap::new(),
            }),
        });

        let expression: SQLExpression = BinaryOperatorExpression {
            operator: BinaryOperator::And,
            lhs: BinaryOperatorExpression {
                operator: BinaryOperator::Gt,
                lhs: SelectColumn::new(None, "id".to_string()).into(),
                rhs: SQLExpression::Integer(10),
            }
            .into(),
            rhs: BinaryOperatorExpression {
                operator: BinaryOperator::Lte,
                lhs: SelectColumn::new(None, "id".to_string()).into(),
                rhs: SQLExpression::Integer(20),
            }
            .into(),
        }
        .into();

        // 범위 스캔 선택도(1/3)로는 풀 스캔이 더 저렴할 수 있으므로 직접 경계만 검증
        let mut conjuncts = vec![];
        collect_conjuncts(&expression, &mut conjuncts);
        assert_eq!(conjuncts.len(), 2);

        let mut bounds = ColumnBounds::default();
        for conjunct in conjuncts {
            let (column, new_bounds) =
                extract_sargable_bounds(conjunct, &table(), None).expect("sargable");
            assert_eq!(column, "id");
            merge_bounds(&mut bounds, new_bounds);
        }

        assert_eq!(bounds.eq_key, None);
        assert_eq!(bounds.start_key, Some(exclusive_after(&integer_key(10))));
        assert_eq!(bounds.end_key, Some(exclusive_after(&integer_key(20))));
    }

    #[test]
    fn extract_sargable_bounds_handles_flipped_operands() {
        // 42 = id
        let expression: SQLExpression = BinaryOperatorExpression {
            operator: BinaryOperator::Eq,
            lhs: SQLExpression::Integer(42),
            rhs: SelectColumn::new(None, "id".to_string()).into(),
        }
        .into();

        let (column, bounds) = extract_sargable_bounds(&expression, &table(), None).unwrap();
        assert_eq!(column, "id");
        assert_eq!(bounds.eq_key, Some(integer_key(42)));

        // 42 < id → id > 42
        let expression: SQLExpression = BinaryOperatorExpression {
            operator: BinaryOperator::Lt,
            lhs: SQLExpression::Integer(42),
            rhs: SelectColumn::new(None, "id".to_string()).into(),
        }
        .into();

        let (_, bounds) = extract_sargable_bounds(&expression, &table(), None).unwrap();
        assert_eq!(bounds.start_key, Some(exclusive_after(&integer_key(42))));
    }

    #[test]
    fn extract_sargable_bounds_rejects_other_table_column() {
        let expression: SQLExpression = BinaryOperatorExpression {
            operator: BinaryOperator::Eq,
            lhs: SelectColumn::new(Some("other_table".to_string()), "id".to_string()).into(),
            rhs: SQLExpression::Integer(42),
        }
        .into();

        assert!(extract_sargable_bounds(&expression, &table(), None).is_none());

        // 별칭은 허용
        let expression: SQLExpression = BinaryOperatorExpression {
            operator: BinaryOperator::Eq,
            lhs: SelectColumn::new(Some("u".to_string()), "id".to_string()).into(),
            rhs: SQLExpression::Integer(42),
        }
        .into();

        let alias = "u".to_string();
        assert!(extract_sargable_bounds(&expression, &table(), Some(&alias)).is_some());
    }

    #[test]
    fn literal_to_field_handles_negative_numbers() {
        let mut parser = Parser::with_string("select 1 from t where id = -5;".into()).unwrap();
        let statements = parser
            .parse(ParserContext::default().set_default_database("rrdb".to_string()))
            .unwrap();

        // 파서가 음수를 어떤 형태로 만들든 literal_to_field가 처리해야 함
        if let crate::engine::ast::SQLStatement::DML(
            crate::engine::ast::DMLStatement::SelectQuery(query),
        ) = &statements[0]
        {
            let expression = &query.where_clause.as_ref().unwrap().expression;
            let mut conjuncts = vec![];
            collect_conjuncts(expression, &mut conjuncts);
            let (_, bounds) =
                extract_sargable_bounds(conjuncts[0], &table(), None).expect("sargable");
            assert_eq!(
                bounds.eq_key,
                Some(field_to_key(&TableDataFieldType::Integer(-5)))
            );
        } else {
            panic!("expected select query");
        }
    }

    #[tokio::test]
    async fn optimize_select_emits_index_scan_with_residual_filter() {
        let mut parser =
            Parser::with_string("select * from users where id = 42 and name = 'x';".into())
                .unwrap();
        let statements = parser
            .parse(ParserContext::default().set_default_database("rrdb".to_string()))
            .unwrap();

        let query = match statements.into_iter().next().unwrap() {
            crate::engine::ast::SQLStatement::DML(
                crate::engine::ast::DMLStatement::SelectQuery(query),
            ) => query,
            other => panic!("expected select query, got {:?}", other),
        };

        let optimizer = Optimizer::with_context(context(10_000, true));
        let plan = optimizer.optimize_select(query).await.unwrap();

        // From(IndexScan) + Filter 두 항목이 있어야 함
        assert_eq!(plan.list.len(), 2);

        match &plan.list[0] {
            SelectPlanItem::From(from) => match &from.scan {
                ScanType::IndexScan(index_scan) => {
                    assert_eq!(index_scan.eq_key, Some(integer_key(42)));
                }
                other => panic!("expected IndexScan, got {:?}", other),
            },
            other => panic!("expected From plan, got {:?}", other),
        }

        assert!(matches!(plan.list[1], SelectPlanItem::Filter(_)));
    }

    #[tokio::test]
    async fn optimize_select_keeps_full_scan_for_join_queries() {
        let mut parser = Parser::with_string(
            "select * from users u inner join orders o on u.id = o.user_id where u.id = 42;".into(),
        )
        .unwrap();
        let statements = parser
            .parse(ParserContext::default().set_default_database("rrdb".to_string()))
            .unwrap();

        let query = match statements.into_iter().next().unwrap() {
            crate::engine::ast::SQLStatement::DML(
                crate::engine::ast::DMLStatement::SelectQuery(query),
            ) => query,
            other => panic!("expected select query, got {:?}", other),
        };

        let optimizer = Optimizer::with_context(context(10_000, true));
        let plan = optimizer.optimize_select(query).await.unwrap();

        match &plan.list[0] {
            SelectPlanItem::From(from) => assert_eq!(from.scan, ScanType::FullScan),
            other => panic!("expected From plan, got {:?}", other),
        }
    }
}
