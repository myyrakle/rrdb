use crate::engine::ast::ddl::alter_table::{
    AlterColumnDropDefault, AlterColumnDropNotNull, AlterColumnSetDefault, AlterColumnSetNotNull,
    AlterColumnSetType, AlterTableAddColumn, AlterTableAlterColumn, AlterTableDropColumn,
    AlterTableQuery, AlterTableRenameColumn, AlterTableRenameTo,
};
use crate::engine::ast::ddl::create_table::CreateTableQuery;
use crate::engine::ast::ddl::drop_database::SQLStatement;
use crate::engine::ast::ddl::drop_table::DropTableQuery;
use crate::engine::lexer::predule::Token;
use crate::engine::parser::context::ParserContext;
use crate::engine::parser::predule::Parser;
use crate::errors;
use crate::errors::parsing_error::ParsingError;

impl Parser {
    // CREATE TABLE 쿼리 분석
    pub(crate) fn handle_create_table_query(
        &mut self,
        context: ParserContext,
    ) -> errors::Result<SQLStatement> {
        if !self.has_next_token() {
            return Err(ParsingError::wrap("need more tokens".to_string()));
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
            return Err(ParsingError::wrap("need more tokens".to_string()));
        }

        let current_token = self.get_next_token();

        if Token::LeftParentheses != current_token {
            return Err(ParsingError::wrap(format!(
                "expected '('. but your input word is '{:?}'",
                current_token
            )));
        }

        // 닫는 괄호 나올때까지 행/테이블 제약 파싱 반복 (#220)
        let mut saw_inline_primary_key = false;
        let mut saw_table_level_primary_key = false;

        loop {
            if !self.has_next_token() {
                return Err(ParsingError::wrap("need more tokens".to_string()));
            }

            let current_token = self.get_next_token();

            match current_token {
                Token::RightParentheses => {
                    break;
                }
                // 테이블 레벨 제약: PRIMARY KEY (column_name [, ...]) (#220)
                Token::Primary => {
                    if !self.has_next_token() {
                        return Err(ParsingError::wrap("need more tokens".to_string()));
                    }

                    let current_token = self.get_next_token();

                    if Token::Key != current_token {
                        return Err(ParsingError::wrap(format!(
                            "expected 'PRIMARY KEY'. but your input word is '{:?}'",
                            current_token
                        )));
                    }

                    if !self.has_next_token() {
                        return Err(ParsingError::wrap("need more tokens".to_string()));
                    }

                    let current_token = self.get_next_token();

                    if Token::LeftParentheses != current_token {
                        return Err(ParsingError::wrap(format!(
                            "expected '('. but your input word is '{:?}'",
                            current_token
                        )));
                    }

                    let mut columns: Vec<String> = vec![];
                    // 식별자와 쉼표가 교대로 와야 합니다 (#220):
                    // PRIMARY KEY (a, b) O, (,a) / (a,) / (a,,b) X
                    let mut expect_identifier = true;

                    loop {
                        if !self.has_next_token() {
                            return Err(ParsingError::wrap("need more tokens".to_string()));
                        }

                        let current_token = self.get_next_token();

                        match current_token {
                            Token::RightParentheses => {
                                if expect_identifier && !columns.is_empty() {
                                    return Err(ParsingError::wrap(
                                        "trailing comma in 'PRIMARY KEY (...)'".to_string(),
                                    ));
                                }
                                break;
                            }
                            Token::Comma => {
                                if expect_identifier {
                                    return Err(ParsingError::wrap(
                                        "expected column name in 'PRIMARY KEY (...)'. but your input word is 'Comma'"
                                            .to_string(),
                                    ));
                                }
                                expect_identifier = true;
                            }
                            Token::Identifier(column_name) => {
                                if !expect_identifier {
                                    return Err(ParsingError::wrap(format!(
                                        "expected ',' or ')' after column name in 'PRIMARY KEY (...)'. but your input word is '{:?}'",
                                        column_name
                                    )));
                                }
                                columns.push(column_name);
                                expect_identifier = false;
                            }
                            _ => {
                                return Err(ParsingError::wrap(format!(
                                    "expected column name in 'PRIMARY KEY (...)'. but your input word is '{:?}'",
                                    current_token
                                )));
                            }
                        }
                    }

                    if columns.is_empty() {
                        return Err(ParsingError::wrap(
                            "expected at least one column name in 'PRIMARY KEY (...)'".to_string(),
                        ));
                    }

                    if saw_table_level_primary_key {
                        return Err(ParsingError::wrap(
                            "multiple table-level primary keys specified".to_string(),
                        ));
                    }

                    saw_table_level_primary_key = true;
                    query_builder = query_builder.set_primary_key(columns);
                }
                _ => {
                    self.unget_next_token(current_token);
                    let column = self.parse_table_column()?;

                    if column.primary_key {
                        saw_inline_primary_key = true;
                    }

                    query_builder = query_builder.add_column(column);
                }
            }
        }

        // 인라인 PRIMARY KEY와 테이블 레벨 PRIMARY KEY를 동시에 지정할 수 없습니다 (#220)
        if saw_inline_primary_key && saw_table_level_primary_key {
            return Err(ParsingError::wrap(
                "multiple primary keys specified: cannot combine an inline 'PRIMARY KEY' column constraint with a table-level 'PRIMARY KEY (...)'" 
                    .to_string(),
            ));
        }

        // 테이블 레벨 PK는 정의된 컬럼만 참조할 수 있습니다 (#220)
        if saw_table_level_primary_key {
            let query = query_builder.build();
            if let crate::engine::ast::SQLStatement::DDL(
                crate::engine::ast::DDLStatement::CreateTableQuery(inner),
            ) = &query
            {
                let column_names: std::collections::HashSet<&str> = inner
                    .columns
                    .iter()
                    .map(|column| column.name.as_str())
                    .collect();

                for column_name in &inner.primary_key {
                    if !column_names.contains(column_name.as_str()) {
                        return Err(ParsingError::wrap(format!(
                            "primary key column '{}' is not defined in the table",
                            column_name
                        )));
                    }
                }
            }

            // 문장 종료 검증은 일반 경로와 동일하게 적용합니다 (#220):
            // 테이블 레벨 PK 뒤에 이상한 토큰이 남으면 에러.
            if !self.has_next_token() {
                return Ok(query);
            }

            let current_token = self.get_next_token();

            if Token::SemiColon != current_token {
                return Err(ParsingError::wrap(format!(
                    "expected ';'. but your input word is '{:?}'",
                    current_token
                )));
            }

            return Ok(query);
        }

        if !self.has_next_token() {
            return Ok(query_builder.build());
        }

        let current_token = self.get_next_token();

        if Token::SemiColon != current_token {
            return Err(ParsingError::wrap(format!(
                "expected ';'. but your input word is '{:?}'",
                current_token
            )));
        }

        Ok(query_builder.build())
    }

