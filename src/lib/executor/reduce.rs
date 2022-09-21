

use std::error::Error;

use futures::future::join_all;

use crate::lib::ast::dml::{BinaryOperator, UnaryOperator};
use crate::lib::ast::predule::SQLExpression;
use crate::lib::errors::predule::{TypeError};
use super::config::TableDataFieldType;
use super::predule::Executor;

pub struct ReduceContext {

}

impl Executor {
    #[async_recursion::async_recursion]
    pub async fn reduce_expression(
        &self,
        expression: SQLExpression,
    ) -> Result<TableDataFieldType, Box<dyn Error>> {
        match expression {
            SQLExpression::Integer(value) => Ok(TableDataFieldType::Integer(value)),
            SQLExpression::Boolean(value) => Ok(TableDataFieldType::Boolean(value)),
            SQLExpression::Float(value) => Ok(TableDataFieldType::Float(value)),
            SQLExpression::String(value) => Ok(TableDataFieldType::String(value)),
            SQLExpression::Null => Ok(TableDataFieldType::Null),
            SQLExpression::List(list) =>  {
                let futures = list.value.into_iter().map(|e|{self.reduce_expression(e)});
                let fields = join_all(futures).await.into_iter().collect::<Result<Vec<_>, 
                _>>()?;

                let serialized: String = fields.into_iter().map(|e|e.to_string()).intersperse(", ".to_owned()).collect();

                Ok(TableDataFieldType::String(format!("({})", serialized)))
        }
            SQLExpression::Unary(unary) => match unary.operator {
                UnaryOperator::Neg => {
                    let operand = self.reduce_expression(unary.operand).await?;

                    match operand {
                        TableDataFieldType::Integer(value) => {
                            Ok(TableDataFieldType::Integer(-value))
                        }
                        TableDataFieldType::Float(value) => {
                            Ok(TableDataFieldType::Float(-value))
                        }
                        _ => Err(TypeError::dyn_boxed(
                            "unary '-' operator is valid only for integer and float types.",
                        )),
                    }
                }
                UnaryOperator::Pos => {
                    let operand = self.reduce_expression(unary.operand).await?;

                    match operand {
                        TableDataFieldType::Integer(_) => Ok(operand),
                        TableDataFieldType::Float(_) => Ok(operand),
                        _ => Err(TypeError::dyn_boxed(
                            "unary '+' operator is valid only for integer and float types.",
                        )),
                    }
                }
                UnaryOperator::Not => {
                    let operand = self.reduce_expression(unary.operand).await?;

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
                let lhs = self.reduce_expression(binary.lhs).await?;
                let rhs = self.reduce_expression(binary.rhs).await?;

                if lhs.type_code() != rhs.type_code() {
                    return Err(TypeError::dyn_boxed(
                        "The types of lhs and rhs do not match.",
                    ));
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
                                return Ok(TableDataFieldType::Float(lhs_value - rhs_value));
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
            SQLExpression::Between(_between) => unimplemented!("미구현"),
            SQLExpression::NotBetween(_between) => unimplemented!("미구현"),
            SQLExpression::Parentheses(paren) => {
                 self.reduce_expression(paren.expression).await
            }
            SQLExpression::FunctionCall(_function_call) => unimplemented!("미구현"),
            SQLExpression::Subquery(_) => unimplemented!("미구현"),
            SQLExpression::SelectColumn(_) => unimplemented!("미구현"),
        }
    }
}
