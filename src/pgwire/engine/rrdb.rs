use async_trait::async_trait;

use crate::engine::ast::SQLStatement;
use crate::engine::server::shared_state::SharedState;
use crate::engine::types::{ExecuteField, ExecuteResult};
use crate::pgwire::engine::{Engine, Portal};
use crate::pgwire::protocol::backend::{ErrorResponse, FieldDescription};
use crate::pgwire::protocol::{DataRowBatch, SqlState};

#[derive(Clone)]
pub struct RRDBPortal {
    pub shared_state: SharedState,
    pub execute_result: ExecuteResult,
}

#[async_trait]
impl Portal for RRDBPortal {
    // 실제 결과 데이터 리스트 전송
    async fn fetch(&mut self, batch: &mut DataRowBatch) -> Result<(), ErrorResponse> {
        for row in self.execute_result.rows.iter().cloned() {
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

#[async_trait]
impl Engine for RRDBEngine {
    type PortalType = RRDBPortal;

    // 결과 데이터 리스트의 컬럼 정보 전송
    async fn prepare(
        &mut self,
        _statement: &SQLStatement,
    ) -> Result<Vec<FieldDescription>, ErrorResponse> {
        Ok(Vec::new())
    }

    async fn create_portal(
        &mut self,
        statement: &SQLStatement,
    ) -> Result<Self::PortalType, ErrorResponse> {
        let result = self
            .shared_state
            .engine
            .process_query(
                statement.to_owned(),
                self.shared_state.wal_manager.clone(),
                self.shared_state.client_info.connection_id.clone(),
            )
            .await
            .map_err(|error| ErrorResponse::error(SqlState::SYNTAX_ERROR, error.to_string()))?;

        Ok(RRDBPortal {
            execute_result: result,
            shared_state: self.shared_state.clone(),
        })
    }
}
