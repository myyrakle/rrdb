use async_trait::async_trait;

use crate::lib::pgwire::protocol::{DataRowBatch, ErrorResponse};

/// A Postgres portal. Portals represent a prepared statement with all parameters specified.
///
/// See Postgres' protocol docs regarding the [extended query overview](https://www.postgresql.org/docs/current/protocol-overview.html#PROTOCOL-QUERY-CONCEPTS)
/// for more details.
#[async_trait]
pub trait Portal: Send + Sync {
    /// Fetches the contents of the portal into a [DataRowBatch].
    async fn fetch(&mut self, batch: &mut DataRowBatch) -> Result<(), ErrorResponse>;
}
