use async_trait::async_trait;
use tokio::sync::oneshot;

use crate::lib::ast::predule::SQLStatement;
use crate::lib::executor::predule::ExecuteResult;
use crate::lib::executor::result::ExecuteField;
use crate::lib::pgwire::engine::{Engine, Portal};
use crate::lib::pgwire::protocol::{DataRowBatch, ErrorResponse, FieldDescription, SqlState};
use crate::lib::server::predule::{ChannelRequest, ChannelResponse, SharedState};

#[derive(Debug, Clone)]
pub struct RRDBPortal {
    pub shared_state: SharedState,
    pub execute_result: ExecuteResult,
}

#[async_trait]
impl Portal for RRDBPortal {
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

                    // portal이 없을 경우 생성
                    if self.portal.is_none() {
                        self.portal = Some(RRDBPortal {
                            execute_result: result,
                            shared_state: self.shared_state.clone(),
                        });
                    }

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
