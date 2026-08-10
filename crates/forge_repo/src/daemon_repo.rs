use std::path::PathBuf;
use std::sync::Arc;

use forge_dbd::client::DbClient;
use forge_dbd::protocol::{Request, Response};
use forge_domain::{
    Conversation, ConversationId, ConversationRepository, ConversationSummary,
    ForgeExportOptions, ForgeExportReport, ForgeForgetOptions, ForgeForgetReport,
    ForgeImportOptions, ForgeImportReport, ForgeMigrateReport, HeliosdoctorDbStats,
    MigrateOptions,
};
use tracing::{debug, warn};

use crate::conversation::ConversationRepositoryImpl;

/// Decorator that routes the hot-path conversation WRITES through the
/// `forge-dbd` single-writer daemon while keeping reads on the direct diesel
/// path (P3 design: writes serialise in the daemon, reads stay latency
/// sensitive and local).
///
/// Only four methods are routed to the daemon — `upsert_conversation`,
/// `upsert_conversation_ref`, `update_parent_id`, `delete_conversation` — the
/// exact set the daemon protocol implements. Every other
/// [`ConversationRepository`] method is a plain pass-through to `inner`.
///
/// ## Sync-over-async mechanics
///
/// The original plan called for a lazily-built current-thread tokio Runtime
/// with `block_on`, on the assumption that the trait methods were SYNC. That
/// assumption does not hold: `ConversationRepository` is an `async_trait`, so
/// these methods are async and are invoked from tokio worker contexts
/// (`ForgeConversationService` and friends await the trait directly). The
/// daemon round-trip is therefore a plain `.await` on `DbClient::send` — no
/// dedicated Runtime, no `Mutex`, no `block_on`-reentrancy hazard, and no
/// `Handle::current()` risk. We deliberately do NOT spawn a separate runtime
/// or wrap the trait in sync form, which would only add complexity.
///
/// ## Client lifecycle
///
/// `DbClient` is created lazily on the first routed write and cached in a
/// `tokio::sync::OnceCell`. A failed connect leaves the cell empty, so the
/// next write retries (best-effort reconnect). `DbClient::send` opens a fresh
/// connection per request anyway (client.rs), so caching only skips the
/// connect probe.
///
/// ## Fallback semantics
///
/// The daemon path is BEST-EFFORT: on any failure — connect error, send error,
/// `Response::Error`, or an unexpected response — we log and fall back to
/// `inner`'s direct implementation. A daemon error is never propagated to the
/// caller. Because the daemon write did not happen when we fall back, the
/// direct path cannot double-write.
pub struct DaemonConversationRepository {
    inner: Arc<ConversationRepositoryImpl>,
    socket_path: PathBuf,
    client: tokio::sync::OnceCell<DbClient>,
}

impl DaemonConversationRepository {
    pub fn new(inner: Arc<ConversationRepositoryImpl>, socket_path: PathBuf) -> Self {
        Self { inner, socket_path, client: tokio::sync::OnceCell::new() }
    }

    /// Best-effort daemon round-trip for a write request.
    ///
    /// Returns `true` when the daemon acknowledged the request. Returns
    /// `false` (after logging) on any failure, so the caller falls back to
    /// the direct implementation.
    async fn try_daemon(&self, request: Request) -> bool {
        let client = match self
            .client
            .get_or_try_init(|| DbClient::connect(&self.socket_path))
            .await
        {
            Ok(client) => client,
            Err(err) => {
                debug!(
                    socket = %self.socket_path.display(),
                    error = %err,
                    "forge_dbd unavailable; falling back to direct write"
                );
                return false;
            }
        };
        match client.send(request).await {
            Ok(Response::Ack) => true,
            Ok(Response::Error { message }) => {
                warn!(
                    message = %message,
                    "forge_dbd rejected write; falling back to direct write"
                );
                false
            }
            Ok(other) => {
                debug!(
                    response = ?other,
                    "forge_dbd returned an unexpected response; falling back to direct write"
                );
                false
            }
            Err(err) => {
                warn!(error = %err, "forge_dbd send failed; falling back to direct write");
                false
            }
        }
    }
}

