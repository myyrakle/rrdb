use crate::ast::ddl::{
    AlterColumnDropDefault, AlterColumnDropNotNull, AlterColumnSetDefault, AlterColumnSetNotNull,
    AlterColumnSetType, AlterTableAddColumn, AlterTableAlterColumn, AlterTableDropColumn,
    AlterTableQuery, AlterTableRenameColumn, AlterTableRenameTo,
};
use crate::ast::predule::{CreateTableQuery, DropTableQuery, SQLStatement};
use crate::errors::predule::ParsingError;
use crate::lexer::predule::Token;
use crate::parser::context::ParserContext;
use crate::parser::predule::Parser;
use std::error::Error;

impl Parser {
    // CREATE TABLE 쿼리 분석
    pub(crate) fn handle_create_table_query(
        &mut self,
        context: ParserContext,
    ) -> Result<SQLStatement, Box<dyn Error + Send>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E1205 need more tokens"));
        }

        let mut query_builder = CreateTableQuery::builder();

        // IF NOT EXISTS 파싱
        let if_not_exists = self.has_if_not_exists()?;
        query_builder = query_builder.set_if_not_exists(if_not_exists);

        // 테이블명 설정
        let table = self.parse_table_name(context)?;
        query_builder = query_builder.set_table(table);

        // 여는 괄호 체크
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E1206 need more tokens"));
        }

        let current_token = self.get_next_token();

        if Token::LeftParentheses != current_token {
            return Err(ParsingError::boxed(format!(
                "E1207 expected '('. but your input word is '{:?}'",
                current_token
            )));
        }

        // 닫는 괄호 나올때까지 행 파싱 반복
        loop {
            if !self.has_next_token() {
                return Err(ParsingError::boxed("E1208 need more tokens"));
            }

            let current_token = self.get_next_token();

            match current_token {
                Token::RightParentheses => {
                    self.unget_next_token(current_token);
                    break;
                }
                _ => {
                    self.unget_next_token(current_token);
                    let column = self.parse_table_column()?;
                    query_builder = query_builder.add_column(column);
                }
            }
        }

        // 닫는 괄호 체크
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E1209 need more tokens"));
        }

        let current_token = self.get_next_token();

        if Token::RightParentheses != current_token {
            return Err(ParsingError::boxed(format!(
                "E1210 expected ')'. but your input word is '{:?}'",
                current_token
            )));
        }

        if !self.has_next_token() {
            return Ok(query_builder.build());
        }

        let current_token = self.get_next_token();

        if Token::SemiColon != current_token {
            return Err(ParsingError::boxed(format!(
                "E1211 expected ';'. but your input word is '{:?}'",
                current_token
            )));
        }

        Ok(query_builder.build())
    }

    // ALTER TABLE 쿼리 분석
    pub(crate) fn handle_alter_table_query(
        &mut self,
        context: ParserContext,
    ) -> Result<SQLStatement, Box<dyn Error + Send>> {
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E1201 need more tokens"));
        }

        let mut query_builder = AlterTableQuery::builder();

        let table_name = self.parse_table_name(context.clone())?;

        query_builder = query_builder.set_table(table_name);

        if !self.has_next_token() {
            return Ok(query_builder.build());
        }

        let current_token = self.get_next_token();

        match current_token {
            Token::SemiColon => return Ok(query_builder.build()),
            Token::Rename => {
                if !self.has_next_token() {
                    return Err(ParsingError::boxed("E1212 need more tokens"));
                }

                let current_token = self.get_next_token();

                match current_token {
                    // table name rename
                    Token::To => {
                        if !self.has_next_token() {
                            return Err(ParsingError::boxed("E1213 need more tokens"));
                        }

                        let current_token = self.get_next_token();

                        match current_token {
                            Token::Identifier(identifier) => {
                                query_builder = query_builder
                                    .set_action(AlterTableRenameTo { name: identifier }.into());
                            }
                            _ => {
                                return Err(ParsingError::boxed(format!(
                                    "E1214 unexpected token {:?}",
                                    current_token
                                )))
                            }
                        }
                    }
                    // table column name rename
                    Token::Column => {
                        if !self.has_next_token() {
                            return Err(ParsingError::boxed("E1217 need more tokens"));
                        }

                        let current_token = self.get_next_token();

                        if let Token::Identifier(from_name) = current_token {
                            if !self.has_next_token() {
                                return Err(ParsingError::boxed("E1219 need more tokens"));
                            }

                            let current_token = self.get_next_token();

                            if Token::To != current_token {
                                return Err(ParsingError::boxed(format!(
                                    "E1220 expected token is 'TO', but you input is {:?}",
                                    current_token
                                )));
                            }

                            if !self.has_next_token() {
                                return Err(ParsingError::boxed("E1221 need more tokens"));
                            }

                            let current_token = self.get_next_token();

                            if let Token::Identifier(to_name) = current_token {
                                query_builder = query_builder.set_action(
                                    AlterTableRenameColumn { from_name, to_name }.into(),
                                );
                            } else {
                                return Err(ParsingError::boxed(format!(
                                    "E1222 expected token is 'identifer', but you input is {:?}",
                                    current_token
                                )));
                            }
                        } else {
                            return Err(ParsingError::boxed(format!(
                                "E1218 expected token {:?}",
                                current_token
                            )));
                        }
                    }
                    // table column name rename
                    Token::Identifier(from_name) => {
                        if !self.has_next_token() {
                            return Err(ParsingError::boxed("E1218 need more tokens"));
                        }

                        let current_token = self.get_next_token();

                        if Token::To != current_token {
                            return Err(ParsingError::boxed(format!(
                                "E1223 expected token is 'TO', but you input is {:?}",
                                current_token
                            )));
                        }

                        if !self.has_next_token() {
                            return Err(ParsingError::boxed("E1224 need more tokens"));
                        }

                        let current_token = self.get_next_token();

                        if let Token::Identifier(to_name) = current_token {
                            query_builder = query_builder
                                .set_action(AlterTableRenameColumn { from_name, to_name }.into());
                        } else {
                            return Err(ParsingError::boxed(format!(
                                "E1225 expected token is 'identifer', but you input is {:?}",
                                current_token
                            )));
                        }
                    }
                    _ => {
                        return Err(ParsingError::boxed(format!(
                            "E1213 expected token is 'TO' or 'COLUMN', but you input is {:?}",
                            current_token
                        )))
                    }
                }
            }
            Token::Add => {
                if !self.has_next_token() {
                    return Err(ParsingError::boxed("E1215 need more tokens"));
                }

                let current_token = self.get_next_token();

                match current_token {
                    Token::Column => {
                        let column = self.parse_table_column()?;

                        query_builder =
                            query_builder.set_action(AlterTableAddColumn { column }.into());
                    }
                    Token::Identifier(_) => {
                        self.unget_next_token(current_token);

                        let column = self.parse_table_column()?;

                        query_builder =
                            query_builder.set_action(AlterTableAddColumn { column }.into());
                    }
                    _ => {
                        return Err(ParsingError::boxed(format!(
                            "E1216 unexpected keyword '{:?}'",
                            current_token
                        )))
                    }
                }
            }
            Token::Drop => {
                if self.next_token_is_column() {
                    self.get_next_token();
                }

                if !self.has_next_token() {
                    return Err(ParsingError::boxed("E1226 need more tokens"));
                }

                let current_token = self.get_next_token();

                if let Token::Identifier(column_name) = current_token {
                    query_builder =
                        query_builder.set_action(AlterTableDropColumn { column_name }.into());
                } else {
                    return Err(ParsingError::boxed(format!(
                        "E1227 unexpected token {:?}",
                        current_token
                    )));
                }
            }
            Token::Alter => {
                if self.next_token_is_column() {
                    self.get_next_token();
                }

                if !self.has_next_token() {
                    return Err(ParsingError::boxed("E1228 need more tokens"));
                }

                let current_token = self.get_next_token();

                if let Token::Identifier(column_name) = current_token {
                    if !self.has_next_token() {
                        return Err(ParsingError::boxed("E1230 need more tokens"));
                    }

                    let current_token = self.get_next_token();

                    match current_token {
                        Token::Set => {
                            if self.next_token_is_not_null() {
                                self.get_next_token();
                                self.get_next_token();

                                query_builder = query_builder.set_action(
                                    AlterTableAlterColumn {
                                        action: AlterColumnSetNotNull {}.into(),
                                        column_name,
                                    }
                                    .into(),
                                );
                            } else if self.next_token_is_data_type() {
                                self.get_next_token();
                                self.get_next_token();

                                if !self.has_next_token() {
                                    return Err(ParsingError::boxed("E1233 need more tokens"));
                                }

                                let data_type = self.parse_data_type()?;

                                query_builder = query_builder.set_action(
                                    AlterTableAlterColumn {
                                        action: AlterColumnSetType { data_type }.into(),
                                        column_name,
                                    }
                                    .into(),
                                );
                            } else if self.next_token_is_default() {
                                self.get_next_token();

                                if !self.has_next_token() {
                                    return Err(ParsingError::boxed("E1234 need more tokens"));
                                }

                                let expression = self.parse_expression(context)?;

                                query_builder = query_builder.set_action(
                                    AlterTableAlterColumn {
                                        action: AlterColumnSetDefault { expression }.into(),
                                        column_name,
                                    }
                                    .into(),
                                );
                            } else {
                                return Err(ParsingError::boxed("E1231 unexpected tokens"));
                            }
                        }
                        Token::Drop => {
                            if self.next_token_is_not_null() {
                                self.get_next_token();
                                self.get_next_token();

                                query_builder = query_builder.set_action(
                                    AlterTableAlterColumn {
                                        action: AlterColumnDropNotNull {}.into(),
                                        column_name,
                                    }
                                    .into(),
                                );
                            } else if self.next_token_is_default() {
                                self.get_next_token();

                                query_builder = query_builder.set_action(
                                    AlterTableAlterColumn {
                                        action: AlterColumnDropDefault {}.into(),
                                        column_name,
                                    }
                                    .into(),
                                );
                            } else {
                                return Err(ParsingError::boxed("E1231 unexpected tokens"));
                            }
                        }
                        Token::Type => {
                            if !self.has_next_token() {
                                return Err(ParsingError::boxed("E1232 need more tokens"));
                            }

                            let data_type = self.parse_data_type()?;

                            query_builder = query_builder.set_action(
                                AlterTableAlterColumn {
                                    action: AlterColumnSetType { data_type }.into(),
                                    column_name,
                                }
                                .into(),
                            );
                        }
                        _ => {
                            return Err(ParsingError::boxed(format!(
                                "E1229 unexpected token {:?}",
                                current_token
                            )))
                        }
                    }
                } else {
                    return Err(ParsingError::boxed(format!(
                        "E1229 unexpected token {:?}",
                        current_token
                    )));
                }
            }
            _ => {
                return Err(ParsingError::boxed(format!(
                    "E1202 unexpected keyword '{:?}'",
                    current_token
                )))
            }
        }

        Ok(query_builder.build())
    }

    // DROP TABLE 쿼리 분석
    pub(crate) fn handle_drop_table_query(
        &mut self,
        context: ParserContext,
    ) -> Result<SQLStatement, Box<dyn Error + Send>> {
        let mut query_builder = DropTableQuery::builder();

        // IF EXISTS 파싱
        let if_exists = self.has_if_exists()?;
        query_builder = query_builder.set_if_exists(if_exists);

        // 테이블명 획득 로직
        if !self.has_next_token() {
            return Err(ParsingError::boxed("E1203 need more tokens"));
        }

        let table = self.parse_table_name(context)?;

        // 테이블명 설정
        query_builder = query_builder.set_table(table);

        if !self.has_next_token() {
            return Ok(query_builder.build());
        }

        let current_token = self.get_next_token();

        if Token::SemiColon != current_token {
            return Err(ParsingError::boxed(format!(
                "E1204 expected ';'. but your input word is '{:?}'",
                current_token
            )));
        }

        Ok(query_builder.build())
    }
}
