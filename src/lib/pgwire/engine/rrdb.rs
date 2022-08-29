use async_trait::async_trait;

use crate::lib::ast::predule::SQLStatement;
use crate::lib::pgwire::engine::{Engine, Portal};
use crate::lib::pgwire::protocol::{DataRowBatch, DataTypeOid, ErrorResponse, FieldDescription};

pub struct RRDBPortal;

#[async_trait]
impl Portal for RRDBPortal {
    async fn fetch(&mut self, batch: &mut DataRowBatch) -> Result<(), ErrorResponse> {
        let mut row = batch.create_row();
        // 실제 데이터 리스트를 보냄
        row.write_int4(1);
        Ok(())
    }
}

pub struct RRDBEngine;

#[async_trait]
impl Engine for RRDBEngine {
    type PortalType = RRDBPortal;

    async fn prepare(
        &mut self,
        _statement: &SQLStatement,
    ) -> Result<Vec<FieldDescription>, ErrorResponse> {
        // TODO: 필드와 데이터타입 등을 보냄

        Ok(vec![FieldDescription {
            name: "test".to_owned(),
            data_type: DataTypeOid::Int4,
        }])
    }

    async fn create_portal(&mut self, _: &SQLStatement) -> Result<Self::PortalType, ErrorResponse> {
        Ok(RRDBPortal)
    }
}
