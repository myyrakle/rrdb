

use std::collections::HashMap;
use std::error::Error;

use futures::future::join_all;
use itertools::Itertools;

use crate::lib::ast::dml::BinaryOperatorExpression;
use crate::lib::ast::predule::{SQLExpression, TableName, BinaryOperator, UnaryOperator, Column, BuiltInFunction, Function,  AggregateFunction};
use crate::lib::errors::predule::{TypeError, ExecuteError};
use crate::lib::executor::predule::{TableDataFieldType, TableDataRow, Executor, ExecuteColumnType};

#[derive(Debug, Default, Clone)]
pub struct ReduceContext {
    pub table_alias_map: HashMap<String, TableName>,
    pub row: Option<TableDataRow>,
    pub config_columns: Vec<(TableName, Column)>,
    pub total_count: usize,
}

impl Executor {
    #[async_recursion::async_recursion]
    pub async fn reduce_expression(
        &self,
        expression: SQLExpression,
        context: ReduceContext
    ) -> Result<TableDataFieldType, Box<dyn Error + Send>> {
        match expression {
            SQLExpression::Integer(value) => Ok(TableDataFieldType::Integer(value)),
            SQLExpression::Boolean(value) => Ok(TableDataFieldType::Boolean(value)),
            SQLExpression::Float(value) => Ok(TableDataFieldType::Float(value.into())),
            SQLExpression::String(value) => Ok(TableDataFieldType::String(value)),
            SQLExpression::Null => Ok(TableDataFieldType::Null),
            SQLExpression::List(list) =>  {
                let futures = list.value.into_iter().map(|e|{self.reduce_expression(e, context.clone())});
                let fields = join_all(futures).await.into_iter().collect::<Result<Vec<_>, 
                _>>()?;

                #[allow(unstable_name_collisions)]
                let serialized: String = fields.into_iter().map(|e|e.to_string()).intersperse(", ".to_owned()).collect();

                Ok(TableDataFieldType::String(format!("({})", serialized)))
            }
            SQLExpression::Unary(unary) => match unary.operator {
                UnaryOperator::Neg => {
                    let operand = self.reduce_expression(unary.operand, context).await?;

                    match operand {
                        TableDataFieldType::Integer(value) => {
                            Ok(TableDataFieldType::Integer(-value))
                        }
                        TableDataFieldType::Float(value) => {
                            Ok(TableDataFieldType::Float(-value))
                        }
                        TableDataFieldType::Array(mut array) => {
                            for e in &mut array {
                                match e {
                                    TableDataFieldType::Integer(value) => {
                                        *e = TableDataFieldType::Integer(-*value);
                                    }
                                    TableDataFieldType::Float(value) => {
                                        *e = TableDataFieldType::Float(-*value);
                                    }
                                    _ => return  Err(TypeError::dyn_boxed(
                                        "unary '!' operator is valid only for integer and float types.",
                                    )),
                                }
                            }
                            Ok(TableDataFieldType::Array(array))
                        }
                        _ => Err(TypeError::dyn_boxed(
                            "unary '-' operator is valid only for integer and float types.",
                        )),
                    }
                }
                UnaryOperator::Pos => {
                    let operand = self.reduce_expression(unary.operand, context).await?;

                    match operand {
                        TableDataFieldType::Integer(_) => Ok(operand),
                        TableDataFieldType::Float(_) => Ok(operand),
                        _ => Err(TypeError::dyn_boxed(
                            "unary '+' operator is valid only for integer and float types.",
                        )),
                    }
                }
                UnaryOperator::Not => {
                    let operand = self.reduce_expression(unary.operand, context).await?;

                    match operand {
                        TableDataFieldType::Boolean(value) => {
                            Ok(TableDataFieldType::Boolean(!value))
                        }
                        _ => Err(TypeError::dyn_boxed(
                            "unary '!' operator is valid only for integer and float types.",
                        )),
                    }
                }
            },
            SQLExpression::Binary(binary) => {
                let lhs = self.reduce_expression(binary.lhs.clone(), context.clone()).await?;
                let rhs = self.reduce_expression(binary.rhs.clone(), context.clone()).await?;

                if lhs.type_code() != rhs.type_code() {
                    return Err(TypeError::dyn_boxed(
                        "The types of lhs and rhs do not match.",
                    ));
                }

                if let TableDataFieldType::Array(ref left_array) = lhs {
                    if let TableDataFieldType::Array(ref right_array) = rhs{
                        let futures = (0..left_array.len()).into_iter().map(|i|
                           {
                            let binary = binary.clone(); 
                            let context = context.clone(); 
                       
                            async move {
                                let i = i.clone();

                                let expression = BinaryOperatorExpression {
                                    operator: binary.operator.clone(),
                                    lhs: left_array[i].clone().into(), 
                                    rhs: right_array[i].clone().into(),
                                };
                            
                                match self.reduce_expression(expression.into(), context.clone()).await {
                                    Ok(expression)=> Ok(expression), 
                                    Err(error)=>Err(error),
                                }
                            }
                        });
    
                        let result = join_all(futures).await.into_iter().collect::<Result<Vec<_>, _>>()?;
                        return Ok(TableDataFieldType::Array(result));
                    } else {
                        let futures = left_array.iter().map(|e|async {
                            let expression = BinaryOperatorExpression {
                                operator: binary.operator.clone(),
                                lhs: e.clone().into(), 
                                rhs: rhs.clone().into(),
                            };
                        
                            match self.reduce_expression(expression.into(), context.clone()).await {
                                Ok(expression)=> Ok(expression), 
                                Err(error)=>Err(error),
                            }
                        });

                        let result = join_all(futures).await.into_iter().collect::<Result<Vec<_>, _>>()?;
                        return Ok(TableDataFieldType::Array(result));
                    }
                } else if let TableDataFieldType::Array(ref right_array) = rhs{
                    let futures = right_array.iter().map(|e|async {
                        let expression = BinaryOperatorExpression {
                            operator: binary.operator.clone(),
                            lhs: lhs.clone().into(), 
                            rhs: e.clone().into(),
                        };
                    
                        match self.reduce_expression(expression.into(), context.clone()).await {
                            Ok(expression)=> Ok(expression), 
                            Err(error)=>Err(error),
                        }
                    });

                    let result = join_all(futures).await.into_iter().collect::<Result<Vec<_>, _>>()?;
                    return Ok(TableDataFieldType::Array(result));
                }

                match binary.operator {
                    BinaryOperator::Add => match lhs {
                        TableDataFieldType::Integer(lhs_value) => {
                            if let TableDataFieldType::Integer(rhs_value) = rhs {
                                return Ok(TableDataFieldType::Integer(lhs_value + rhs_value));
                            }
                            unreachable!()
                        }
                        TableDataFieldType::Float(lhs_value) => {
                            if let TableDataFieldType::Float(rhs_value) = rhs {
                                return Ok(TableDataFieldType::Float(lhs_value + rhs_value));
                            }
                            unreachable!()
                        }
                        TableDataFieldType::String(lhs_value) => {
                            if let TableDataFieldType::String(rhs_value) = rhs {
                                return Ok(TableDataFieldType::String(
                                    lhs_value + rhs_value.as_str(),
                                ));
                            }
                            unreachable!()
                        }
                        _ => Err(TypeError::dyn_boxed(
                            "binary '-' operator is valid only for integer and float and string types.",
                        )),
                    },
                    BinaryOperator::Sub => match lhs {
                        TableDataFieldType::Integer(lhs_value) => {
                            if let TableDataFieldType::Integer(rhs_value) = rhs {
                                return Ok(TableDataFieldType::Integer(lhs_value -rhs_value));
                            }
                            unreachable!()
                        }
                        TableDataFieldType::Float(lhs_value) => {
                            if let TableDataFieldType::Float(rhs_value) = rhs {
                                return Ok(TableDataFieldType::Float(lhs_value + rhs_value));
                            }
                            unreachable!()
                        }
                        _ => Err(TypeError::dyn_boxed(
                            "binary '-' operator is valid only for integer and float types.",
                        )),
                    },
                    BinaryOperator::Mul => match lhs {
                        TableDataFieldType::Integer(lhs_value) => {
                            if let TableDataFieldType::Integer(rhs_value) = rhs {
                                return Ok(TableDataFieldType::Integer(lhs_value *rhs_value));
                            }
                            unreachable!()
                        }
                        TableDataFieldType::Float(lhs_value) => {
                            if let TableDataFieldType::Float(rhs_value) = rhs {
                                return Ok(TableDataFieldType::Float(lhs_value * rhs_value));
                            }
                            unreachable!()
                        }
                        _ => Err(TypeError::dyn_boxed(
                            "binary '*' operator is valid only for integer and float types.",
                        )),
                    },
                    BinaryOperator::Div => match lhs {
                        TableDataFieldType::Integer(lhs_value) => {
                            if let TableDataFieldType::Integer(rhs_value) = rhs {
                                return Ok(TableDataFieldType::Integer(lhs_value / rhs_value));
                            }
                            unreachable!()
                        }
                        TableDataFieldType::Float(lhs_value) => {
                            if let TableDataFieldType::Float(rhs_value) = rhs {
                                return Ok(TableDataFieldType::Float(lhs_value / rhs_value));
                            }
                            unreachable!()
                        }
                        _ => Err(TypeError::dyn_boxed(
                            "binary '/' operator is valid only for integer and float types.",
                        )),
                    },
                    BinaryOperator::And => match lhs {
                        TableDataFieldType::Boolean(lhs_value) => {
                            if let TableDataFieldType::Boolean(rhs_value) = rhs {
                                return Ok(TableDataFieldType::Boolean(lhs_value && rhs_value));
                            }
                            unreachable!()
                        }
                        _ => Err(TypeError::dyn_boxed(
                            "binary 'And' operator is valid only for boolean type.",
                        )),
                    },
                    BinaryOperator::Or => match lhs {
                        TableDataFieldType::Boolean(lhs_value) => {
                            if let TableDataFieldType::Boolean(rhs_value) = rhs {
                                return Ok(TableDataFieldType::Boolean(lhs_value || rhs_value));
                            }
                            unreachable!()
                        }
                        _ => Err(TypeError::dyn_boxed(
                            "binary 'Or' operator is valid only for boolean type.",
                        )),
                    },
                    BinaryOperator::Lt => match lhs {
                        TableDataFieldType::Integer(lhs_value) => {
                            if let TableDataFieldType::Integer(rhs_value) = rhs {
                                return Ok(TableDataFieldType::Boolean(lhs_value < rhs_value));
                            }
                            unreachable!()
                        }
                        TableDataFieldType::Float(lhs_value) => {
                            if let TableDataFieldType::Float(rhs_value) = rhs {
                                return Ok(TableDataFieldType::Boolean(lhs_value < rhs_value));
                            }
                            unreachable!()
                        }
                        TableDataFieldType::String(lhs_value) => {
                            if let TableDataFieldType::String(rhs_value) = rhs {
                                return Ok(TableDataFieldType::Boolean(lhs_value < rhs_value));
                            }
                            unreachable!()
                        }
                        _ => Err(TypeError::dyn_boxed(
                            "binary '<' operator is valid only for integer and float and string types.",
                        )),
                    },
                    BinaryOperator::Gt => match lhs {
                        TableDataFieldType::Integer(lhs_value) => {
                            if let TableDataFieldType::Integer(rhs_value) = rhs {
                                return Ok(TableDataFieldType::Boolean(lhs_value > rhs_value));
                            }
                            unreachable!()
                        }
                        TableDataFieldType::Float(lhs_value) => {
                            if let TableDataFieldType::Float(rhs_value) = rhs {
                                return Ok(TableDataFieldType::Boolean(lhs_value > rhs_value));
                            }
                            unreachable!()
                        }
                        TableDataFieldType::String(lhs_value) => {
                            if let TableDataFieldType::String(rhs_value) = rhs {
                                return Ok(TableDataFieldType::Boolean(lhs_value > rhs_value));
                            }
                            unreachable!()
                        }
                        _ => Err(TypeError::dyn_boxed(
                            "binary '>' operator is valid only for integer and float and string types.",
                        )),
                    }, 
                    BinaryOperator::Lte => match lhs {
                        TableDataFieldType::Integer(lhs_value) => {
                            if let TableDataFieldType::Integer(rhs_value) = rhs {
                                return Ok(TableDataFieldType::Boolean(lhs_value <= rhs_value));
                            }
                            unreachable!()
                        }
                        TableDataFieldType::Float(lhs_value) => {
                            if let TableDataFieldType::Float(rhs_value) = rhs {
                                return Ok(TableDataFieldType::Boolean(lhs_value <= rhs_value));
                            }
                            unreachable!()
                        }
                        TableDataFieldType::String(lhs_value) => {
                            if let TableDataFieldType::String(rhs_value) = rhs {
                                return Ok(TableDataFieldType::Boolean(lhs_value <= rhs_value));
                            }
                            unreachable!()
                        }
                        _ => Err(TypeError::dyn_boxed(
                            "binary '<=' operator is valid only for integer and float and string types.",
                        )),
                    },
                    BinaryOperator::Gte => match lhs {
                        TableDataFieldType::Integer(lhs_value) => {
                            if let TableDataFieldType::Integer(rhs_value) = rhs {
                                return Ok(TableDataFieldType::Boolean(lhs_value >= rhs_value));
                            }
                            unreachable!()
                        }
                        TableDataFieldType::Float(lhs_value) => {
                            if let TableDataFieldType::Float(rhs_value) = rhs {
                                return Ok(TableDataFieldType::Boolean(lhs_value >= rhs_value));
                            }
                            unreachable!()
                        }
                        TableDataFieldType::String(lhs_value) => {
                            if let TableDataFieldType::String(rhs_value) = rhs {
                                return Ok(TableDataFieldType::Boolean(lhs_value >= rhs_value));
                            }
                            unreachable!()
                        }
                        _ => Err(TypeError::dyn_boxed(
                            "binary '>=' operator is valid only for integer and float and string types.",
                        )),
                    },
                    BinaryOperator::Eq =>
                         Ok(TableDataFieldType::Boolean(lhs == rhs)),    
                    BinaryOperator::Neq =>
                         Ok(TableDataFieldType::Boolean(lhs != rhs)),    
                    BinaryOperator::Like => unimplemented!("미구현"),   
                    BinaryOperator::NotLike => unimplemented!("미구현"),  
                    BinaryOperator::In => unimplemented!("미구현"),      
                    BinaryOperator::NotIn => unimplemented!("미구현"),  
                    BinaryOperator::Is => unimplemented!("미구현"),      
                    BinaryOperator::IsNot => unimplemented!("미구현"),  
                }
            }
            SQLExpression::Between(between) => {
                let _a =  self.reduce_expression(between.a, context.clone()).await?;
                let _x =  self.reduce_expression(between.x, context.clone()).await?;
                let _y =  self.reduce_expression(between.y, context).await?;
           
                //Ok(TableDataFieldType::Boolean(x <= a && a <= y ))

                unimplemented!("미구현")
            },
            SQLExpression::NotBetween(_between) => unimplemented!("미구현"),
            SQLExpression::Parentheses(paren) => {
                 self.reduce_expression(paren.expression, context).await
            }
            SQLExpression::FunctionCall(call) => {
                match call.function {
                    Function::BuiltIn(builtin)=>{
                        match builtin {
                            BuiltInFunction::Aggregate(aggregate)=>{
                                match aggregate {
                                    AggregateFunction::Count => {
                                        if call.arguments.len() != 1 {
                                            return Err(ExecuteError::dyn_boxed(
                                                "Count function takes only one parameter.",
                                            ));
                                        }

                                        let argument = call.arguments[0].clone();
                                        let value = self.reduce_expression(argument, context.clone()).await?;

                                        match value {
                                            TableDataFieldType::Array(array) => {
                                                
                                                let value = array.into_iter().filter(|e|{
                                                    match e {
                                                        TableDataFieldType::Null => {
                                                            false
                                                        }, 
                                                        _ => true,
                                                    }
                                                }).count();

                                                return Ok(TableDataFieldType::Integer(value as i64));
                                            }
                                            TableDataFieldType::Null => return Ok(TableDataFieldType::Integer(0)),
                                            _ => return Ok(TableDataFieldType::Integer(context.total_count as i64))
                                        }
                                    }
                                    AggregateFunction::Sum => {
                                        if call.arguments.len() != 1 {
                                            return Err(ExecuteError::dyn_boxed(
                                                "Sum function takes only one parameter.",
                                            ));
                                        }

                                        let argument = call.arguments[0].clone();
                                        let value = self.reduce_expression(argument, context.clone()).await?;

                                        match value {
                                            TableDataFieldType::Array(array) => {
                                                let value = array.into_iter().fold(TableDataFieldType::Null, |acc, e|{
                                                    match e {
                                                        TableDataFieldType::Integer(integer) => {
                                                            if let TableDataFieldType::Integer(acc_value) = acc  {
                                                                TableDataFieldType::Integer(acc_value + integer)
                                                            } else {
                                                                TableDataFieldType::Integer(integer)
                                                            }
                                                        }, 
                                                        TableDataFieldType::Float(integer) => {
                                                            if let TableDataFieldType::Float(acc_value) = acc  {
                                                                TableDataFieldType::Float(acc_value + integer)
                                                            } else {
                                                                TableDataFieldType::Float(integer)
                                                            }
                                                        }, 
                                                        _ => acc,
                                                    }
                                                });

                                                return Ok(value);
                                            }
                                            _ => {
                                                unimplemented!("미구현");
                                            }
                                        } 
                                    }
                                    AggregateFunction::Max => {
                                        if call.arguments.len() != 1 {
                                            return Err(ExecuteError::dyn_boxed(
                                                "Max function takes only one parameter.",
                                            ));
                                        }

                                        unimplemented!("미구현");
                                    }
                                    AggregateFunction::Min => {
                                        if call.arguments.len() != 1 {
                                            return Err(ExecuteError::dyn_boxed(
                                                "Min function takes only one parameter.",
                                            ));
                                        }
                                        
                                        unimplemented!("미구현");
                                    }
                                    _ => unimplemented!("미구현")
                                }
                                unimplemented!("미구현")
                            }
                            BuiltInFunction::Conditional(_)=>{
                                unimplemented!("미구현")
                            }
                        }
                    }
                    Function::UserDefined(_)=>unimplemented!("미구현"),
                }
            },
            SQLExpression::Subquery(_) => unimplemented!("미구현"),
            SQLExpression::SelectColumn(select_column) => {
                let column_name  = select_column.column_name.clone();

                match context.row {
                    Some(ref row) => {
                        let same_name_datas = row.fields.iter().filter(|e|e.column_name == column_name).cloned().collect::<Vec<_>>();

                        // 없으면 오류
                        if same_name_datas.is_empty() {
                            return Err(ExecuteError::dyn_boxed(
                                format!("1 column select '{:?}' not exists", select_column),
                            ));
                        }

                        // 테이블명 선택한게 있으면 
                        match select_column.table_name {
                            Some(ref table_name)=> {
                                
                                if let Some(found) = same_name_datas.iter().find(|e|{
                            
                                    // alias가 있으면
                                    if let Some(table_name) = context.table_alias_map.get(table_name) {
                                        *table_name == e.table_name
                                    }
                                    // 없으면 자체 테이블명 비교
                                    else {
                                        table_name == &e.table_name.table_name
                                    }
                                }) 
                                {
                                    Ok(found.data.to_owned())
                                } else{
                                    Err(ExecuteError::dyn_boxed(
                                        format!("column select '{:?}' is ambiguous", select_column),
                                    ))
                                }
                            }
                            None=>{
                                if same_name_datas.len()>=2 {
                                     Err(ExecuteError::dyn_boxed(
                                        format!("column select '{:?}' is ambiguous", select_column),
                                    ))
                                } else {
                                    Ok(same_name_datas[0].data.to_owned())
                                }
                            }
                        }
                    }
                    None => {
                        return Err(ExecuteError::dyn_boxed(
                            format!("column select '{:?}' not exists", select_column),
                        ));
                    }
                }
                
            },
        }
    }