#[async_trait::async_trait]
impl ConversationRepository for DaemonConversationRepository {
    // -----------------------------------------------------------------------
    // Daemon-routed writes
    // -----------------------------------------------------------------------

    async fn upsert_conversation(&self, conversation: Conversation) -> anyhow::Result<()> {
        if self
            .try_daemon(Request::UpsertConversation { conversation: conversation.clone() })
            .await
        {
            return Ok(());
        }
        self.inner.upsert_conversation(conversation).await
    }

    async fn upsert_conversation_ref(&self, conversation: &Conversation) -> anyhow::Result<()> {
        if self
            .try_daemon(Request::UpsertConversationRef { conversation: conversation.clone() })
            .await
        {
            return Ok(());
        }
        self.inner.upsert_conversation_ref(conversation).await
    }

    async fn update_parent_id(
        &self,
        conversation_id: &ConversationId,
        new_parent_id: Option<&ConversationId>,
    ) -> anyhow::Result<()> {
        if self
            .try_daemon(Request::UpdateParentId {
                conversation_id: *conversation_id,
                new_parent_id: new_parent_id.cloned(),
            })
            .await
        {
            return Ok(());
        }
        self.inner.update_parent_id(conversation_id, new_parent_id).await
    }

    async fn delete_conversation(&self, conversation_id: &ConversationId) -> anyhow::Result<()> {
        if self
            .try_daemon(Request::DeleteConversation { conversation_id: *conversation_id })
            .await
        {
            return Ok(());
        }
        self.inner.delete_conversation(conversation_id).await
    }

    // -----------------------------------------------------------------------
    // Pass-throughs (reads + maintenance stay direct)
    // -----------------------------------------------------------------------

    async fn get_conversation(
        &self,
        conversation_id: &ConversationId,
    ) -> anyhow::Result<Option<Conversation>> {
        self.inner.get_conversation(conversation_id).await
    }

    async fn get_all_conversations(
        &self,
        limit: Option<usize>,
    ) -> anyhow::Result<Option<Vec<Conversation>>> {
        self.inner.get_all_conversations(limit).await
    }

    async fn get_last_conversation(&self) -> anyhow::Result<Option<Conversation>> {
        self.inner.get_last_conversation().await
    }

    async fn get_conversations_by_parent(
        &self,
        parent_id: &ConversationId,
    ) -> anyhow::Result<Option<Vec<Conversation>>> {
        self.inner.get_conversations_by_parent(parent_id).await
    }

    async fn get_parent_conversations(
        &self,
        limit: Option<usize>,
    ) -> anyhow::Result<Option<Vec<Conversation>>> {
        self.inner.get_parent_conversations(limit).await
    }

    async fn get_parent_conversations_lite(
        &self,
        limit: Option<usize>,
        all_workspaces: bool,
    ) -> anyhow::Result<Option<Vec<ConversationSummary>>> {
        self.inner.get_parent_conversations_lite(limit, all_workspaces).await
    }

    async fn get_conversations_by_source(
        &self,
        source: &str,
        limit: Option<usize>,
    ) -> anyhow::Result<Option<Vec<Conversation>>> {
        self.inner.get_conversations_by_source(source, limit).await
    }

    async fn search_conversations(
        &self,
        query: &str,
        limit: Option<usize>,
    ) -> anyhow::Result<Vec<Conversation>> {
        self.inner.search_conversations(query, limit).await
    }

    async fn get_conversation_snippet(
        &self,
        conversation_id: &ConversationId,
        query: &str,
        token_count: usize,
    ) -> anyhow::Result<Option<String>> {
        self.inner.get_conversation_snippet(conversation_id, query, token_count).await
    }

