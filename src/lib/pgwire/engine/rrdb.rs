use async_trait::async_trait;
use sqlparser::ast::{Expr, SelectItem, SetExpr, Statement};

use crate::lib::pgwire::engine::{Engine, Portal};
use crate::lib::pgwire::protocol::{
    DataRowBatch, DataTypeOid, ErrorResponse, FieldDescription, SqlState,
};

pub struct RRDBPortal;

#[async_trait]
impl Portal for RRDBPortal {
    async fn fetch(&mut self, batch: &mut DataRowBatch) -> Result<(), ErrorResponse> {
        let mut row = batch.create_row();
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
        statement: &Statement,
    ) -> Result<Vec<FieldDescription>, ErrorResponse> {
        if let Statement::Query(query) = &statement {
            if let SetExpr::Select(select) = *query.body.clone() {
                if select.projection.len() == 1 {
                    if let SelectItem::UnnamedExpr(Expr::Identifier(column_name)) =
                        &select.projection[0]
                    {
                        match column_name.value.as_str() {
                            "test_error" => {
                                return Err(ErrorResponse::error(
                                    SqlState::DATA_EXCEPTION,
                                    "test error",
                                ))
                            }
                            "test_fatal" => {
                                return Err(ErrorResponse::fatal(
                                    SqlState::DATA_EXCEPTION,
                                    "fatal error",
                                ))
                            }
                            _ => (),
                        }
                    }
                }
            }
        }

        Ok(vec![FieldDescription {
            name: "test".to_owned(),
            data_type: DataTypeOid::Int4,
        }])
    }

    async fn create_portal(&mut self, _: &Statement) -> Result<Self::PortalType, ErrorResponse> {
        Ok(RRDBPortal)
    }
}