    // ALTER TABLE 쿼리 분석
    pub(crate) fn handle_alter_table_query(
        &mut self,
        context: ParserContext,
    ) -> errors::Result<SQLStatement> {
        if !self.has_next_token() {
            return Err(ParsingError::wrap("need more tokens".to_string()));
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
                    return Err(ParsingError::wrap("need more tokens".to_string()));
                }

                let current_token = self.get_next_token();

                match current_token {
                    // table name rename
                    Token::To => {
                        if !self.has_next_token() {
                            return Err(ParsingError::wrap("need more tokens".to_string()));
                        }

                        let current_token = self.get_next_token();

                        match current_token {
                            Token::Identifier(identifier) => {
                                query_builder = query_builder
                                    .set_action(AlterTableRenameTo { name: identifier }.into());
                            }
                            _ => {
                                return Err(ParsingError::wrap(format!(
                                    "unexpected token {:?}",
                                    current_token
                                )));
                            }
                        }
                    }
                    // table column name rename
                    Token::Column => {
                        if !self.has_next_token() {
                            return Err(ParsingError::wrap("need more tokens".to_string()));
                        }

                        let current_token = self.get_next_token();

                        if let Token::Identifier(from_name) = current_token {
                            if !self.has_next_token() {
                                return Err(ParsingError::wrap("need more tokens".to_string()));
                            }

                            let current_token = self.get_next_token();

                            if Token::To != current_token {
                                return Err(ParsingError::wrap(format!(
                                    "expected token is 'TO', but you input is {:?}",
                                    current_token
                                )));
                            }

                            if !self.has_next_token() {
                                return Err(ParsingError::wrap("need more tokens".to_string()));
                            }

                            let current_token = self.get_next_token();

                            if let Token::Identifier(to_name) = current_token {
                                query_builder = query_builder.set_action(
                                    AlterTableRenameColumn { from_name, to_name }.into(),
                                );
                            } else {
                                return Err(ParsingError::wrap(format!(
                                    "expected token is 'identifer', but you input is {:?}",
                                    current_token
                                )));
                            }
                        } else {
                            return Err(ParsingError::wrap(format!(
                                "expected token {:?}",
                                current_token
                            )));
                        }
                    }
                    // table column name rename
                    Token::Identifier(from_name) => {
                        if !self.has_next_token() {
                            return Err(ParsingError::wrap("need more tokens".to_string()));
                        }

                        let current_token = self.get_next_token();

                        if Token::To != current_token {
                            return Err(ParsingError::wrap(format!(
                                "expected token is 'TO', but you input is {:?}",
                                current_token
                            )));
                        }

                        if !self.has_next_token() {
                            return Err(ParsingError::wrap("need more tokens".to_string()));
                        }

                        let current_token = self.get_next_token();

                        if let Token::Identifier(to_name) = current_token {
                            query_builder = query_builder
                                .set_action(AlterTableRenameColumn { from_name, to_name }.into());
                        } else {
                            return Err(ParsingError::wrap(format!(
                                "expected token is 'identifer', but you input is {:?}",
                                current_token
                            )));
                        }
                    }
                    _ => {
                        return Err(ParsingError::wrap(format!(
                            "expected token is 'TO' or 'COLUMN', but you input is {:?}",
                            current_token
                        )));
                    }
                }
            }
            Token::Add => {
                if !self.has_next_token() {
                    return Err(ParsingError::wrap("need more tokens".to_string()));
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
                        return Err(ParsingError::wrap(format!(
                            "unexpected keyword '{:?}'",
                            current_token
                        )));
                    }
                }
            }
            Token::Drop => {
                if self.next_token_is_column() {
                    self.get_next_token();
                }

                if !self.has_next_token() {
                    return Err(ParsingError::wrap("need more tokens".to_string()));
                }

                let current_token = self.get_next_token();

                if let Token::Identifier(column_name) = current_token {
                    query_builder =
                        query_builder.set_action(AlterTableDropColumn { column_name }.into());
                } else {
                    return Err(ParsingError::wrap(format!(
                        "unexpected token {:?}",
                        current_token
                    )));
                }
            }
            Token::Alter => {
                if self.next_token_is_column() {
                    self.get_next_token();
                }

                if !self.has_next_token() {
                    return Err(ParsingError::wrap("need more tokens".to_string()));
                }

                let current_token = self.get_next_token();

                if let Token::Identifier(column_name) = current_token {
                    if !self.has_next_token() {
                        return Err(ParsingError::wrap("need more tokens".to_string()));
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
                                    return Err(ParsingError::wrap("need more tokens".to_string()));
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
                                    return Err(ParsingError::wrap("need more tokens".to_string()));
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
                                return Err(ParsingError::wrap("unexpected tokens".to_string()));
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
                                return Err(ParsingError::wrap("unexpected tokens".to_string()));
                            }
                        }
                        Token::Type => {
                            if !self.has_next_token() {
                                return Err(ParsingError::wrap("need more tokens".to_string()));
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
                            return Err(ParsingError::wrap(format!(
                                "unexpected token {:?}",
                                current_token
                            )));
                        }
                    }
                } else {
                    return Err(ParsingError::wrap(format!(
                        "unexpected token {:?}",
                        current_token
                    )));
                }
            }
            _ => {
                return Err(ParsingError::wrap(format!(
                    "unexpected keyword '{:?}'",
                    current_token
                )));
            }
        }

        Ok(query_builder.build())
    }

    // DROP TABLE 쿼리 분석
    pub(crate) fn handle_drop_table_query(
        &mut self,
        context: ParserContext,
    ) -> errors::Result<SQLStatement> {
        let mut query_builder = DropTableQuery::builder();

        // IF EXISTS 파싱
        let if_exists = self.has_if_exists()?;
        query_builder = query_builder.set_if_exists(if_exists);

        // 테이블명 획득 로직
        if !self.has_next_token() {
            return Err(ParsingError::wrap("need more tokens".to_string()));
        }

        let table = self.parse_table_name(context)?;

        // 테이블명 설정
        query_builder = query_builder.set_table(table);

        if !self.has_next_token() {
            return Ok(query_builder.build());
        }

        let current_token = self.get_next_token();

        if Token::SemiColon != current_token {
            return Err(ParsingError::wrap(format!(
                "expected ';'. but your input word is '{:?}'",
                current_token
            )));
        }

        Ok(query_builder.build())
    }
}
