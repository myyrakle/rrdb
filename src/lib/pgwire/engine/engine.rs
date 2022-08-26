use crate::protocol::{ErrorResponse, FieldDescription};
use crate::protocol_ext::DataRowBatch;
use async_trait::async_trait;
use sqlparser::ast::Statement;

/// The engine trait is the core of the `convergence` crate, and is responsible for dispatching most SQL operations.
///
/// Each connection is allocated an [Engine] instance, which it uses to prepare statements, create portals, etc.
#[async_trait]
pub trait Engine: Send + Sync + 'static {
    /// The [Portal] implementation used by [Engine::create_portal].
    type PortalType: Portal;

    /// Prepares a statement, returning a vector of field descriptions for the final statement result.
    async fn prepare(&mut self, stmt: &Statement) -> Result<Vec<FieldDescription>, ErrorResponse>;

    /// Creates a new portal for the given statement.
    async fn create_portal(&mut self, stmt: &Statement) -> Result<Self::PortalType, ErrorResponse>;
}