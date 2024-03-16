use async_trait::async_trait;
use tokio::sync::oneshot;

use crate::ast::SQLStatement;
use crate::executor::predule::ExecuteResult;
use crate::executor::result::ExecuteField;
use crate::pgwire::engine::{Engine, Portal};
use crate::pgwire::protocol::backend::{ErrorResponse, FieldDescription};
use crate::pgwire::protocol::{DataRowBatch, SqlState};
use crate::server::predule::{ChannelRequest, ChannelResponse, SharedState};

#[derive(Debug, Clone)]
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
                        writer.write_int8(data as i64);
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
    pub portal: Option<RRDBPortal>,
}

#[async_trait]
impl Engine for RRDBEngine {
    type PortalType = RRDBPortal;

    // 결과 데이터 리스트의 컬럼 정보 전송
    async fn prepare(
        &mut self,
        statement: &SQLStatement,
    ) -> Result<Vec<FieldDescription>, ErrorResponse> {
        // Server Background Loop와의 통신용 채널
        let (response_sender, response_receiver) = oneshot::channel::<ChannelResponse>();

        if let Err(error) = self
            .shared_state
            .sender
            .send(ChannelRequest {
                statement: statement.to_owned(),
                response_sender,
                connection_id: self.shared_state.client_info.connection_id.clone(),
            })
            .await
        {
            return Err(ErrorResponse::fatal(
                SqlState::CONNECTION_EXCEPTION,
                error.to_string(),
            ));
        }

        match response_receiver.await {
            Ok(response) => match response.result {
                Ok(result) => {
                    let return_value = Ok(result
                        .columns
                        .iter()
                        .map(|e| FieldDescription {
                            name: e.name.to_owned(),
                            data_type: e.data_type.to_owned().into(),
                        })
                        .collect());

                    self.portal = Some(RRDBPortal {
                        execute_result: result,
                        shared_state: self.shared_state.clone(),
                    });

                    return return_value;
                }
                Err(error) => {
                    return Err(ErrorResponse::error(
                        SqlState::SYNTAX_ERROR,
                        error.to_string(),
                    ));
                }
            },
            Err(error) => {
                return Err(ErrorResponse::fatal(
                    SqlState::CONNECTION_EXCEPTION,
                    error.to_string(),
                ));
            }
        }
    }

    async fn create_portal(&mut self, _: &SQLStatement) -> Result<Self::PortalType, ErrorResponse> {
        match &self.portal {
            Some(portal) => Ok(portal.to_owned()),
            None => {
                return Err(ErrorResponse::fatal(
                    SqlState::CONNECTION_EXCEPTION,
                    "not prepared yet".to_string(),
                ));
            }
        }
    }
}