    pub fn reduce_type(
        &self,
        expression: SQLExpression,
        context: ReduceContext
    ) -> Result<ExecuteColumnType, Box<dyn Error + Send>> {
        match expression {
            SQLExpression::Integer(_) => Ok(ExecuteColumnType::Integer),
            SQLExpression::Boolean(_) => Ok(ExecuteColumnType::Bool),
            SQLExpression::Float(_) => Ok(ExecuteColumnType::Float),
            SQLExpression::String(_) => Ok(ExecuteColumnType::String),
            SQLExpression::Null => Ok(ExecuteColumnType::Null),
            SQLExpression::List(_list) =>  {
                unimplemented!()
            }
            SQLExpression::Unary(unary) => match unary.operator {
                UnaryOperator::Neg | UnaryOperator::Pos | UnaryOperator::Not => {
                    self.reduce_type(unary.operand, context)
                }
            },
            SQLExpression::Binary(binary) => {
                let lhs = self.reduce_type(binary.lhs, context.clone())?;
                let rhs = self.reduce_type(binary.rhs, context)?;

                match binary.operator {
                    BinaryOperator::Add | BinaryOperator::Sub | BinaryOperator::Mul | BinaryOperator::Div => {
                        if let ExecuteColumnType::Null = lhs {
                            return Ok(ExecuteColumnType::Null);
                        }
        
                        if let ExecuteColumnType::Null = rhs {
                            return Ok(ExecuteColumnType::Null);
                        }

                        Ok(lhs)
                    },
                    BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Lt | BinaryOperator::Gt | BinaryOperator::Lte | BinaryOperator::Gte |  BinaryOperator::Eq | BinaryOperator::Neq | BinaryOperator::Like | BinaryOperator::NotLike | BinaryOperator::In | BinaryOperator::NotIn => {
                        if let ExecuteColumnType::Null = lhs {
                            return Ok(ExecuteColumnType::Null);
                        }
        
                        if let ExecuteColumnType::Null = rhs {
                            return Ok(ExecuteColumnType::Null);
                        }

                        Ok(ExecuteColumnType::Bool)
                    },   
                    BinaryOperator::Is | BinaryOperator::IsNot => {
                        Ok(ExecuteColumnType::Bool)
                    }
                }
            }
            SQLExpression::Between(_) => {
                Ok(ExecuteColumnType::Bool)
            },
            SQLExpression::NotBetween(_between) => Ok(ExecuteColumnType::Bool),
            SQLExpression::Parentheses(paren) => {
                 self.reduce_type(paren.expression, context)
            }
            SQLExpression::FunctionCall(call) => match call.function {
                Function::BuiltIn(builtin) => {
                    match builtin  {
                        BuiltInFunction::Aggregate(aggregate)=> {
                            match aggregate {
                                AggregateFunction::Sum => {
                                    Ok(ExecuteColumnType::Integer)
                                }
                                AggregateFunction::Count => {
                                    Ok(ExecuteColumnType::Integer)
                                }
                                AggregateFunction::Max => {
                                    Ok(ExecuteColumnType::Integer)
                                }
                                AggregateFunction::Min => {
                                    Ok(ExecuteColumnType::Integer)
                                }
                                _ => unimplemented!("미구현"),
                            }
                        }
                        BuiltInFunction::Conditional(_)=>unimplemented!("미구현"),
                    }
                }
                Function::UserDefined(_)=>{
                    unimplemented!("미구현")
                }
            },
            SQLExpression::Subquery(_) => unimplemented!("미구현"),
            SQLExpression::SelectColumn(select_column) => {
                let column_name  = select_column.column_name.clone();
                
                if context.config_columns.is_empty() {
                    return Err(ExecuteError::dyn_boxed(
                        format!("column select '{:?}' not exists", select_column),
                    ));
                }

                let same_name_columns = context.config_columns.iter().filter(|(_, e)|e.name == column_name).cloned().collect::<Vec<_>>();

                // 테이블명 선택한게 있으면 
                match select_column.table_name {
                    Some(ref table_name)=> {
                        
                        if let Some(found) = context.config_columns.iter().find(|(each_table_name, _)|{
                    
                            // alias가 있으면
                            if let Some(table_name) = context.table_alias_map.get(table_name) {
                                table_name == each_table_name
                            }
                            // 없으면 자체 테이블명 비교
                            else {
                                table_name == &each_table_name.table_name
                            }
                        }) 
                        {
                            Ok(found.1.data_type.to_owned().into())
                        } else{
                             Err(ExecuteError::dyn_boxed(
                                format!("column select '{:?}' is ambiguous", select_column),
                            ))
                        }
                    }
                    None=>{
                        if same_name_columns.len()>=2 {
                             Err(ExecuteError::dyn_boxed(
                                format!("column select '{:?}' is ambiguous", select_column),
                            ))
                        } else {
                            Ok(same_name_columns[0].1.data_type.to_owned().into())
                        }
                    }
                }
            },
        }
    }
}
