use async_trait::async_trait;

use crate::engine::ast::{DMLStatement, OtherStatement, SQLStatement};
use crate::engine::server::shared_state::SharedState;
use crate::engine::types::{ExecuteColumn, ExecuteField, ExecuteResult};
use crate::pgwire::engine::{Engine, Portal};
use crate::pgwire::protocol::backend::{ErrorResponse, FieldDescription};
use crate::pgwire::protocol::{DataRowBatch, SqlState};

#[derive(Clone)]
pub struct RRDBPortal {
    pub shared_state: SharedState,
    pub statement: SQLStatement,
    pub execute_result: Option<ExecuteResult>,
}

fn columns_to_fields(columns: &[ExecuteColumn]) -> Vec<FieldDescription> {
    columns
        .iter()
        .map(|column| FieldDescription {
            name: column.name.to_owned(),
            data_type: column.data_type.to_owned().into(),
        })
        .collect()
}

impl RRDBPortal {
    pub async fn execute(&mut self) -> Result<ExecuteResult, ErrorResponse> {
        if let Some(result) = &self.execute_result {
            return Ok(result.clone());
        }

        let result = RRDBEngine {
            shared_state: self.shared_state.clone(),
        }
        .execute_statement(&self.statement)
        .await?;

        self.execute_result = Some(result.clone());
        Ok(result)
    }
}

#[async_trait]
impl Portal for RRDBPortal {
    // 실제 결과 데이터 리스트 전송
    async fn fetch(&mut self, batch: &mut DataRowBatch) -> Result<(), ErrorResponse> {
        let result = self.execute().await?;

        for row in result.rows {
            let mut writer = batch.create_row();

            for field in row.fields {
                match field {
                    ExecuteField::Bool(data) => {
                        writer.write_bool(data);
                    }
                    ExecuteField::Integer(data) => {
                        writer.write_int8(data);
                    }
                    ExecuteField::Float(data) => {
                        writer.write_float8(data);
                    }
                    ExecuteField::String(data) => {
                        writer.write_string(&data);
                    }
                    ExecuteField::Null => {
                        writer.write_null();
                    }
                }
            }
        }

        return Ok(());
    }
}

pub struct RRDBEngine {
    pub shared_state: SharedState,
}

impl RRDBEngine {
    fn statement_returns_rows(statement: &SQLStatement) -> bool {
        matches!(
            statement,
            SQLStatement::DML(DMLStatement::SelectQuery(_))
                | SQLStatement::Other(OtherStatement::ShowDatabases(_))
                | SQLStatement::Other(OtherStatement::ShowTables(_))
                | SQLStatement::Other(OtherStatement::DescTable(_))
                | SQLStatement::Other(OtherStatement::UseDatabase(_))
        )
    }

    pub async fn execute_statement(
        &mut self,
        statement: &SQLStatement,
    ) -> Result<ExecuteResult, ErrorResponse> {
        self.shared_state
            .engine
            .process_query(
                statement.to_owned(),
                self.shared_state.wal_manager.clone(),
                self.shared_state.client_info.connection_id.clone(),
            )
            .await
            .map_err(|error| ErrorResponse::error(SqlState::SYNTAX_ERROR, error.to_string()))
    }
}

#[async_trait]
impl Engine for RRDBEngine {
    type PortalType = RRDBPortal;

    // 결과 데이터 리스트의 컬럼 정보 전송
    async fn prepare(
        &mut self,
        statement: &SQLStatement,
    ) -> Result<Vec<FieldDescription>, ErrorResponse> {
        match statement {
            SQLStatement::DML(DMLStatement::SelectQuery(query)) => {
                let columns = self
                    .shared_state
                    .engine
                    .describe_select_columns(query.clone())
                    .await
                    .map_err(|error| {
                        ErrorResponse::error(SqlState::SYNTAX_ERROR, error.to_string())
                    })?;
                return Ok(columns_to_fields(&columns));
            }
            statement if !Self::statement_returns_rows(statement) => {
                return Ok(Vec::new());
            }
            _ => {}
        }

        let result = self.execute_statement(statement).await?;
        Ok(columns_to_fields(&result.columns))
    }

