use crate::engine::DBEngine;
use crate::engine::wal::endec::WALEncoder;
use crate::engine::wal::manager::WALManager;
use crate::engine::wal::types::WALEntry;
use crate::errors;

impl DBEngine {
    pub(crate) async fn recover_from_wal<T>(
        &self,
        wal_manager: &mut WALManager<T>,
    ) -> errors::Result<()>
    where
        T: WALEncoder<WALEntry>,
    {
        self.replay_wal(wal_manager.pending_entries()).await?;
        self.flush_row_buffers_durable().await?;
        wal_manager.flush().await
    }
}