    async fn optimize_fts_index(&self) -> anyhow::Result<()> {
        self.inner.optimize_fts_index().await
    }

    async fn refresh_fts_index(&self) -> anyhow::Result<()> {
        self.inner.refresh_fts_index().await
    }

    async fn get_conversations_by_cwd(
        &self,
        cwd: &str,
        limit: Option<usize>,
    ) -> anyhow::Result<Option<Vec<Conversation>>> {
        self.inner.get_conversations_by_cwd(cwd, limit).await
    }

    async fn mark_intent_state(
        &self,
        conversation_id: &ConversationId,
        new_state: &str,
    ) -> anyhow::Result<()> {
        self.inner.mark_intent_state(conversation_id, new_state).await
    }

    async fn list_prune_eligible(
        &self,
        workspace_id: Option<i64>,
        limit: usize,
    ) -> anyhow::Result<Vec<Conversation>> {
        self.inner.list_prune_eligible(workspace_id, limit).await
    }

    async fn prune_conversation(&self, conversation_id: &ConversationId) -> anyhow::Result<()> {
        self.inner.prune_conversation(conversation_id).await
    }

    async fn rewind_conversation(
        &self,
        conversation_id: &ConversationId,
    ) -> anyhow::Result<Option<Conversation>> {
        self.inner.rewind_conversation(conversation_id).await
    }

    async fn compress_uncompressed_contexts(&self) -> anyhow::Result<(usize, usize, usize)> {
        self.inner.compress_uncompressed_contexts().await
    }

    async fn import_forge_db(&self, source: PathBuf) -> anyhow::Result<ForgeImportReport> {
        self.inner.import_forge_db(source).await
    }

    async fn import_forge_db_with_options(
        &self,
        source: PathBuf,
        options: &ForgeImportOptions,
    ) -> anyhow::Result<ForgeImportReport> {
        self.inner.import_forge_db_with_options(source, options).await
    }

    async fn export_forge_db(
        &self,
        dest: PathBuf,
        options: &ForgeExportOptions,
    ) -> anyhow::Result<ForgeExportReport> {
        self.inner.export_forge_db(dest, options).await
    }

    async fn database_stats(&self) -> anyhow::Result<HeliosdoctorDbStats> {
        self.inner.database_stats().await
    }

    async fn forget_conversations(
        &self,
        options: &ForgeForgetOptions,
    ) -> anyhow::Result<ForgeForgetReport> {
        self.inner.forget_conversations(options).await
    }

    async fn migrate_data_dir(
        &self,
        options: &MigrateOptions,
    ) -> anyhow::Result<ForgeMigrateReport> {
        self.inner.migrate_data_dir(options).await
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use forge_domain::{Conversation, ConversationId, WorkspaceHash};
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::conversation::ConversationRepositoryImpl;
    use crate::database::DatabasePool;

    fn in_memory_inner() -> Arc<ConversationRepositoryImpl> {
        let pool = Arc::new(DatabasePool::in_memory().expect("in-memory pool"));
        Arc::new(ConversationRepositoryImpl::new(pool, WorkspaceHash::new(0)))
    }

    /// The decorator must fall back to the direct implementation when the
    /// daemon is absent: a socket path that does not exist makes
    /// `DbClient::connect` fail, and the write still lands in the inner
    /// repository.
    #[tokio::test]
    async fn falls_back_to_direct_write_when_daemon_absent() -> anyhow::Result<()> {
        let inner = in_memory_inner();
        let repo = DaemonConversationRepository::new(
            inner.clone(),
            PathBuf::from("/nonexistent/.forge.db.sock"),
        );

        let conversation =
            Conversation::new(ConversationId::generate()).title(Some("daemon-fallback".to_string()));
        let id = conversation.id;

        repo.upsert_conversation(conversation).await?;

        let actual = inner.get_conversation(&id).await?;
        assert_eq!(actual.expect("row persisted via direct fallback").title, Some("daemon-fallback".to_string()));
        Ok(())
    }
}
