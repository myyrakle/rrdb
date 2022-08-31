use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::lib::ast::predule::SQLStatement;
use crate::lib::executor::predule::ExecuteResult;
use crate::lib::executor::result::ExecuteField;
use crate::lib::pgwire::engine::{Engine, Portal};
use crate::lib::pgwire::protocol::{DataRowBatch, ErrorResponse, FieldDescription};
use crate::lib::server::predule::{ChannelRequest, SharedState};

pub struct RRDBPortal {
    pub execute_result: Arc<Mutex<Option<ExecuteResult>>>,
}

#[async_trait]
impl Portal for RRDBPortal {
    async fn fetch(&mut self, batch: &mut DataRowBatch) -> Result<(), ErrorResponse> {
        while let Ok(locked) = self.execute_result.lock() {
            match &*locked {
                Some(data) => match data.rows.to_owned() {
                    Some(rows) => {
                        for row in rows {
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
                    None => continue,
                },
                None => continue,
            }
        }

        unreachable!()
    }
}

pub struct RRDBEngine {
    pub shared_state: SharedState,
    pub execute_result: Arc<Mutex<Option<ExecuteResult>>>,
}

#[async_trait]
impl Engine for RRDBEngine {
    type PortalType = RRDBPortal;

    async fn prepare(
        &mut self,
        statement: &SQLStatement,
    ) -> Result<Vec<FieldDescription>, ErrorResponse> {
        let _send_result = self
            .shared_state
            .sender
            .send(ChannelRequest {
                statement: statement.to_owned(),
                execute_result: Arc::clone(&self.execute_result),
            })
            .await;

        while let Ok(locked) = self.execute_result.lock() {
            match &*locked {
                Some(data) => match data.columns.to_owned() {
                    Some(columns) => {
                        return Ok(columns
                            .iter()
                            .map(|e| FieldDescription {
                                name: e.name.to_owned(),
                                data_type: e.data_type.to_owned().into(),
                            })
                            .collect());
                    }
                    None => continue,
                },
                None => continue,
            }
        }

        unreachable!()
    }

    async fn create_portal(&mut self, _: &SQLStatement) -> Result<Self::PortalType, ErrorResponse> {
        Ok(RRDBPortal {
            execute_result: Arc::clone(&self.execute_result),
        })
    }
}