    async fn create_portal(
        &mut self,
        statement: &SQLStatement,
    ) -> Result<Self::PortalType, ErrorResponse> {
        Ok(RRDBPortal {
            shared_state: self.shared_state.clone(),
            statement: statement.to_owned(),
            execute_result: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};
    use std::path::PathBuf;
    use std::sync::Arc;

    use tokio::sync::Mutex;

    use crate::config::launch_config::LaunchConfig;
    use crate::engine::DBEngine;
    use crate::engine::parser::predule::{Parser, ParserContext};
    use crate::engine::server::client::ClientInfo;
    use crate::engine::server::shared_state::SharedState;
    use crate::engine::types::ExecuteField;
    use crate::engine::wal::endec::implements::bincode::{BincodeDecoder, BincodeEncoder};
    use crate::engine::wal::manager::builder::WALBuilder;
    use crate::pgwire::engine::{Engine, RRDBEngine};

    async fn build_test_engine(test_name: &str) -> RRDBEngine {
        let base_path = PathBuf::from("target").join(test_name);
        if base_path.exists() {
            tokio::fs::remove_dir_all(&base_path).await.unwrap();
        }

        let config = LaunchConfig::default_for_base_path(&base_path);
        tokio::fs::create_dir_all(&config.data_directory)
            .await
            .unwrap();
        tokio::fs::create_dir_all(&config.wal_directory)
            .await
            .unwrap();

        let wal = WALBuilder::new(&config)
            .build(BincodeDecoder::new(), BincodeEncoder::new())
            .await
            .unwrap();
        let shared_state = SharedState {
            engine: Arc::new(DBEngine::new(config)),
            wal_manager: Arc::new(Mutex::new(wal)),
            client_info: ClientInfo {
                ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
                connection_id: "test-connection".to_string(),
                database: "rrdb".to_string(),
            },
        };

        RRDBEngine { shared_state }
    }

    fn parse_statement(sql: &str) -> crate::engine::ast::SQLStatement {
        let sql = if sql.trim_end().ends_with(';') {
            sql.to_string()
        } else {
            format!("{};", sql)
        };
        let mut parser = Parser::with_string(sql).unwrap();
        parser
            .parse(ParserContext::default().set_default_database("rrdb".to_string()))
            .unwrap()
            .remove(0)
    }

    async fn execute_sql(engine: &mut RRDBEngine, sql: &str) {
        let statement = parse_statement(sql);
        engine.execute_statement(&statement).await.unwrap();
    }

    #[tokio::test]
    async fn prepare_select_returns_row_description_fields() {
        let mut engine = build_test_engine("test_rrdb_engine/prepare_select_fields").await;
        execute_sql(&mut engine, "create database rrdb").await;
        execute_sql(
            &mut engine,
            "create table key_value (name varchar(255), value int)",
        )
        .await;

        let statement = parse_statement("select name, value from key_value");
        let fields = engine.prepare(&statement).await.unwrap();

        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].name, "name");
        assert_eq!(fields[1].name, "value");
    }

    #[tokio::test]
    async fn create_portal_does_not_execute_until_execute_is_called() {
        let mut engine = build_test_engine("test_rrdb_engine/portal_execute_lazily").await;
        execute_sql(&mut engine, "create database rrdb").await;
        execute_sql(
            &mut engine,
            "create table key_value (key varchar(255), value int)",
        )
        .await;

        let insert =
            parse_statement("insert into key_value (key, value) values ('a', 1), ('b', 2)");
        let mut portal = engine.create_portal(&insert).await.unwrap();

        let count = engine
            .execute_statement(&parse_statement("select count(1) from key_value"))
            .await
            .unwrap();
        assert_eq!(count.rows[0].fields[0], ExecuteField::Integer(0));

        let result = portal.execute().await.unwrap();
        assert_eq!(result.affected_rows, Some(2));

        let count = engine
            .execute_statement(&parse_statement("select count(1) from key_value"))
            .await
            .unwrap();
        assert_eq!(count.rows[0].fields[0], ExecuteField::Integer(2));
    }
}
