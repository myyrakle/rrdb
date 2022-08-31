use async_trait::async_trait;

use crate::lib::ast::predule::SQLStatement;
use crate::lib::pgwire::engine::{Engine, Portal};
use crate::lib::pgwire::protocol::{DataRowBatch, DataTypeOid, ErrorResponse, FieldDescription};
use crate::lib::server::channel::ChannelRequest;
use crate::lib::server::predule::SharedState;

pub struct RRDBPortal {}

#[async_trait]
impl Portal for RRDBPortal {
    async fn fetch(&mut self, batch: &mut DataRowBatch) -> Result<(), ErrorResponse> {
        {
            let mut row = batch.create_row();
            row.write_int4(1);
            row.write_string("foo");
        }

        {
            let mut row = batch.create_row();
            row.write_int4(2);
            row.write_string("bar");
            Ok(())
        }
    }
}

pub struct RRDBEngine {
    pub shared_state: SharedState,
}

#[async_trait]
impl Engine for RRDBEngine {
    type PortalType = RRDBPortal;

    async fn prepare(
        &mut self,
        statement: &SQLStatement,
    ) -> Result<Vec<FieldDescription>, ErrorResponse> {
        // TODO: 필드와 데이터타입 등을 보냄
        self.shared_state.sender.send(ChannelRequest {
            statement: statement.to_owned(),
        });

        Ok(vec![
            FieldDescription {
                name: "test".to_owned(),
                data_type: DataTypeOid::Int4,
            },
            FieldDescription {
                name: "test2".to_owned(),
                data_type: DataTypeOid::Int4,
            },
        ])
    }

    async fn create_portal(&mut self, _: &SQLStatement) -> Result<Self::PortalType, ErrorResponse> {
        Ok(RRDBPortal {})
    }
}
