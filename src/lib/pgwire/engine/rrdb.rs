use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tokio::sync::{mpsc, oneshot};

use crate::lib::ast::predule::SQLStatement;
use crate::lib::executor::predule::ExecuteResult;
use crate::lib::executor::result::ExecuteField;
use crate::lib::pgwire::engine::{Engine, Portal};
use crate::lib::pgwire::protocol::{
    DataRowBatch, DataTypeOid, ErrorResponse, FieldDescription, SqlState,
};
use crate::lib::server::predule::{ChannelRequest, ChannelResponse, SharedState};

pub struct RRDBPortal {
    pub shared_state: SharedState,
    pub portal_receiver: mpsc::Receiver<ChannelResponse>,
}

#[async_trait]
impl Portal for RRDBPortal {
    async fn fetch(&mut self, batch: &mut DataRowBatch) -> Result<(), ErrorResponse> {
        match self.portal_receiver.recv().await {
            Some(response) => match response.result {
                Ok(result) => {
                    for row in result.rows.to_owned() {
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
                Err(error) => {
                    return Err(ErrorResponse::fatal(
                        SqlState::CONNECTION_EXCEPTION,
                        error.to_string(),
                    ))
                }
            },
            None => {
                return Err(ErrorResponse::fatal(
                    SqlState::CONNECTION_EXCEPTION,
                    "error".to_string(),
                ))
            }
        }
    }
}

pub struct RRDBEngine {
    pub shared_state: SharedState,
    pub portal_sender: Option<mpsc::Sender<ChannelResponse>>,
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
                Ok(ref result) => {
                    let return_value = Ok(result
                        .columns
                        .iter()
                        .map(|e| FieldDescription {
                            name: e.name.to_owned(),
                            data_type: e.data_type.to_owned().into(),
                        })
                        .collect());

                    self.portal_sender = match &self.portal_sender {
                        Some(sender) => {
                            sender.send(response).await.unwrap();
                            None
                        }
                        None => None,
                    };

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
        let (sender, receiver) = mpsc::channel::<ChannelResponse>(1);

        self.portal_sender = Some(sender);

        Ok(RRDBPortal {
            shared_state: self.shared_state.clone(),
            portal_receiver: receiver,
        })
    }
}
