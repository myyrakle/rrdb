use std::collections::HashSet;
use std::error::Error;
use std::iter::FromIterator;

use crate::ast::dml::{OrderByNulls, SelectWildCard};
use crate::ast::predule::{
    GroupByItem, HavingClause, JoinClause, JoinType, OrderByItem, OrderByType, SelectItem,
    SelectQuery, WhereClause,
};
use crate::errors::predule::ParsingError;
use crate::lexer::predule::{OperatorToken, Token};
use crate::parser::predule::{Parser, ParserContext};

impl Parser {
    pub(crate) fn handle_select_query(
        &mut self,
        context: ParserContext,
    ) -> Result<SelectQuery, Box<dyn Error + Send>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0301: need more tokens"));
        }

        // SELECT 토큰 삼키기
        let current_token = self.get_next_token();

        if current_token != Token::Select {
            return Err(ParsingError::boxed(format!(
                "E0302: expected 'SELECT'. but your input word is '{:?}'",
                current_token
            )));
        }

        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0303: need more tokens"));
        }

        let mut query_builder = SelectQuery::builder();

        // FROM 절이나 세미콜론이 나오기 전까지 select 절 파싱
        loop {
            if !self.has_next_token() {
                break;
            }

            let current_token = self.get_next_token();

            match current_token {
                Token::From => {
                    // from 다시 집어넣고 종료
                    self.unget_next_token(current_token);
                    break;
                }
                Token::SemiColon => {
                    // from 없는 select절로 간주. 종료.
                    return Ok(query_builder.build());
                }
                Token::Comma => continue,
                Token::Operator(OperatorToken::Asterisk) => {
                    query_builder =
                        query_builder.add_select_wildcard(SelectWildCard { alias: None });
                    continue;
                }
                _ => {
                    self.unget_next_token(current_token);
                    let select_item = self.parse_select_item(context.clone())?;
                    query_builder = query_builder.add_select_item(select_item);
                }
            }
        }

        if !self.has_next_token() {
            return Ok(query_builder.build());
        }

        // FROM 절 파싱
        let current_token = self.get_next_token();

        match current_token {
            Token::From => {
                if self.next_token_is_left_parentheses() {
                    let subquery = self.parse_subquery(context.clone())?;
                    query_builder = query_builder.set_from_subquery(subquery);
                } else {
                    let table_name = self.parse_table_name(context.clone())?;
                    query_builder = query_builder.set_from_table(table_name);
                }

                if self.next_token_is_table_alias() {
                    let alias = self.parse_table_alias()?;
                    query_builder = query_builder.set_from_alias(alias);
                }
            }
            _ => {
                return Err(ParsingError::boxed(format!(
                    "E0304 expected 'FROM' clause. but your input word is '{:?}'",
                    current_token
                )));
            }
        }

        // JOIN 절 파싱
        while let Some(join_type) = self.get_next_join_type() {
            let join = self.parse_join(join_type, context.clone())?;
            query_builder = query_builder.add_join(join);
        }

        // WHERE 절 파싱
        if self.next_token_is_where() {
            let where_clause = self.parse_where(context.clone())?;
            query_builder = query_builder.set_where(where_clause);
        }

        // Group By 절 파싱
        if self.next_token_is_group_by() {
            // GROUP BY 삼킴
            self.get_next_token();
            self.get_next_token();

            loop {
                if !self.has_next_token() {
                    break;
                }

                let current_token = self.get_next_token();

                match current_token {
                    Token::SemiColon => {
                        return Ok(query_builder.build());
                    }
                    Token::Comma => continue,
                    Token::Having | Token::Limit | Token::Offset | Token::Order => {
                        self.unget_next_token(current_token);
                        break;
                    }
                    _ => {
                        if current_token.is_expression() {
                            self.unget_next_token(current_token);
                            let group_by_item = self.parse_group_by_item(context.clone())?;
                            query_builder = query_builder.add_group_by(group_by_item);
                        } else {
                            return Err(ParsingError::boxed(format!(
                                "E0319 unexpected token '{:?}'",
                                current_token
                            )));
                        }
                    }
                }
            }
        }

        if !query_builder.select_items.is_empty() {
            // 집계 함수 <> GROUP BY 불일치 검증
            if query_builder.has_aggregate() {
                query_builder = query_builder.set_has_aggregate(true);

                let group_by_columns = match query_builder.group_by_clause {
                    Some(ref clause) => HashSet::from_iter(
                        clause.group_by_items.clone().into_iter().map(|e| e.item),
                    ),
                    None => HashSet::new(),
                };

                // 집계함수가 사용되지 않은 select column 목록
                let non_aggregate_columns = query_builder.get_non_aggregate_column();

                // 집계함수가 사용되지 않은 컬럼이 group by에 없다면 오류
                for non_aggregate_column in non_aggregate_columns {
                    if !group_by_columns.contains(&non_aggregate_column) {
                        return Err(ParsingError::boxed(format!(
                            "E0331: column '{:?}' must be in a GROUP BY clause or used within an aggregate function",
                            non_aggregate_column
                        )));
                    }
                }

                // 집계함수가 사용된 select column 목록
                let aggregate_columns = query_builder.get_aggregate_column();

                // 집계함수가 사용된 컬럼이 group by에 있다면 오류
                for aggregate_column in aggregate_columns {
                    if group_by_columns.contains(&aggregate_column) {
                        return Err(ParsingError::boxed(format!(
                            "E0332: column '{:?}' cannot be in a GROUP BY clause",
                            group_by_columns
                        )));
                    }
                }
            }
        }

        // Having 절 파싱
        if self.next_token_is_having() {
            if query_builder.has_group_by() {
                let having_clause = self.parse_having(context.clone())?;
                query_builder = query_builder.set_having(having_clause);
            } else {
                return Err(ParsingError::boxed(
                    "E0315 Having without group by is invalid.",
                ));
            }
        }

        // Order By 절 파싱
        if self.next_token_is_order_by() {
            // ORDER BY 삼킴
            self.get_next_token();
            self.get_next_token();

            loop {
                if !self.has_next_token() {
                    break;
                }

                let current_token = self.get_next_token();

                match current_token {
                    Token::SemiColon => {
                        return Ok(query_builder.build());
                    }
                    Token::Comma => continue,
                    Token::Group | Token::Limit | Token::Offset => {
                        self.unget_next_token(current_token);
                        break;
                    }
                    _ => {
                        if current_token.is_expression() {
                            self.unget_next_token(current_token);
                            let order_by_item = self.parse_order_by_item(context.clone())?;
                            query_builder = query_builder.add_order_by(order_by_item);
                        } else {
                            return Err(ParsingError::boxed(format!(
                                "E0318 unexpected token '{:?}'",
                                current_token
                            )));
                        }
                    }
                }
            }
        }

        // Limit & Offset 절 파싱
        // Offset이 먼저인 경우와, Limit이 먼저인 경우 둘다 대응
        if self.next_token_is_offset() {
            let offset = self.parse_offset(context.clone())?;
            query_builder = query_builder.set_offset(offset);

            if self.next_token_is_limit() {
                let limit = self.parse_limit(context)?;
                query_builder = query_builder.set_limit(limit);
            }
        } else if self.next_token_is_limit() {
            let limit = self.parse_limit(context.clone())?;
            query_builder = query_builder.set_limit(limit);

            if self.next_token_is_offset() {
                let offset = self.parse_offset(context)?;
                query_builder = query_builder.set_offset(offset);
            }
        }

        Ok(query_builder.build())
    }

    pub(crate) fn parse_select_item(
        &mut self,
        context: ParserContext,
    ) -> Result<SelectItem, Box<dyn Error + Send>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0305 need more tokens"));
        }

        let select_item = SelectItem::builder();

        // 표현식 파싱
        let select_item = select_item.set_item(self.parse_expression(context)?);

        // 더 없을 경우 바로 반환
        if !self.has_next_token() {
            return Ok(select_item.build());
        }

        let current_token = self.get_next_token();

        match current_token {
            Token::As => {
                // 더 없을 경우 바로 반환
                if !self.has_next_token() {
                    return Err(ParsingError::boxed("E0306 expected alias. need more"));
                }

                let current_token = self.get_next_token();

                match current_token {
                    Token::Identifier(identifier) => {
                        let select_item = select_item.set_alias(identifier);
                        Ok(select_item.build())
                    }
                    _ => Err(ParsingError::boxed(format!(
                        "E0307 expected alias, but your input word is '{:?}'",
                        current_token
                    ))),
                }
            }
            Token::Comma => {
                self.unget_next_token(current_token);
                // 현재 select_item은 종료된 것으로 판단.
                Ok(select_item.build())
            }
            Token::From => {
                self.unget_next_token(current_token);
                // 현재 select_item은 종료된 것으로 판단.
                Ok(select_item.build())
            }
            _ => Err(ParsingError::boxed(format!(
                "E0308 expected expression. but your input word is '{:?}'",
                current_token
            ))),
        }
    }

    pub(crate) fn parse_order_by_item(
        &mut self,
        context: ParserContext,
    ) -> Result<OrderByItem, Box<dyn Error + Send>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0313 need more tokens"));
        }

        // 표현식 파싱
        let item = self.parse_expression(context)?;

        let mut order_by_item = OrderByItem {
            item,
            order_type: OrderByType::Asc,
            nulls: OrderByNulls::First,
        };

        // 더 없을 경우 바로 반환
        if !self.has_next_token() {
            return Ok(order_by_item);
        }

        let current_token = self.get_next_token();

        match current_token {
            Token::Asc => {
                order_by_item.order_type = OrderByType::Asc;
            }
            Token::Desc => {
                order_by_item.order_type = OrderByType::Desc;
            }
            _ => {
                self.unget_next_token(current_token);
            }
        }

        // 더 없을 경우 바로 반환
        if !self.has_next_token() {
            return Ok(order_by_item);
        }

        let current_token = self.get_next_token();

        match current_token {
            Token::Nulls => {
                if !self.has_next_token() {
                    return Err(ParsingError::boxed("E0329 need more tokens"));
                }

                let current_token = self.get_next_token();

                match current_token {
                    Token::First => {}
                    Token::Last => order_by_item.nulls = OrderByNulls::Last,
                    _ => {
                        return Err(ParsingError::boxed(format!(
                            "E0330 expected keyword is FIRST or LAST, but your input is {:?}",
                            current_token
                        )))
                    }
                }

                Ok(order_by_item)
            }
            _ => {
                self.unget_next_token(current_token);
                Ok(order_by_item)
            }
        }
    }

    pub(crate) fn parse_group_by_item(
        &mut self,
        _context: ParserContext,
    ) -> Result<GroupByItem, Box<dyn Error + Send>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0314 need more tokens"));
        }

        // 표현식 파싱
        let item = self.parse_select_column()?;

        let order_by_item = GroupByItem { item };

        Ok(order_by_item)
    }

    pub(crate) fn parse_join(
        &mut self,
        join_type: JoinType,
        context: ParserContext,
    ) -> Result<JoinClause, Box<dyn Error + Send>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0310 need more tokens"));
        }

        let right = self.parse_table_name(context.clone())?;

        let right_alias = if self.next_token_is_table_alias() {
            self.parse_table_alias().ok()
        } else {
            None
        };

        let on = if !self.has_next_token() {
            None
        } else {
            let current_token = self.get_next_token();

            if current_token == Token::On {
                let expression = self.parse_expression(context)?;
                Some(expression)
            } else {
                self.unget_next_token(current_token);
                None
            }
        };

        let join = JoinClause {
            join_type,
            on,
            right,
            right_alias,
        };

        Ok(join)
    }

    pub(crate) fn parse_where(
        &mut self,
        context: ParserContext,
    ) -> Result<WhereClause, Box<dyn Error + Send>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0311 need more tokens"));
        }

        let current_token = self.get_next_token();

        if current_token != Token::Where {
            return Err(ParsingError::boxed(format!(
                "E0312 expected 'WHERE'. but your input word is '{:?}'",
                current_token
            )));
        }

        let expression = self.parse_expression(context)?;

        Ok(expression.into())
    }

    pub(crate) fn parse_having(
        &mut self,
        context: ParserContext,
    ) -> Result<HavingClause, Box<dyn Error + Send>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0316 need more tokens"));
        }

        let current_token = self.get_next_token();

        if current_token != Token::Having {
            return Err(ParsingError::boxed(format!(
                "E0317 expected 'Having'. but your input word is '{:?}'",
                current_token
            )));
        }

        let expression = self.parse_expression(context)?;

        Ok(HavingClause {
            expression: expression.into(),
        })
    }

    pub(crate) fn parse_offset(
        &mut self,
        _context: ParserContext,
    ) -> Result<u32, Box<dyn Error + Send>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0320 need more tokens"));
        }

        // OFFSET 삼키기
        let current_token = self.get_next_token();

        if current_token != Token::Offset {
            return Err(ParsingError::boxed(format!(
                "E0321 expected 'Offset'. but your input word is '{:?}'",
                current_token
            )));
        }

        // OFFSET 숫자값 획득
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0322 need more tokens"));
        }

        let current_token = self.get_next_token();

        match current_token {
            Token::Integer(integer) => {
                if integer >= 0 {
                    Ok(integer as u32)
                } else {
                    Err(ParsingError::boxed(
                        "E0323 Offset can only contain positive numbers.",
                    ))
                }
            }
            _ => Err(ParsingError::boxed(format!(
                "E0324 expected positive numbers. but your input word is '{:?}'",
                current_token
            ))),
        }
    }

    pub(crate) fn parse_limit(
        &mut self,
        _context: ParserContext,
    ) -> Result<u32, Box<dyn Error + Send>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0325 need more tokens"));
        }

        // OFFSET 삼키기
        let current_token = self.get_next_token();

        if current_token != Token::Limit {
            return Err(ParsingError::boxed(format!(
                "E0326 expected 'Limit'. but your input word is '{:?}'",
                current_token
            )));
        }

        // OFFSET 숫자값 획득
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E0327 need more tokens"));
        }

        let current_token = self.get_next_token();

        match current_token {
            Token::Integer(integer) => {
                if integer >= 0 {
                    Ok(integer as u32)
                } else {
                    Err(ParsingError::boxed(
                        "E0327 Limit can only contain positive numbers.",
                    ))
                }
            }
            _ => Err(ParsingError::boxed(format!(
                "E0328 expected positive numbers. but your input word is '{:?}'",
                current_token
            ))),
        }
    }
}
