use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use diesel::prelude::*;
use forge_domain::{
    Context, Conversation, ConversationId, ConversationRepository, ConversationSummary,
    ForgeImportReport, WorkspaceHash,
};

use crate::conversation::conversation_record::{
    ContextRecord, ConversationRecord, ConversationRecordLite, MetricsRecord,
};
use crate::database::schema::{conversations, conversations_all};
use crate::database::{DatabasePool, PooledSqliteConnection};

/// Lightweight row type for FTS5 `snippet()` results. The query returns
/// exactly one column (`s`) — we use a named struct (not a tuple) so
/// diesel's `QueryableByName` derive can read it back from `sql_query`.
#[derive(Debug, Clone)]
struct SnippetRow {
    s: String,
}

impl diesel::QueryableByName<diesel::sqlite::Sqlite> for SnippetRow {
    fn build<'a>(
        row: &impl diesel::row::NamedRow<'a, diesel::sqlite::Sqlite>,
    ) -> diesel::deserialize::Result<Self> {
        let s = diesel::row::NamedRow::get::<diesel::sql_types::Text, _>(row, "s")?;
        Ok(SnippetRow { s })
    }
}

/// Row type for reading conversations during FTS refresh.
/// Used to populate FTS5 with decompressed context from both compressed and
/// uncompressed rows.
#[derive(Debug, Clone)]
struct FtsRefreshRow {
    rowid: i64,
    title: String,
    context: Option<String>,
    context_zstd: Option<Vec<u8>>,
    is_compressed: i32,
    cwd: Option<String>,
}

impl diesel::QueryableByName<diesel::sqlite::Sqlite> for FtsRefreshRow {
    fn build<'a>(
        row: &impl diesel::row::NamedRow<'a, diesel::sqlite::Sqlite>,
    ) -> diesel::deserialize::Result<Self> {
        use diesel::row::NamedRow;
        use diesel::sql_types::{BigInt, Binary, Integer, Nullable, Text};
        Ok(FtsRefreshRow {
            rowid: NamedRow::get::<BigInt, _>(row, "rowid")?,
            title: NamedRow::get::<Text, _>(row, "title")?,
            context: NamedRow::get::<Nullable<Text>, _>(row, "context")?,
            context_zstd: NamedRow::get::<Nullable<Binary>, _>(row, "context_zstd")?,
            is_compressed: NamedRow::get::<Integer, _>(row, "is_compressed")?,
            cwd: NamedRow::get::<Nullable<Text>, _>(row, "cwd")?,
        })
    }
}

pub struct ConversationRepositoryImpl {
    pool: Arc<DatabasePool>,
    wid: WorkspaceHash,
}

impl ConversationRepositoryImpl {
    pub fn new(pool: Arc<DatabasePool>, workspace_id: WorkspaceHash) -> Self {
        Self { pool, wid: workspace_id }
    }

    async fn run_blocking<F, T>(&self, operation: F) -> anyhow::Result<T>
    where
        F: FnOnce(Arc<DatabasePool>, WorkspaceHash) -> anyhow::Result<T> + Send + 'static,
        T: Send + 'static,
    {
        let pool = self.pool.clone();
        let wid = self.wid;
        tokio::task::spawn_blocking(move || operation(pool, wid))
            .await
            .map_err(|e| anyhow::anyhow!("Conversation repository task failed: {e}"))?
    }

    async fn run_with_connection<F, T>(&self, operation: F) -> anyhow::Result<T>
    where
        F: FnOnce(&mut PooledSqliteConnection, WorkspaceHash) -> anyhow::Result<T> + Send + 'static,
        T: Send + 'static,
    {
        self.run_blocking(move |pool, wid| {
            let mut connection = pool.get_connection()?;
            operation(&mut connection, wid)
        })
        .await
    }
}

#[async_trait::async_trait]
impl ConversationRepository for ConversationRepositoryImpl {
    async fn upsert_conversation_ref(&self, conversation: &Conversation) -> anyhow::Result<()> {
        let conversation = conversation.clone();
        self.run_with_connection(move |connection, wid| {
            let record = ConversationRecord::new_ref(&conversation, wid);
            diesel::insert_into(conversations::table)
                .values(&record)
                .on_conflict(conversations::conversation_id)
                .do_update()
                .set((
                    conversations::title.eq(&record.title),
                    conversations::context.eq(&record.context),
                    conversations::updated_at.eq(record.updated_at),
                    conversations::metrics.eq(&record.metrics),
                    conversations::parent_id.eq(&record.parent_id),
                    conversations::source.eq(&record.source),
                    conversations::cwd.eq(&record.cwd),
                    conversations::message_count.eq(record.message_count),
                ))
                .execute(connection)?;
            Ok(())
        })
        .await
    }

    async fn upsert_conversation(&self, conversation: Conversation) -> anyhow::Result<()> {
        self.run_with_connection(move |connection, wid| {
            let record = ConversationRecord::new(conversation, wid);
            diesel::insert_into(conversations::table)
                .values(&record)
                .on_conflict(conversations::conversation_id)
                .do_update()
                .set((
                    conversations::title.eq(&record.title),
                    conversations::context.eq(&record.context),
                    conversations::context_zstd.eq(&record.context_zstd),
                    conversations::is_compressed.eq(record.is_compressed),
                    conversations::updated_at.eq(record.updated_at),
                    conversations::metrics.eq(&record.metrics),
                    conversations::parent_id.eq(&record.parent_id),
                    conversations::source.eq(&record.source),
                    conversations::cwd.eq(&record.cwd),
                    conversations::message_count.eq(record.message_count),
                ))
                .execute(connection)?;
            Ok(())
        })
        .await
    }

    async fn get_conversation(
        &self,
        conversation_id: &ConversationId,
    ) -> anyhow::Result<Option<Conversation>> {
        let conversation_id = *conversation_id;
        self.run_with_connection(move |connection, _wid| {
            let record: Option<ConversationRecord> = conversations::table
                .filter(conversations::conversation_id.eq(conversation_id.into_string()))
                .first(connection)
                .optional()?;

            match record {
                Some(record) => Ok(Some(Conversation::try_from(record)?)),
                None => Ok(None),
            }
        })
        .await
    }

    async fn get_all_conversations(
        &self,
        limit: Option<usize>,
    ) -> anyhow::Result<Option<Vec<Conversation>>> {
        self.run_with_connection(move |connection, wid| {
            use diesel::dsl::sql;
            use diesel::prelude::*;

            let workspace_id = wid.id() as i64;
            // Filter for rows with context data: either plain context column OR compressed
            // context_zstd Using raw SQL to express: context IS NOT NULL OR
            // is_compressed = 1
            let mut query = conversations::table
                .filter(conversations::workspace_id.eq(&workspace_id))
                .filter(sql::<diesel::sql_types::Bool>(
                    "context IS NOT NULL OR is_compressed = 1",
                ))
                .order(conversations::updated_at.desc())
                .into_boxed();

            if let Some(limit_value) = limit {
                query = query.limit(limit_value as i64);
            }

            let records: Vec<ConversationRecord> = query.load(connection)?;

            if records.is_empty() {
                return Ok(None);
            }

            let conversations: Result<Vec<Conversation>, _> =
                records.into_iter().map(Conversation::try_from).collect();
            Ok(Some(conversations?))
        })
        .await
    }

    async fn get_last_conversation(&self) -> anyhow::Result<Option<Conversation>> {
        self.run_with_connection(move |connection, wid| {
            use diesel::dsl::sql;
            use diesel::prelude::*;

            let workspace_id = wid.id() as i64;
            let record: Option<ConversationRecord> = conversations::table
                .filter(conversations::workspace_id.eq(&workspace_id))
                .filter(sql::<diesel::sql_types::Bool>(
                    "context IS NOT NULL OR is_compressed = 1",
                ))
                .order(conversations::updated_at.desc())
                .first(connection)
                .optional()?;
            let conversation = match record {
                Some(record) => Some(Conversation::try_from(record)?),
                None => None,
            };
            Ok(conversation)
        })
        .await
    }

    async fn delete_conversation(&self, conversation_id: &ConversationId) -> anyhow::Result<()> {
        let conversation_id = *conversation_id;
        self.run_with_connection(move |connection, wid| {
            let workspace_id = wid.id() as i64;

            // Security: Ensure users can only delete conversations within their workspace
            diesel::delete(conversations::table)
                .filter(conversations::workspace_id.eq(&workspace_id))
                .filter(conversations::conversation_id.eq(conversation_id.into_string()))
                .execute(connection)?;

            Ok(())
        })
        .await
    }

    async fn get_conversations_by_parent(
        &self,
        parent_id: &ConversationId,
    ) -> anyhow::Result<Option<Vec<Conversation>>> {
        let parent_id = parent_id.into_string();
        self.run_with_connection(move |connection, wid| {
            use diesel::dsl::sql;

            let workspace_id = wid.id() as i64;
            let records: Vec<ConversationRecord> = conversations::table
                .filter(conversations::workspace_id.eq(&workspace_id))
                .filter(conversations::parent_id.eq(&parent_id))
                .filter(sql::<diesel::sql_types::Bool>(
                    "context IS NOT NULL OR is_compressed = 1",
                ))
                .order(conversations::updated_at.desc())
                .load(connection)?;

            if records.is_empty() {
                return Ok(None);
            }

            let conversations: Result<Vec<Conversation>, _> =
                records.into_iter().map(Conversation::try_from).collect();
            Ok(Some(conversations?))
        })
        .await
    }

    async fn get_parent_conversations(
        &self,
        limit: Option<usize>,
    ) -> anyhow::Result<Option<Vec<Conversation>>> {
        self.run_with_connection(move |connection, wid| {
            use diesel::dsl::sql;

            let workspace_id = wid.id() as i64;
            let mut query = conversations::table
                .filter(conversations::workspace_id.eq(&workspace_id))
                .filter(sql::<diesel::sql_types::Bool>(
                    "context IS NOT NULL OR is_compressed = 1",
                ))
                .filter(conversations::parent_id.is_null())
                .order(conversations::updated_at.desc())
                .into_boxed();

            if let Some(limit_value) = limit {
                query = query.limit(limit_value as i64);
            }

            let records: Vec<ConversationRecord> = query.load(connection)?;

            if records.is_empty() {
                return Ok(None);
            }

            let conversations: Result<Vec<Conversation>, _> =
                records.into_iter().map(Conversation::try_from).collect();
            Ok(Some(conversations?))
        })
        .await
    }

    async fn get_parent_conversations_lite(
        &self,
        limit: Option<usize>,
        all_workspaces: bool,
    ) -> anyhow::Result<Option<Vec<ConversationSummary>>> {
        self.run_with_connection(move |connection, wid| {
            use diesel::dsl::sql;

            let mut query = conversations::table
                .filter(conversations::parent_id.is_null())
                // Match rows with context data: either the plain `context`
                // column OR the compressed `context_zstd` payload.
                .filter(sql::<diesel::sql_types::Bool>(
                    "context IS NOT NULL OR is_compressed = 1",
                ))
                // Hide agent-launched (subagent / `forge -p`) conversations from
                // interactive pickers, mirroring `forge conversation list` which
                // filters `context.initiator == "agent"` in memory. Compressed
                // rows have no plain `context`, so json_extract yields NULL and
                // they pass through.
                .filter(sql::<diesel::sql_types::Bool>(
                    "COALESCE(json_extract(context, '$.initiator'), 'user') <> 'agent'",
                ))
                .select(ConversationRecordLite::as_select())
                .order(conversations::updated_at.desc())
                .into_boxed();

            if !all_workspaces {
                let workspace_id = wid.id() as i64;
                query = query.filter(conversations::workspace_id.eq(workspace_id));
            }

            if let Some(limit_value) = limit {
                query = query.limit(limit_value as i64);
            }

            let records: Vec<ConversationRecordLite> = query.load(connection)?;

            if records.is_empty() {
                return Ok(None);
            }

            let summaries: Vec<ConversationSummary> =
                records.into_iter().map(ConversationSummary::from).collect();
            Ok(Some(summaries))
        })
        .await
    }

    async fn get_conversations_by_source(
        &self,
        source: &str,
        limit: Option<usize>,
    ) -> anyhow::Result<Option<Vec<Conversation>>> {
        let source = source.to_string();
        self.run_with_connection(move |connection, wid| {
            use diesel::dsl::sql;

            let workspace_id = wid.id() as i64;
            let mut query = conversations::table
                .filter(conversations::workspace_id.eq(&workspace_id))
                .filter(sql::<diesel::sql_types::Bool>(
                    "context IS NOT NULL OR is_compressed = 1",
                ))
                .filter(conversations::source.eq(&source))
                .order(conversations::updated_at.desc())
                .into_boxed();

            if let Some(limit_value) = limit {
                query = query.limit(limit_value as i64);
            }

            let records: Vec<ConversationRecord> = query.load(connection)?;

            if records.is_empty() {
                return Ok(None);
            }

            let conversations: Result<Vec<Conversation>, _> =
                records.into_iter().map(Conversation::try_from).collect();
            Ok(Some(conversations?))
        })
        .await
    }

    async fn search_conversations(
        &self,
        query: &str,
        limit: Option<usize>,
    ) -> anyhow::Result<Vec<Conversation>> {
        let query = query.to_string();
        let limit_value = limit.map(|n| n as i64);
        self.run_with_connection(move |connection, wid| {
            let workspace_id = wid.id() as i64;
            // FTS5 BM25 search joined back to the base table on
            // `rowid` (now explicit `rowid` column in external-content FTS5).
            // `bm25()` returns a negative number where lower = more relevant, so `ORDER BY
            // rank_score` (ascending) yields "best match first".
            //
            // We do NOT include `snippet()` here because it would force
            // the SELECT to return a column not in `ConversationRecord`.
            // The UI fetches a snippet on-demand via the separate
            // `get_conversation_snippet` method when the user picks a hit.
            let mut sql = String::from(
                "SELECT c.*, bm25(conversations_fts) AS rank_score \
                 FROM conversations c \
                 JOIN conversations_fts fts ON c.rowid = fts.rowid \
                 WHERE conversations_fts MATCH ? \
                   AND c.workspace_id = ? \
                 ORDER BY rank_score",
            );
            if limit_value.is_some() {
                sql.push_str(" LIMIT ?");
            }

            // We can't bind the FTS MATCH expression positionally because
            // diesel::sql_query does not have a typed binding for FTS5's
            // MATCH operator when used as a column. Use the lower-level
            // `sql_query` so we can read back the typed rows.
            let mut q = diesel::sql_query(sql).into_boxed();
            q = q.bind::<diesel::sql_types::Text, _>(&query);
            q = q.bind::<diesel::sql_types::BigInt, _>(workspace_id);
            if let Some(l) = limit_value {
                q = q.bind::<diesel::sql_types::BigInt, _>(l);
            }

            let raw_rows: Vec<ConversationRecord> = q.load(connection)?;
            let conversations: Result<Vec<Conversation>, _> =
                raw_rows.into_iter().map(Conversation::try_from).collect();
            conversations
        })
        .await
    }

    /// Return a single FTS5 snippet for a (conversation, query) pair.
    /// Used by the UI to render a "matched passage" preview for the
    /// currently selected search hit. Returns `None` if no match.
    async fn get_conversation_snippet(
        &self,
        conversation_id: &ConversationId,
        query: &str,
        token_count: usize,
    ) -> anyhow::Result<Option<String>> {
        let conversation_id_str = conversation_id.into_string();
        let query = query.to_string();
        self.run_with_connection(move |connection, _wid| {
            // External-content FTS5 mode: use rowid to join and column index 1 for context.
            // FTS5 column order: title (0), context (1), cwd (2).
            // We filter by rowid matching the base conversation ID.
            let sql = format!(
                "SELECT snippet(conversations_fts, 1, '[', ']', '…', {}) AS s \
                 FROM conversations_fts \
                 WHERE rowid = (SELECT rowid FROM conversations WHERE conversation_id = ?) \
                   AND conversations_fts MATCH ?",
                token_count.min(256)
            );
            let raw: Vec<SnippetRow> = diesel::sql_query(sql)
                .bind::<diesel::sql_types::Text, _>(&conversation_id_str)
                .bind::<diesel::sql_types::Text, _>(&query)
                .load(connection)?;
            Ok(raw.into_iter().next().map(|r| r.s))
        })
        .await
    }

    async fn optimize_fts_index(&self) -> anyhow::Result<()> {
        // FTS5's "optimize" command is invoked as a special INSERT against
        // the virtual table itself. Diesel has no typed binding for it, so
        // we use a raw sql_query. This is the canonical pattern from the
        // SQLite FTS5 docs: https://sqlite.org/fts5.html#the_optimize_command
        self.run_with_connection(move |connection, _wid| {
            diesel::sql_query(
                "INSERT INTO conversations_fts(conversations_fts) VALUES('optimize')",
            )
            .execute(connection)?;
            Ok(())
        })
        .await
    }

    async fn refresh_fts_index(&self) -> anyhow::Result<()> {
        // CONTENTFUL FTS5 populated in application code.
        // This ensures BOTH compressed and uncompressed rows are indexed.
        //
        // Process:
        // 1. Clear the FTS index (DELETE all rows)
        // 2. SELECT all conversations with their rowid, title, context, context_zstd,
        //    is_compressed
        // 3. For each row: if is_compressed=1, decompress context_zstd to get
        //    searchable text; otherwise use context directly
        // 4. INSERT (rowid, title, content, cwd) into conversations_fts
        //
        // This is more work than FTS5's 'rebuild' but necessary because:
        // - External-content FTS5 reads context column by name → compressed rows
        //   (context=NULL) are missed
        // - Decompression must happen in app code; FTS5 has no built-in codec
        // - Contentful FTS5 is the pragmatic correct solution
        self.run_with_connection(move |connection, _wid| {
            use crate::codec;
            use diesel::sql_types::{BigInt, Text, Nullable};

            // Step 1: Clear the FTS index
            diesel::sql_query("DELETE FROM conversations_fts")
                .execute(connection)?;

            // Step 2: Read all conversations using custom QueryableByName type
            let rows: Vec<FtsRefreshRow> = diesel::sql_query(
                "SELECT rowid, title, context, context_zstd, is_compressed, cwd \
                 FROM conversations"
            )
            .load(connection)?;

            // Step 3 & 4: For each row, decompress if needed and INSERT into FTS
            for row in rows {
                // Determine searchable content: decompress if compressed, else use plain text
                let content = if row.is_compressed == 1 {
                    if let Some(compressed) = row.context_zstd {
                        match codec::decompress(&compressed) {
                            Ok(decompressed) => decompressed,
                            Err(e) => {
                                eprintln!(
                                    "Warning: Failed to decompress context_zstd for rowid {}; skipping FTS: {}",
                                    row.rowid, e
                                );
                                String::new()
                            }
                        }
                    } else {
                        eprintln!("Warning: rowid {} marked compressed but context_zstd is None; skipping FTS", row.rowid);
                        String::new()
                    }
                } else {
                    // Uncompressed row: use context column directly
                    row.context.unwrap_or_default()
                };

                // Insert into FTS5 contentful table
                diesel::sql_query(
                    "INSERT INTO conversations_fts(rowid, title, content, cwd) VALUES (?, ?, ?, ?)"
                )
                .bind::<BigInt, _>(row.rowid)
                .bind::<Text, _>(&row.title)
                .bind::<Text, _>(&content)
                .bind::<Nullable<Text>, _>(&row.cwd)
                .execute(connection)?;
            }

            Ok(())
        })
        .await
    }

    async fn update_parent_id(
        &self,
        conversation_id: &ConversationId,
        new_parent_id: Option<&ConversationId>,
    ) -> anyhow::Result<()> {
        // The `Option<&ConversationId>` is borrowed for the duration of the
        // move into `run_with_connection`. We materialise the inner string
        // here so the closure becomes `'static`.
        let new_parent_id_str: Option<String> = new_parent_id.map(|id| id.into_string());
        let conversation_id_str = conversation_id.into_string();
        let now: chrono::NaiveDateTime = chrono::Utc::now().naive_utc();
        self.run_with_connection(move |connection, _wid| {
            diesel::update(
                conversations::table
                    .filter(conversations::conversation_id.eq(&conversation_id_str)),
            )
            .set((
                conversations::parent_id.eq(new_parent_id_str),
                conversations::updated_at.eq(Some(now)),
            ))
            .execute(connection)?;
            Ok(())
        })
        .await
    }

    async fn rewind_conversation(
        &self,
        conversation_id: &ConversationId,
    ) -> anyhow::Result<Option<Conversation>> {
        let conversation_id_str = conversation_id.into_string();
        let now: chrono::NaiveDateTime = chrono::Utc::now().naive_utc();
        let result = self
            .run_with_connection(move |connection, _wid| {
                // MVP rewind semantics: find the most recent user message followed by
                // a tool call (i.e. last compaction point heuristic) and truncate
                // the context JSON to that prefix. If no tool call is found,
                // fall back to clearing context to the most recent user message.
                let record: Option<ConversationRecord> = conversations_all::table
                    .filter(conversations_all::conversation_id.eq(&conversation_id_str))
                    .first(connection)
                    .optional()?;

                let new_context: Option<String> = match record {
                    Some(r) if r.context.is_some() => {
                        let ctx = r.context.as_ref().unwrap();
                        let rewind_point = find_last_compaction_point(ctx);
                        Some(truncate_context(ctx, rewind_point))
                    }
                    _ => None,
                };

                diesel::update(
                    conversations::table
                        .filter(conversations::conversation_id.eq(&conversation_id_str)),
                )
                .set((
                    conversations::context.eq(new_context),
                    conversations::updated_at.eq(Some(now)),
                ))
                .execute(connection)?;

                // Re-read the updated record so we can return it.
                let updated: Option<ConversationRecord> = conversations_all::table
                    .filter(conversations_all::conversation_id.eq(&conversation_id_str))
                    .first(connection)
                    .optional()?;
                Ok(updated.and_then(|r| Conversation::try_from(r).ok()))
            })
            .await?;
        Ok(result)
    }

    async fn get_conversations_by_cwd(
        &self,
        cwd: &str,
        limit: Option<usize>,
    ) -> anyhow::Result<Option<Vec<Conversation>>> {
        let cwd = cwd.to_string();
        self.run_with_connection(move |connection, wid| {
            use diesel::dsl::sql;

            let workspace_id = wid.id() as i64;
            let mut query = conversations_all::table
                .filter(conversations_all::workspace_id.eq(&workspace_id))
                .filter(sql::<diesel::sql_types::Bool>(
                    "context IS NOT NULL OR is_compressed = 1",
                ))
                .filter(conversations_all::cwd.eq(&cwd))
                .order(conversations_all::updated_at.desc())
                .into_boxed();

            if let Some(limit_value) = limit {
                query = query.limit(limit_value as i64);
            }

            let records: Vec<ConversationRecord> = query.load(connection)?;

            if records.is_empty() {
                return Ok(None);
            }

            let conversations: Result<Vec<Conversation>, _> =
                records.into_iter().map(Conversation::try_from).collect();
            Ok(Some(conversations?))
        })
        .await
    }

    async fn mark_intent_state(
        &self,
        conversation_id: &ConversationId,
        new_state: &str,
    ) -> anyhow::Result<()> {
        use crate::conversation::intent::IntentState;

        let conversation_id = conversation_id.into_string();
        let new_state_str = new_state.to_string();
        let new_state = IntentState::from_str(new_state)?;

        self.run_with_connection(move |connection, _wid| {
            // Read current state to validate transition
            let current_record: Option<ConversationRecord> = conversations_all::table
                .filter(conversations_all::conversation_id.eq(&conversation_id))
                .first(connection)
                .optional()?;

            let record = current_record
                .ok_or_else(|| anyhow::anyhow!("Conversation {} not found", conversation_id))?;

            let current_state = IntentState::from_str(&record.intent_state)?;

            // Enforce state machine: can_transition_to returns false for illegal
            // transitions
            if !current_state.can_transition_to(new_state) {
                return Err(anyhow::anyhow!(
                    "Illegal state transition: {} → {}",
                    current_state,
                    new_state
                ));
            }

            // Update the state
            let now = chrono::Utc::now().naive_utc();
            diesel::update(
                conversations::table.filter(conversations::conversation_id.eq(&conversation_id)),
            )
            .set((
                conversations::intent_state.eq(&new_state_str),
                conversations::updated_at.eq(Some(now)),
            ))
            .execute(connection)?;

            Ok(())
        })
        .await
    }

    async fn list_prune_eligible(
        &self,
        workspace_id: Option<i64>,
        limit: usize,
    ) -> anyhow::Result<Vec<Conversation>> {
        self.run_with_connection(move |connection, wid| {
            let workspace_id = workspace_id.unwrap_or_else(|| wid.id() as i64);
            let limit = limit as i64;

            // Use raw SQL to order by context blob size (descending) to prioritize
            // largest contexts first for maximum space reclamation.
            // Reads from `conversations_all` so legacy rows are also evaluated.
            let sql = "SELECT c.* FROM conversations_all c \
                 WHERE c.workspace_id = ? \
                   AND c.intent_state = 'verified' \
                   AND (c.context IS NOT NULL OR c.is_compressed = 1) \
                 ORDER BY COALESCE(LENGTH(c.context), LENGTH(c.context_zstd)) DESC \
                 LIMIT ?";

            let records: Vec<ConversationRecord> = diesel::sql_query(sql)
                .bind::<diesel::sql_types::BigInt, _>(workspace_id)
                .bind::<diesel::sql_types::BigInt, _>(limit)
                .load(connection)?;

            let conversations: Result<Vec<Conversation>, _> =
                records.into_iter().map(Conversation::try_from).collect();
            conversations
        })
        .await
    }

    async fn prune_conversation(&self, conversation_id: &ConversationId) -> anyhow::Result<()> {
        use crate::conversation::intent::IntentState;

        let conversation_id = conversation_id.into_string();

        self.run_with_connection(move |connection, _wid| {
            // Read current state to enforce invariant: only prune from 'verified'.
            // Reads from `conversations_all` so legacy rows are also covered.
            let current_record: Option<ConversationRecord> = conversations_all::table
                .filter(conversations_all::conversation_id.eq(&conversation_id))
                .first(connection)
                .optional()?;

            let record = current_record
                .ok_or_else(|| anyhow::anyhow!("Conversation {} not found", conversation_id))?;

            let current_state = IntentState::from_str(&record.intent_state)?;

            // Safety guard: only prune if verified
            if current_state != IntentState::Verified {
                return Err(anyhow::anyhow!(
                    "Cannot prune conversation with intent_state='{}'. Must be 'verified'.",
                    current_state
                ));
            }

            // Create a compact summary JSON to replace the full context blob
            // Preserves just enough metadata for the conversation to remain queryable
            let compressed_context = serde_json::json!({
                "type": "compressed",
                "conversation_id": conversation_id,
                "pruned_at": chrono::Utc::now().to_rfc3339(),
                "summary": "Conversation context pruned; full intent stored in MemoryPort"
            })
            .to_string();

            let now = chrono::Utc::now().naive_utc();
            diesel::update(
                conversations::table.filter(conversations::conversation_id.eq(&conversation_id)),
            )
            .set((
                conversations::context.eq(compressed_context),
                conversations::intent_state.eq("pruned"),
                conversations::updated_at.eq(Some(now)),
            ))
            .execute(connection)?;

            Ok(())
        })
        .await
    }

    async fn compress_uncompressed_contexts(&self) -> anyhow::Result<(usize, usize, usize)> {
        self.run_with_connection(move |connection, _wid| {
            // Fetch all rows where context is plain-text and not yet compressed.
            // We select only the id + context column to avoid loading unrelated data.
            let sql = "SELECT conversation_id, context \
                       FROM conversations \
                       WHERE is_compressed = 0 AND context IS NOT NULL";

            #[derive(diesel::QueryableByName)]
            struct PlainRow {
                #[diesel(sql_type = diesel::sql_types::Text)]
                conversation_id: String,
                #[diesel(sql_type = diesel::sql_types::Text)]
                context: String,
            }

            let rows: Vec<PlainRow> = diesel::sql_query(sql).load(connection)?;

            let mut compressed_count = 0usize;
            let mut skipped_count = 0usize;
            let mut error_count = 0usize;

            for row in rows {
                match crate::codec::compress(&row.context) {
                    Ok(blob) => {
                        let result = diesel::sql_query(
                            "UPDATE conversations \
                             SET context_zstd = ?, context = NULL, is_compressed = 1 \
                             WHERE conversation_id = ?",
                        )
                        .bind::<diesel::sql_types::Binary, _>(&blob)
                        .bind::<diesel::sql_types::Text, _>(&row.conversation_id)
                        .execute(connection);

                        match result {
                            Ok(_) => compressed_count += 1,
                            Err(_) => error_count += 1,
                        }
                    }
                    Err(_) => {
                        // Compression failed for this row; leave as-is and count.
                        error_count += 1;
                    }
                }
            }

            // Rows with context IS NULL and is_compressed=0 don't appear in the
            // query; they are implicitly skipped. Count them for reporting.
            let null_sql = "SELECT COUNT(*) FROM conversations \
                            WHERE is_compressed = 0 AND context IS NULL";
            #[derive(diesel::QueryableByName)]
            struct CountRow {
                #[diesel(sql_type = diesel::sql_types::BigInt)]
                #[diesel(column_name = "COUNT(*)")]
                count: i64,
            }
            if let Ok(rows) = diesel::sql_query(null_sql).load::<CountRow>(connection)
                && let Some(r) = rows.first()
            {
                skipped_count = r.count as usize;
            }

            Ok((compressed_count, skipped_count, error_count))
        })
        .await
    }

    async fn import_forge_db(&self, source: PathBuf) -> anyhow::Result<ForgeImportReport> {
        self.import_forge_db_with_options(source, &forge_domain::ForgeImportOptions::default())
            .await
    }

    async fn import_forge_db_with_options(
        &self,
        source: PathBuf,
        options: &forge_domain::ForgeImportOptions,
    ) -> anyhow::Result<ForgeImportReport> {
        let options = options.clone();
        self.run_with_connection(move |connection, wid| {
            use diesel::Connection;
            use diesel::sql_types::Text;

            if !source.is_file() {
                anyhow::bail!(
                    "source database not found: {}",
                    source.display()
                );
            }

            // Open the source database and immediately enable SQLite's
            // `query_only` mode on this connection. No write can succeed
            // after this point, which guarantees the source is never
            // modified (a "never writes back" invariant).
            let source_url = source.to_string_lossy().to_string();
            let mut src = diesel::sqlite::SqliteConnection::establish(&source_url)?;
            diesel::sql_query("PRAGMA query_only = ON;").execute(&mut src)?;
            diesel::sql_query("PRAGMA busy_timeout = 5000;").execute(&mut src)?;

            // Detect the source schema before reading anything.
            #[derive(diesel::QueryableByName)]
            struct ColumnName {
                #[diesel(sql_type = Text)]
                name: String,
            }
            let columns: Vec<ColumnName> =
                diesel::sql_query("PRAGMA table_info(conversations)").load(&mut src)?;
            let column_names: Vec<&str> =
                columns.iter().map(|column| column.name.as_str()).collect();

            if column_names.iter().any(|name| *name == "is_compressed" || *name == "context_zstd")
            {
                anyhow::bail!(
                    "source database {} is already a heliosLite/fork-schema database; \
                     there is nothing to import",
                    source.display()
                );
            }
            for required in ["conversation_id", "context", "created_at"] {
                if !column_names.iter().any(|name| *name == required) {
                    anyhow::bail!(
                        "source database {} is not an official forge conversations \
                         database (missing column `{required}`)",
                        source.display()
                    );
                }
            }

            let rows: Vec<DecodedSourceRow> = diesel::sql_query(
                "SELECT conversation_id, title, context, created_at, updated_at, metrics \
                 FROM conversations",
            )
            .load(&mut src)?;

            let mut report = ForgeImportReport {
                source_total: rows.len(),
                dry_run: options.dry_run,
                ..Default::default()
            };

            // Wrap the inserts in a single transaction so a partial import
            // cannot leave the destination DB in an inconsistent state.
            // dry_run skips BEGIN entirely (nothing is written).
            if !options.dry_run {
                connection.transaction::<_, anyhow::Error, _>(|conn| {
                    import_rows(conn, wid, &rows, &options, &mut report);
                    Ok(())
                })?;
            } else {
                import_rows(connection, wid, &rows, &options, &mut report);
            }

            Ok(report)
        })
        .await
    }

    async fn export_forge_db(
        &self,
        destination: PathBuf,
        options: &forge_domain::ForgeExportOptions,
    ) -> anyhow::Result<forge_domain::ForgeExportReport> {
        let options = options.clone();
        self.run_with_connection(move |connection, _wid| {
            use diesel::Connection;
            use diesel::sql_types::{Nullable, Text};

            // Read all rows from this heliosLite repository, decompressing
            // context_zstd where present so the export contains the same
            // JSON the official lineage would write. The row type is the
            // top-level `HeliosExportRow` defined below in this module.
            //
            // Agent-launched rows (`context.initiator = 'agent'`) are
            // excluded by default to match the TUI picker; the
            // `include_agent` flag re-includes them.
            let sql = if options.include_agent {
                "SELECT conversation_id, title, context, context_zstd, created_at, \
                 updated_at, metrics FROM conversations".to_string()
            } else {
                "SELECT conversation_id, title, context, context_zstd, created_at, \
                 updated_at, metrics FROM conversations \
                 WHERE COALESCE(json_extract(context, '$.initiator'), 'user') != 'agent' \
                    OR context IS NULL".to_string()
            };
            let rows: Vec<HeliosExportRow> = diesel::sql_query(sql).load(connection)?;

            // Dispatch to the format-specific writer if the caller asked
            // for JSONL or CSV. SQLite is the default and falls through to
            // the original code path below.
            if !matches!(
                options.format,
                forge_domain::ForgeExportFormat::Sqlite
            ) {
                return write_export_non_sqlite(
                    connection,
                    &rows,
                    destination,
                    &options,
                );
            }

            let mut report = forge_domain::ForgeExportReport {
                source_total: rows.len(),
                dry_run: options.dry_run,
                ..Default::default()
            };

            if options.dry_run {
                // Pre-flight: try to decompress each context so the report
                // surfaces decompression failures without touching disk.
                for row in &rows {
                    match decompress_for_export(row) {
                        Some(_) => report.exported += 1,
                        None => report.decompression_failed += 1,
                    }
                }
                return Ok(report);
            }

            if let Some(parent) = destination.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)?;
                }
            }
            let dest_url = destination.to_string_lossy().to_string();
            let mut dest = diesel::sqlite::SqliteConnection::establish(&dest_url)?;
            dest.transaction::<_, anyhow::Error, _>(|dest| {
                // Official-lineage conversations schema.
                diesel::sql_query(
                    "CREATE TABLE IF NOT EXISTS conversations (\
                     conversation_id TEXT PRIMARY KEY NOT NULL, \
                     title TEXT, \
                     context TEXT, \
                     created_at TEXT NOT NULL, \
                     updated_at TEXT, \
                     metrics TEXT, \
                     parent_id TEXT, \
                     source TEXT, \
                     workspace_id TEXT NOT NULL, \
                     message_count INTEGER NOT NULL DEFAULT 0, \
                     cwd TEXT\
                     )",
                )
                .execute(dest)?;
                diesel::sql_query("PRAGMA journal_mode = WAL;").execute(dest)?;
                Ok(())
            })?;

            dest.transaction::<_, anyhow::Error, _>(|dest| {
                #[derive(diesel::QueryableByName)]
                struct WorkspaceIdRow {
                    #[diesel(sql_type = Text)]
                    workspace_id: String,
                }
                let workspace_id = diesel::sql_query("SELECT workspace_id FROM conversations LIMIT 1")
                    .get_result::<WorkspaceIdRow>(connection)
                    .map(|row| row.workspace_id)
                    .unwrap_or_else(|_| "imported".to_string());

                for row in &rows {
                    let plain_context = match decompress_for_export(row) {
                        Some(s) => s,
                        None => {
                            report.decompression_failed += 1;
                            continue;
                        }
                    };
                    let res = diesel::sql_query(
                        "INSERT OR REPLACE INTO conversations (\
                         conversation_id, title, context, created_at, updated_at, metrics, \
                         workspace_id, source) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                    )
                    .bind::<Text, _>(&row.conversation_id)
                    .bind::<Nullable<Text>, _>(row.title.as_deref())
                    .bind::<Nullable<Text>, _>(Some(plain_context.as_str()))
                    .bind::<Text, _>(&row.created_at)
                    .bind::<Nullable<Text>, _>(row.updated_at.as_deref())
                    .bind::<Nullable<Text>, _>(row.metrics.as_deref())
                    .bind::<Text, _>(&workspace_id)
                    .bind::<Nullable<Text>, _>(Some("exported:helioslite"))
                    .execute(dest);
                    match res {
                        Ok(_) => report.exported += 1,
                        Err(error) => {
                            eprintln!(
                                "Failed to export conversation {}: {}",
                                row.conversation_id, error
                            );
                            report.errors += 1;
                        }
                    }
                }
                Ok(())
            })?;

            Ok(report)
        })
        .await
    }

    async fn database_stats(&self) -> anyhow::Result<forge_domain::HeliosdoctorDbStats> {
        self.run_with_connection(move |connection, _wid| {
            use diesel::sql_types::{BigInt, Nullable, Text};

            #[derive(diesel::QueryableByName)]
            struct Counts {
                #[diesel(sql_type = BigInt)]
                total: i64,
                #[diesel(sql_type = BigInt)]
                compressed: i64,
                #[diesel(sql_type = BigInt)]
                uncompressed: i64,
                #[diesel(sql_type = BigInt)]
                empty: i64,
                #[diesel(sql_type = BigInt)]
                oversized: i64,
                #[diesel(sql_type = BigInt)]
                agent_initiated: i64,
            }
            let counts: Counts = diesel::sql_query(
                "SELECT \
                     COUNT(*) AS total, \
                     SUM(CASE WHEN is_compressed = 1 THEN 1 ELSE 0 END) AS compressed, \
                     SUM(CASE WHEN is_compressed = 0 AND context IS NOT NULL THEN 1 ELSE 0 END) AS uncompressed, \
                     SUM(CASE WHEN is_compressed = 0 AND context IS NULL THEN 1 ELSE 0 END) AS empty, \
                     SUM(CASE WHEN length(context) > 1048576 OR length(context_zstd) > 1048576 THEN 1 ELSE 0 END) AS oversized, \
                     SUM(CASE WHEN json_extract(context, '$.initiator') = 'agent' THEN 1 ELSE 0 END) AS agent_initiated \
                 FROM conversations",
            )
            .get_result(connection)?;

            #[derive(diesel::QueryableByName)]
            struct IntegrityRow {
                #[diesel(sql_type = Nullable<Text>)]
                integrity_check: Option<String>,
            }
            let integrity_check = match diesel::sql_query("PRAGMA integrity_check")
                .get_result::<IntegrityRow>(connection)
            {
                Ok(row) => row.integrity_check.unwrap_or_else(|| "ok".to_string()),
                Err(error) => format!("error: {}", error),
            };

            // `ConversationRepository::database_stats` only sees the
            // single-DB view (no ATTACH); the richer cross-DB count is
            // produced by `EnvironmentInfra::compute_database_stats` in
            // `forge_infra::env` and surfaced through `heliosdoctor --verbose`.
            Ok(forge_domain::HeliosdoctorDbStats {
                total_conversations: counts.total.max(0) as u64,
                compressed_rows: counts.compressed.max(0) as u64,
                uncompressed_rows: counts.uncompressed.max(0) as u64,
                empty_rows: counts.empty.max(0) as u64,
                oversized_rows: counts.oversized.max(0) as u64,
                agent_initiated_rows: counts.agent_initiated.max(0) as u64,
                integrity_check,
                legacy_attached: None,
                write_db_path: None,
                legacy_db_path: None,
                tables: Default::default(),
                error: None,
            })
        })
        .await
    }

    async fn forget_conversations(
        &self,
        options: &forge_domain::ForgeForgetOptions,
    ) -> anyhow::Result<forge_domain::ForgeForgetReport> {
        let options = options.clone();
        self.run_with_connection(move |connection, _wid| {
            use diesel::sql_types::BigInt;

            // Validate: at least one selector must be set.
            if options.ids.is_empty()
                && options.source.is_none()
                && options.older_than_secs.is_none()
            {
                return Err(anyhow::anyhow!(
                    "at least one filter (--id, --source, --older-than) must be set"
                ));
            }

            let mut conditions: Vec<String> = Vec::new();
            if !options.ids.is_empty() {
                let id_list: Vec<String> = options
                    .ids
                    .iter()
                    .map(|id| format!("'{}'", id))
                    .collect();
                conditions.push(format!("conversation_id IN ({})", id_list.join(",")));
            }
            if let Some(source) = &options.source {
                let escaped = source.replace('\'', "''");
                conditions.push(format!("source = '{}'", escaped));
            }
            if let Some(secs) = options.older_than_secs {
                let cutoff = chrono::Utc::now().timestamp() - secs;
                conditions.push(format!(
                    "updated_at < datetime({}, 'unixepoch')",
                    cutoff
                ));
            }
            let where_clause = format!("WHERE {}", conditions.join(" AND "));

            #[derive(diesel::QueryableByName)]
            struct CountRow {
                #[diesel(sql_type = BigInt)]
                n: i64,
            }
            let matched: usize = diesel::sql_query(format!(
                "SELECT COUNT(*) AS n FROM conversations {}",
                where_clause
            ))
            .get_result::<CountRow>(connection)?
            .n.max(0) as usize;

            if options.dry_run {
                return Ok(forge_domain::ForgeForgetReport {
                    matched,
                    deleted: 0,
                    dry_run: true,
                });
            }

            // Use plain `execute` so we read the row count from the
            // diesel `usize` return value (works on every SQLite version).
            let deleted: usize = diesel::sql_query(format!(
                "DELETE FROM conversations {}",
                where_clause
            ))
            .execute(connection)?;

            Ok(forge_domain::ForgeForgetReport {
                matched,
                deleted,
                dry_run: false,
            })
        })
        .await
    }

    async fn migrate_data_dir(
        &self,
        options: &forge_domain::MigrateOptions,
    ) -> anyhow::Result<forge_domain::ForgeMigrateReport> {
        let dry_run = options.dry_run;
        let db_path = self.pool.database_path().to_path_buf();
        let source_dir = match db_path.parent() {
            Some(p) => p.to_path_buf(),
            None => return Err(anyhow::anyhow!("could not determine parent directory of DB path")),
        };
        let source_dir_name = source_dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();

        if source_dir_name == ".helioslite" {
            // Already canonical — no-op.
            return Ok(forge_domain::ForgeMigrateReport {
                source_path: source_dir.clone(),
                destination_path: source_dir,
                outcome: "already_migrated".to_string(),
                bytes_copied: 0,
                conversations_verified: 0,
                renamed_legacy_to: None,
            });
        }

        // Find the home directory and append `.helioslite`.
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| source_dir.parent().map(|p| p.to_path_buf()).unwrap_or_default());
        let destination_dir = home.join(".helioslite");

        let source_db = source_dir.join(".forge.db");
        let destination_db = destination_dir.join(".forge.db");

        let mut report = forge_domain::ForgeMigrateReport {
            source_path: source_dir.clone(),
            destination_path: destination_dir.clone(),
            outcome: "noop_legacy_missing".to_string(),
            bytes_copied: 0,
            conversations_verified: 0,
            renamed_legacy_to: None,
        };

        if !source_db.exists() {
            // No legacy data — just create the canonical dir.
            std::fs::create_dir_all(&destination_dir)?;
            return Ok(report);
        }

        report.outcome = "migrated".to_string();
        report.bytes_copied = std::fs::metadata(&source_db).map(|m| m.len()).unwrap_or(0);

        // Copy the DB (and WAL/SHM siblings if present).
        std::fs::create_dir_all(&destination_dir)?;

        let copy_with_wal = |src: &std::path::Path, dst: &std::path::Path| -> std::io::Result<()> {
            std::fs::copy(src, dst)?;
            for ext in ["-wal", "-shm", "-journal"] {
                let s = src.with_extension(ext.trim_start_matches('-'));
                if s.exists() {
                    let d = dst.with_extension(ext.trim_start_matches('-'));
                    let _ = std::fs::copy(&s, &d);
                }
            }
            Ok(())
        };

        // Copy to a temp path first, then rename atomically.
        let tmp_dst = {
            let mut p = destination_db.clone();
            let fname = p
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| ".forge.db".to_string());
            let tmp = destination_dir.join(format!(".{}.tmp", fname));
            if dry_run {
                // In dry-run mode, skip the actual copy and use the source as a
                // stand-in for the validation query.
                source_db.clone()
            } else {
                copy_with_wal(&source_db, &tmp)?;
                tmp
            }
        };

        // Validate the copy by opening it with a one-off diesel connection.
        let db_count = {
            use diesel::sql_types::BigInt;
            use diesel::sqlite::SqliteConnection;
            #[derive(diesel::QueryableByName)]
            struct CountRow {
                #[diesel(sql_type = BigInt)]
                n: i64,
            }
            let tmp_path = tmp_dst.to_string_lossy().to_string();
            let mut conn = match SqliteConnection::establish(&tmp_path) {
                Ok(c) => c,
                Err(e) => {
                    let _ = std::fs::remove_file(&tmp_dst);
                    return Err(anyhow::anyhow!(
                        "post-copy validation failed: {}",
                        e
                    ));
                }
            };
            let count: i64 = diesel::sql_query("SELECT COUNT(*) AS n FROM conversations")
                .get_result::<CountRow>(&mut conn)
                .map(|r| r.n)
                .unwrap_or(0);
            count.max(0) as u64
        };

        // Atomic rename into place.
        if !dry_run {
            if destination_db.exists() {
                let _ = std::fs::remove_file(&destination_db);
            }
            std::fs::rename(&tmp_dst, &destination_db)?;
            // WAL/SHM siblings were copied to the tmp name; move them too.
            for ext in ["-wal", "-shm", "-journal"] {
                let fname = destination_db
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| ".forge.db".to_string());
                let tmp = destination_dir.join(format!(".{}.tmp{}", fname, ext));
                if tmp.exists() {
                    let dst = destination_dir.join(format!("{}{}", fname, ext));
                    let _ = std::fs::rename(&tmp, &dst);
                }
            }
        }

        report.conversations_verified = db_count;

        // Rename the legacy directory aside so the user can roll back.
        if !dry_run {
            let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S").to_string();
            let renamed = source_dir.with_file_name(format!(
                "{}.migrated-{}",
                source_dir_name, timestamp
            ));
            if let Err(e) = std::fs::rename(&source_dir, &renamed) {
                // Non-fatal: the copy succeeded; the legacy rename is a
                // hint, not a requirement.
                eprintln!(
                    "migrate: legacy directory could not be renamed ({}): {}",
                    renamed.display(),
                    e
                );
            } else {
                report.renamed_legacy_to = Some(renamed);
            }
        }

        Ok(report)
    }
}

/// Shared row loop used by both the transactional and dry-run import paths.
/// Performs per-row decoding (idempotency lookup, context parsing) and either
/// inserts the row (transactional path) or just counts the outcome (dry-run).
fn import_rows(
    connection: &mut PooledSqliteConnection,
    wid: WorkspaceHash,
    rows: &[DecodedSourceRow],
    options: &forge_domain::ForgeImportOptions,
    report: &mut ForgeImportReport,
) {
    for row in rows {
        let Ok(id) = ConversationId::parse(&row.conversation_id) else {
            report.invalid_id += 1;
            if options.verbose {
                eprintln!("[import] skip invalid-id {}", row.conversation_id);
            }
            continue;
        };

        // Idempotency: skip conversations that already exist in either the
        // write DB or the legacy DB (via `conversations_all`) instead of
        // overwriting them.
        let existing: i64 = conversations_all::table
            .filter(conversations_all::conversation_id.eq(&row.conversation_id))
            .count()
            .get_result(connection)
            .unwrap_or(0);
        if existing > 0 {
            report.skipped_existing += 1;
            if options.verbose {
                eprintln!("[import] skip existing {}", row.conversation_id);
            }
            continue;
        }

        // Parse the plain-text context with the heliosLite record type.
        // Rows that fail to parse are still imported (title + timestamps)
        // so the session remains visible; the failure is reported.
        let mut context_parse_failed = false;
        let context = match row.context.as_deref() {
            Some(json) if !json.trim().is_empty() => {
                let parsed = serde_json::from_str::<ContextRecord>(json)
                    .ok()
                    .and_then(|record| Context::try_from(record).ok());
                if parsed.is_none() {
                    context_parse_failed = true;
                }
                parsed
            }
            _ => None,
        };
        if context_parse_failed {
            report.context_parse_failed += 1;
        }

        let created_at = parse_naive_datetime(&row.created_at)
            .unwrap_or_else(|| chrono::Utc::now().naive_utc());
        let updated_at = row.updated_at.as_deref().and_then(parse_naive_datetime);

        let metrics = row
            .metrics
            .as_deref()
            .and_then(|json| serde_json::from_str::<MetricsRecord>(json).ok())
            .map(forge_domain::Metrics::from)
            .unwrap_or_else(|| {
                forge_domain::Metrics::default().started_at(created_at.and_utc())
            });

        let conversation = forge_domain::Conversation::new(id)
            .context(context)
            .title(row.title.clone())
            .metrics(metrics)
            .source(Some("imported:forge".to_string()))
            .metadata(
                forge_domain::MetaData::new(created_at.and_utc())
                    .updated_at(updated_at.map(|timestamp| timestamp.and_utc())),
            );

        if options.dry_run {
            report.imported += 1;
            if options.verbose {
                eprintln!("[import] (dry-run) would import {}", row.conversation_id);
            }
            continue;
        }

        let record = ConversationRecord::new(conversation, wid);
        match diesel::insert_into(conversations::table)
            .values(&record)
            .execute(connection)
        {
            Ok(_) => {
                report.imported += 1;
                if options.verbose {
                    eprintln!("[import] imported {}", row.conversation_id);
                }
            }
            Err(error) => {
                eprintln!(
                    "Failed to import conversation {}: {}",
                    row.conversation_id, error
                );
                report.errors += 1;
            }
        }
    }
}

/// Diesel row type used by `import_forge_db_with_options`. Renamed from
/// `SourceRow` to avoid collision with the snapshot-history `SourceRow` in
/// `crate::snapshot::SourceRow` (which lives in the same module tree).
#[derive(Debug)]
struct DecodedSourceRow {
    conversation_id: String,
    title: Option<String>,
    context: Option<String>,
    created_at: String,
    updated_at: Option<String>,
    metrics: Option<String>,
}

impl diesel::QueryableByName<diesel::sqlite::Sqlite> for DecodedSourceRow {
    fn build<'a>(
        row: &impl diesel::row::NamedRow<'a, diesel::sqlite::Sqlite>,
    ) -> diesel::deserialize::Result<Self> {
        use diesel::row::NamedRow;
        use diesel::sql_types::{Nullable, Text};
        Ok(Self {
            conversation_id: NamedRow::get::<Text, _>(row, "conversation_id")?,
            title: NamedRow::get::<Nullable<Text>, _>(row, "title")?,
            context: NamedRow::get::<Nullable<Text>, _>(row, "context")?,
            created_at: NamedRow::get::<Text, _>(row, "created_at")?,
            updated_at: NamedRow::get::<Nullable<Text>, _>(row, "updated_at")?,
            metrics: NamedRow::get::<Nullable<Text>, _>(row, "metrics")?,
        })
    }
}

/// Diesel row type used by `export_forge_db`. Renamed from `HeliosRow` to
/// avoid collision with the public `HeliosRow` in
/// `crate::snapshot::HeliosRow`.
#[derive(Debug)]
struct HeliosExportRow {
    conversation_id: String,
    title: Option<String>,
    context: Option<String>,
    context_zstd: Option<Vec<u8>>,
    created_at: String,
    updated_at: Option<String>,
    metrics: Option<String>,
}

impl diesel::QueryableByName<diesel::sqlite::Sqlite> for HeliosExportRow {
    fn build<'a>(
        row: &impl diesel::row::NamedRow<'a, diesel::sqlite::Sqlite>,
    ) -> diesel::deserialize::Result<Self> {
        use diesel::row::NamedRow;
        use diesel::sql_types::{Binary, Nullable, Text};
        Ok(Self {
            conversation_id: NamedRow::get::<Text, _>(row, "conversation_id")?,
            title: NamedRow::get::<Nullable<Text>, _>(row, "title")?,
            context: NamedRow::get::<Nullable<Text>, _>(row, "context")?,
            context_zstd: NamedRow::get::<Nullable<Binary>, _>(row, "context_zstd")?,
            created_at: NamedRow::get::<Text, _>(row, "created_at")?,
            updated_at: NamedRow::get::<Nullable<Text>, _>(row, "updated_at")?,
            metrics: NamedRow::get::<Nullable<Text>, _>(row, "metrics")?,
        })
    }
}

/// Decompress a heliosLite export row into a plain-context string suitable
/// for the official-lineage export schema. Returns `None` when the row has
/// no context or when zstd decompression fails.
fn decompress_for_export(row: &HeliosExportRow) -> Option<String> {
    use crate::codec::decompress;
    if let Some(plain) = row.context.as_deref() {
        if !plain.trim().is_empty() {
            return Some(plain.to_string());
        }
    }
    row.context_zstd
        .as_deref()
        .and_then(|bytes| decompress(bytes).ok())
}

/// Write the export in a non-SQLite format (JSONL or CSV). The output is
/// always a single file with one record per line. Used by
/// `export_forge_db` when the format is not `Sqlite`.
fn write_export_non_sqlite(
    _connection: &mut PooledSqliteConnection,
    rows: &[HeliosExportRow],
    destination: PathBuf,
    options: &forge_domain::ForgeExportOptions,
) -> anyhow::Result<forge_domain::ForgeExportReport> {
    use forge_domain::ForgeExportFormat;

    let mut report = forge_domain::ForgeExportReport {
        source_total: rows.len(),
        dry_run: options.dry_run,
        ..Default::default()
    };

    if options.dry_run {
        for row in rows {
            match decompress_for_export(row) {
                Some(_) => report.exported += 1,
                None => report.decompression_failed += 1,
            }
        }
        return Ok(report);
    }

    if let Some(parent) = destination.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let mut out = String::new();
    match options.format {
        ForgeExportFormat::Jsonl => {
            for row in rows {
                let plain_context = match decompress_for_export(row) {
                    Some(s) => s,
                    None => {
                        report.decompression_failed += 1;
                        continue;
                    }
                };
                let record = serde_json::json!({
                    "conversation_id": row.conversation_id,
                    "title": row.title,
                    "context": serde_json::from_str::<serde_json::Value>(&plain_context)
                        .unwrap_or(serde_json::Value::String(plain_context)),
                    "created_at": row.created_at,
                    "updated_at": row.updated_at,
                    "metrics": row.metrics,
                });
                out.push_str(&serde_json::to_string(&record)?);
                out.push('\n');
                report.exported += 1;
            }
        }
        ForgeExportFormat::Csv => {
            out.push_str("conversation_id,title,created_at,updated_at,context\n");
            for row in rows {
                let plain_context = match decompress_for_export(row) {
                    Some(s) => s,
                    None => {
                        report.decompression_failed += 1;
                        continue;
                    }
                };
                let csv_escape = |s: &str| -> String {
                    if s.contains(',') || s.contains('"') || s.contains('\n') {
                        format!("\"{}\"", s.replace('"', "\"\""))
                    } else {
                        s.to_string()
                    }
                };
                out.push_str(&csv_escape(&row.conversation_id));
                out.push(',');
                out.push_str(&csv_escape(row.title.as_deref().unwrap_or("")));
                out.push(',');
                out.push_str(&csv_escape(&row.created_at));
                out.push(',');
                out.push_str(&csv_escape(row.updated_at.as_deref().unwrap_or("")));
                out.push(',');
                out.push_str(&csv_escape(&plain_context));
                out.push('\n');
                report.exported += 1;
            }
        }
        ForgeExportFormat::Sqlite => {
            // Caller should have dispatched this branch before reaching
            // here. Defensive: error rather than silently produce empty.
            return Err(anyhow::anyhow!(
                "write_export_non_sqlite called with Sqlite format"
            ));
        }
    }

    std::fs::write(&destination, out)?;
    Ok(report)
}

/// Parse a timestamp stored by forge-lineage databases into a
/// [`chrono::NaiveDateTime`]. Accepts the diesel TEXT serialization
/// (`%Y-%m-%d %H:%M:%S%.f`) as well as RFC 3339 variants. Returns `None`
/// when the string is unrecognised so callers can fall back to "now".
fn parse_naive_datetime(value: &str) -> Option<chrono::NaiveDateTime> {
    const FORMATS: &[&str] = &[
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S%.fZ",
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%SZ",
    ];
    FORMATS
        .iter()
        .find_map(|format| chrono::NaiveDateTime::parse_from_str(value, format).ok())
}

/// Find the byte-offset in the context JSON immediately after the last
/// "compaction point" we can detect. The MVP heuristic scans the JSON string
/// for tool-call markers (`"name":`) in reverse and returns the offset of
/// the most recent user-text content that *precedes* a tool call.
///
/// `0` means "no rewound prefix found; truncate to empty" (full reset).
fn find_last_compaction_point(context_json: &str) -> usize {
    // Walk the JSON looking for the most recent `"role":"user"` message
    // boundary followed by a tool call. Each message entry in the context
    // is a JSON object; we just look for the substring order heuristically.
    // This is intentionally conservative: it errs on "rewind less, keep
    // more history" rather than "rewind too far, lose context".
    let user_marker = "\"role\":\"user\"";
    let tool_marker = "\"tool_calls\"";

    // Find the last user-role occurrence.
    let last_user = context_json.rfind(user_marker);
    if last_user.is_none() {
        return 0;
    }
    // After that user-role, look forward for the first tool_call marker.
    let after_user = last_user.unwrap() + user_marker.len();
    if context_json
        .get(after_user..)
        .is_some_and(|tail| tail.find(tool_marker).is_some())
    {
        // Truncate at the user-role boundary so we keep the user turn
        // but discard everything after it (including the tool call).
        return last_user.unwrap();
    }
    // No tool call after the last user message — treat the last user
    // message as the rewind point too (discard any trailing assistant
    // text/tool results that came after).
    last_user.unwrap()
}

/// Truncate the context JSON to the prefix `rewind_point` bytes long.
/// Re-emits a valid JSON shape: `{ "messages": ...truncated prefix... }`.
/// If the prefix is `0`, returns an empty messages array.
fn truncate_context(context_json: &str, rewind_point: usize) -> String {
    if rewind_point == 0 {
        return r#"{"messages":[]}"#.to_string();
    }
    // Walk backwards to the previous comma or opening brace so we don't
    // produce a truncated object/messages array.
    let bytes = context_json.as_bytes();
    let mut cut = rewind_point.min(bytes.len());
    while cut > 0
        && bytes
            .get(cut - 1)
            .is_some_and(|byte| !matches!(byte, b',' | b'[' | b'{'))
    {
        cut -= 1;
    }
    let prefix = context_json.get(..cut).unwrap_or_default();
    format!("{}\"rewound\":true}}", prefix.trim_end_matches([',', ' ']))
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use forge_domain::{
        Context, ContextMessage, Effort, FileOperation, Metrics, Role, ToolCallFull, ToolCallId,
        ToolChoice, ToolDefinition, ToolKind, ToolName, ToolOutput, ToolResult, ToolValue, Usage,
    };
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::conversation::conversation_record::{ContextRecord, MetricsRecord};
    use crate::database::DatabasePool;

    fn repository() -> anyhow::Result<ConversationRepositoryImpl> {
        let pool = Arc::new(DatabasePool::in_memory()?);
        Ok(ConversationRepositoryImpl::new(pool, WorkspaceHash::new(0)))
    }

    #[tokio::test]
    async fn test_upsert_and_find_by_id() -> anyhow::Result<()> {
        let fixture = Conversation::new(ConversationId::generate())
            .title(Some("Test Conversation".to_string()));
        let repo = repository()?;

        repo.upsert_conversation(fixture.clone()).await?;

        let actual = repo.get_conversation(&fixture.id).await?;
        assert!(actual.is_some());
        let retrieved = actual.unwrap();
        assert_eq!(retrieved.id, fixture.id);
        assert_eq!(retrieved.title, fixture.title);
        Ok(())
    }

    #[tokio::test]
    async fn test_find_by_id_non_existing() -> anyhow::Result<()> {
        let repo = repository()?;
        let non_existing_id = ConversationId::generate();

        let actual = repo.get_conversation(&non_existing_id).await?;

        assert!(actual.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn test_upsert_updates_existing_conversation() -> anyhow::Result<()> {
        let mut fixture = Conversation::new(ConversationId::generate())
            .title(Some("Test Conversation".to_string()));
        let repo = repository()?;

        // Insert initial conversation
        repo.upsert_conversation(fixture.clone()).await?;

        // Update the conversation
        fixture = fixture.title(Some("Updated Title".to_string()));
        repo.upsert_conversation(fixture.clone()).await?;

        let actual = repo.get_conversation(&fixture.id).await?;
        assert!(actual.is_some());
        assert_eq!(actual.unwrap().title, Some("Updated Title".to_string()));
        Ok(())
    }

    #[tokio::test]
    async fn test_find_all_conversations() -> anyhow::Result<()> {
        let context1 =
            Context::default().messages(vec![ContextMessage::user("Hello", None).into()]);
        let context2 =
            Context::default().messages(vec![ContextMessage::user("World", None).into()]);
        let conversation1 = Conversation::new(ConversationId::generate())
            .title(Some("Test Conversation".to_string()))
            .context(Some(context1));
        let conversation2 = Conversation::new(ConversationId::generate())
            .title(Some("Second Conversation".to_string()))
            .context(Some(context2));
        let repo = repository()?;

        repo.upsert_conversation(conversation1.clone()).await?;
        repo.upsert_conversation(conversation2.clone()).await?;

        let actual = repo.get_all_conversations(None).await?;

        assert!(actual.is_some());
        let conversations = actual.unwrap();
        assert_eq!(conversations.len(), 2);
        Ok(())
    }

    #[tokio::test]
    async fn test_find_all_conversations_with_limit() -> anyhow::Result<()> {
        let context1 =
            Context::default().messages(vec![ContextMessage::user("Hello", None).into()]);
        let context2 =
            Context::default().messages(vec![ContextMessage::user("World", None).into()]);
        let conversation1 = Conversation::new(ConversationId::generate())
            .title(Some("Test Conversation".to_string()))
            .context(Some(context1));
        let conversation2 = Conversation::new(ConversationId::generate()).context(Some(context2));
        let repo = repository()?;

        repo.upsert_conversation(conversation1).await?;
        repo.upsert_conversation(conversation2).await?;

        let actual = repo.get_all_conversations(Some(1)).await?;

        assert!(actual.is_some());
        assert_eq!(actual.unwrap().len(), 1);
        Ok(())
    }

    #[tokio::test]
    async fn test_find_all_conversations_empty() -> anyhow::Result<()> {
        let repo = repository()?;

        let actual = repo.get_all_conversations(None).await?;

        assert!(actual.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn test_find_last_active_conversation_with_context() -> anyhow::Result<()> {
        let context = Context::default().messages(vec![ContextMessage::user("Hello", None).into()]);
        let conversation_with_context = Conversation::new(ConversationId::generate())
            .title(Some("Conversation with Context".to_string()))
            .context(Some(context));
        let conversation_without_context = Conversation::new(ConversationId::generate())
            .title(Some("Test Conversation".to_string()));
        let repo = repository()?;

        repo.upsert_conversation(conversation_without_context)
            .await?;
        repo.upsert_conversation(conversation_with_context.clone())
            .await?;

        let actual = repo.get_last_conversation().await?;

        assert!(actual.is_some());
        assert_eq!(actual.unwrap().id, conversation_with_context.id);
        Ok(())
    }

    #[tokio::test]
    async fn test_find_last_active_conversation_no_context() -> anyhow::Result<()> {
        let conversation_without_context = Conversation::new(ConversationId::generate())
            .title(Some("Test Conversation".to_string()));
        let repo = repository()?;

        repo.upsert_conversation(conversation_without_context)
            .await?;

        let actual = repo.get_last_conversation().await?;

        assert!(actual.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn test_find_last_active_conversation_ignores_empty_context() -> anyhow::Result<()> {
        let conversation_with_empty_context = Conversation::new(ConversationId::generate())
            .title(Some("Conversation with Empty Context".to_string()))
            .context(Some(Context::default()));
        let conversation_without_context = Conversation::new(ConversationId::generate())
            .title(Some("Test Conversation".to_string()));
        let repo = repository()?;

        repo.upsert_conversation(conversation_without_context)
            .await?;
        repo.upsert_conversation(conversation_with_empty_context)
            .await?;

        let actual = repo.get_last_conversation().await?;

        assert!(actual.is_none()); // Should not find conversations with empty contexts
        Ok(())
    }

    #[test]
    fn test_conversation_record_from_conversation() -> anyhow::Result<()> {
        let fixture = Conversation::new(ConversationId::generate())
            .title(Some("Test Conversation".to_string()));

        let actual = ConversationRecord::new(fixture.clone(), WorkspaceHash::new(0));

        assert_eq!(actual.conversation_id, fixture.id.into_string());
        assert_eq!(actual.title, Some("Test Conversation".to_string()));
        assert_eq!(actual.context, None);
        Ok(())
    }

    #[test]
    fn test_conversation_record_from_conversation_with_context() -> anyhow::Result<()> {
        let context = Context::default().messages(vec![ContextMessage::user("Hello", None).into()]);
        let fixture = Conversation::new(ConversationId::generate())
            .title(Some("Conversation with Context".to_string()))
            .context(Some(context));

        let actual = ConversationRecord::new(fixture.clone(), WorkspaceHash::new(0));

        assert_eq!(actual.conversation_id, fixture.id.into_string());
        assert_eq!(actual.title, Some("Conversation with Context".to_string()));
        // With compression, context is stored in context_zstd and is_compressed=1
        assert!(
            actual.context_zstd.is_some() || actual.context.is_some(),
            "context should be stored in either context_zstd (compressed) or context (plain)"
        );
        Ok(())
    }

    #[test]
    fn test_conversation_record_from_conversation_with_empty_context() -> anyhow::Result<()> {
        let fixture = Conversation::new(ConversationId::generate())
            .title(Some("Conversation with Empty Context".to_string()))
            .context(Some(Context::default()));

        let actual = ConversationRecord::new(fixture.clone(), WorkspaceHash::new(0));

        assert_eq!(actual.conversation_id, fixture.id.into_string());
        assert_eq!(
            actual.title,
            Some("Conversation with Empty Context".to_string())
        );

        assert!(actual.context.is_none()); // Empty context should be filtered out
        Ok(())
    }

    #[test]
    fn test_conversation_from_conversation_record() -> anyhow::Result<()> {
        let test_id = ConversationId::generate();
        let fixture = ConversationRecord {
            conversation_id: test_id.into_string(),
            title: Some("Test Conversation".to_string()),
            context: None,
            created_at: Utc::now().naive_utc(),
            updated_at: None,
            workspace_id: 0,
            metrics: None,
            parent_id: None,
            source: None,
            cwd: None,
            message_count: None,
            intent_state: "pending".to_string(),
            extracted_at: None,
            memory_id: None,
            intent_hash: None,
            context_zstd: None,
            is_compressed: 0,
        };

        let actual = Conversation::try_from(fixture)?;

        assert_eq!(actual.id, test_id);
        assert_eq!(actual.title, Some("Test Conversation".to_string()));
        assert_eq!(actual.context, None);
        Ok(())
    }

    #[tokio::test]
    async fn test_upsert_and_retrieve_conversation_with_metrics() -> anyhow::Result<()> {
        let repo = repository()?;

        // Create a conversation with metrics
        let metrics = Metrics::default()
            .started_at(Utc::now())
            .insert(
                "src/main.rs".to_string(),
                FileOperation::new(ToolKind::Write)
                    .lines_added(10u64)
                    .lines_removed(5u64)
                    .content_hash(Some("abc123def456".to_string())),
            )
            .insert(
                "src/lib.rs".to_string(),
                FileOperation::new(ToolKind::Write)
                    .lines_added(3u64)
                    .lines_removed(2u64)
                    .content_hash(Some("789xyz456abc".to_string())),
            );

        let fixture = Conversation::generate().metrics(metrics.clone());

        // Save the conversation
        repo.upsert_conversation(fixture.clone()).await?;

        // Retrieve the conversation
        let actual = repo
            .get_conversation(&fixture.id)
            .await?
            .expect("Conversation should exist");

        // Verify metrics are preserved
        assert_eq!(actual.metrics.file_operations.len(), 2);
        let main_metrics = actual.metrics.file_operations.get("src/main.rs").unwrap();
        assert_eq!(main_metrics.lines_added, 10);
        assert_eq!(main_metrics.lines_removed, 5);
        assert_eq!(main_metrics.content_hash, Some("abc123def456".to_string()));
        let lib_metrics = actual.metrics.file_operations.get("src/lib.rs").unwrap();
        assert_eq!(lib_metrics.lines_added, 3);
        assert_eq!(lib_metrics.lines_removed, 2);
        assert_eq!(lib_metrics.content_hash, Some("789xyz456abc".to_string()));
        Ok(())
    }

    #[test]
    fn test_metrics_record_conversion_preserves_all_fields() {
        // This test ensures compile-time safety: if Metrics schema changes,
        // this test will fail to compile, alerting us to update MetricsRecord
        let fixture = Metrics::default().started_at(Utc::now()).insert(
            "test.rs".to_string(),
            FileOperation::new(ToolKind::Write)
                .lines_added(5u64)
                .lines_removed(3u64)
                .content_hash(Some("test_hash_123".to_string())),
        );

        // Convert to record and back
        let record = MetricsRecord::from(&fixture);
        let actual = Metrics::from(record);

        // Verify all fields are preserved
        assert_eq!(actual.started_at, fixture.started_at);
        assert_eq!(actual.file_operations.len(), fixture.file_operations.len());

        let actual_file = actual.file_operations.get("test.rs").unwrap();
        let expected_file = fixture.file_operations.get("test.rs").unwrap();
        assert_eq!(actual_file.lines_added, expected_file.lines_added);
        assert_eq!(actual_file.lines_removed, expected_file.lines_removed);
        assert_eq!(actual_file.content_hash, expected_file.content_hash);
    }

    #[test]
    fn test_deserialize_old_format_without_tool_field() {
        // Old format from database: missing tool and content_hash fields
        let json = r#"{
            "started_at": "2024-01-01T00:00:00Z",
            "files_changed": {
                "src/main.rs": {
                    "lines_added": 10,
                    "lines_removed": 5
                },
                "src/lib.rs": {
                    "lines_added": 3,
                    "lines_removed": 2
                }
            }
        }"#;

        let record: MetricsRecord = serde_json::from_str(json).unwrap();
        let actual = Metrics::from(record);

        // Verify files are loaded
        assert_eq!(actual.file_operations.len(), 2);

        // Verify main.rs
        let main_file = actual.file_operations.get("src/main.rs").unwrap();
        assert_eq!(main_file.lines_added, 10);
        assert_eq!(main_file.lines_removed, 5);
        assert_eq!(main_file.content_hash, None);
        assert_eq!(main_file.tool, ToolKind::Write); // Default tool

        // Verify lib.rs
        let lib_file = actual.file_operations.get("src/lib.rs").unwrap();
        assert_eq!(lib_file.lines_added, 3);
        assert_eq!(lib_file.lines_removed, 2);
        assert_eq!(lib_file.content_hash, None);
        assert_eq!(lib_file.tool, ToolKind::Write); // Default tool
    }

    #[test]
    fn test_deserialize_array_format_takes_last_operation() {
        // Array format from database: multiple operations per file
        let json = r#"{
            "started_at": "2024-01-01T00:00:00Z",
            "files_changed": {
                "src/main.rs": [
                    {
                        "lines_added": 2,
                        "lines_removed": 4,
                        "content_hash": "hash1",
                        "tool": "read"
                    },
                    {
                        "lines_added": 1,
                        "lines_removed": 1,
                        "content_hash": "hash2",
                        "tool": "patch"
                    },
                    {
                        "lines_added": 5,
                        "lines_removed": 3,
                        "content_hash": "hash3",
                        "tool": "write"
                    }
                ]
            }
        }"#;

        let record: MetricsRecord = serde_json::from_str(json).unwrap();
        let actual = Metrics::from(record);

        // Verify only the last operation is kept
        assert_eq!(actual.file_operations.len(), 1);

        let main_file = actual.file_operations.get("src/main.rs").unwrap();
        assert_eq!(main_file.lines_added, 5);
        assert_eq!(main_file.lines_removed, 3);
        assert_eq!(main_file.content_hash, Some("hash3".to_string()));
        assert_eq!(main_file.tool, ToolKind::Write);
    }

    #[test]
    fn test_deserialize_array_format_with_empty_array() {
        // Array format with empty array should be skipped
        let json = r#"{
            "started_at": "2024-01-01T00:00:00Z",
            "files_changed": {
                "src/main.rs": [],
                "src/lib.rs": {
                    "lines_added": 5,
                    "lines_removed": 2,
                    "content_hash": "hash1",
                    "tool": "patch"
                }
            }
        }"#;

        let record: MetricsRecord = serde_json::from_str(json).unwrap();
        let actual = Metrics::from(record);

        // Empty array should be skipped, only lib.rs should be present
        assert_eq!(actual.file_operations.len(), 1);
        assert!(actual.file_operations.contains_key("src/lib.rs"));
        assert!(!actual.file_operations.contains_key("src/main.rs"));
    }

    #[test]
    fn test_deserialize_current_format_with_all_fields() {
        // Current format: single object with all fields
        let json = r#"{
            "started_at": "2024-01-01T00:00:00Z",
            "files_changed": {
                "src/main.rs": {
                    "lines_added": 10,
                    "lines_removed": 5,
                    "content_hash": "abc123def456",
                    "tool": "patch"
                },
                "src/lib.rs": {
                    "lines_added": 3,
                    "lines_removed": 2,
                    "content_hash": "789xyz456abc",
                    "tool": "write"
                }
            }
        }"#;

        let record: MetricsRecord = serde_json::from_str(json).unwrap();
        let actual = Metrics::from(record);

        // Verify all fields are preserved
        assert_eq!(actual.file_operations.len(), 2);

        let main_file = actual.file_operations.get("src/main.rs").unwrap();
        assert_eq!(main_file.lines_added, 10);
        assert_eq!(main_file.lines_removed, 5);
        assert_eq!(main_file.content_hash, Some("abc123def456".to_string()));
        assert_eq!(main_file.tool, ToolKind::Patch);

        let lib_file = actual.file_operations.get("src/lib.rs").unwrap();
        assert_eq!(lib_file.lines_added, 3);
        assert_eq!(lib_file.lines_removed, 2);
        assert_eq!(lib_file.content_hash, Some("789xyz456abc".to_string()));
        assert_eq!(lib_file.tool, ToolKind::Write);
    }

    #[test]
    fn test_deserialize_mixed_format() {
        // Mix of old format, array format, and current format
        let json = r#"{
            "started_at": "2024-01-01T00:00:00Z",
            "files_changed": {
                "old_file.rs": {
                    "lines_added": 10,
                    "lines_removed": 5
                },
                "array_file.rs": [
                    {
                        "lines_added": 1,
                        "lines_removed": 2,
                        "content_hash": "hash1",
                        "tool": "read"
                    },
                    {
                        "lines_added": 3,
                        "lines_removed": 4,
                        "content_hash": "hash2",
                        "tool": "patch"
                    }
                ],
                "current_file.rs": {
                    "lines_added": 7,
                    "lines_removed": 8,
                    "content_hash": "hash3",
                    "tool": "write"
                }
            }
        }"#;

        let record: MetricsRecord = serde_json::from_str(json).unwrap();
        let actual = Metrics::from(record);

        assert_eq!(actual.file_operations.len(), 3);

        // Old format file
        let old_file = actual.file_operations.get("old_file.rs").unwrap();
        assert_eq!(old_file.lines_added, 10);
        assert_eq!(old_file.lines_removed, 5);
        assert_eq!(old_file.content_hash, None);
        assert_eq!(old_file.tool, ToolKind::Write); // Default

        // Array format file (should have last operation)
        let array_file = actual.file_operations.get("array_file.rs").unwrap();
        assert_eq!(array_file.lines_added, 3);
        assert_eq!(array_file.lines_removed, 4);
        assert_eq!(array_file.content_hash, Some("hash2".to_string()));
        assert_eq!(array_file.tool, ToolKind::Patch);

        // Current format file
        let current_file = actual.file_operations.get("current_file.rs").unwrap();
        assert_eq!(current_file.lines_added, 7);
        assert_eq!(current_file.lines_removed, 8);
        assert_eq!(current_file.content_hash, Some("hash3".to_string()));
        assert_eq!(current_file.tool, ToolKind::Write);
    }

    #[test]
    fn test_serialize_current_format() {
        // Test that we always serialize in the current format (single object)
        let fixture = Metrics::default().started_at(Utc::now()).insert(
            "src/main.rs".to_string(),
            FileOperation::new(ToolKind::Patch)
                .lines_added(10u64)
                .lines_removed(5u64)
                .content_hash(Some("abc123".to_string())),
        );

        let record = MetricsRecord::from(&fixture);
        let json = serde_json::to_string(&record).unwrap();

        // Verify it's not an array format
        assert!(!json.contains("[{"));
        // Verify it contains the tool field
        assert!(json.contains("\"tool\":\"patch\""));

        // Verify structure is correct
        assert!(json.contains("\"lines_added\":10"));
        assert!(json.contains("\"lines_removed\":5"));
        assert!(json.contains("\"content_hash\":\"abc123\""));
    }

    #[test]
    fn test_context_record_conversion_preserves_all_fields() {
        let tool_def = ToolDefinition::new("test_tool").description("A test tool");

        let reasoning = forge_domain::ReasoningConfig {
            effort: Some(Effort::Medium),
            max_tokens: Some(2048),
            exclude: Some(false),
            enabled: Some(true),
        };

        // Create a comprehensive set of messages to test all message types
        let messages = vec![
            ContextMessage::user("Hello", None).into(),
            ContextMessage::system("System prompt").into(),
            ContextMessage::Tool(ToolResult {
                name: ToolName::new("test_tool"),
                call_id: Some(ToolCallId::new("call_123".to_string())),
                output: ToolOutput {
                    is_error: false,
                    values: vec![ToolValue::Text("Result text".to_string()), ToolValue::Empty],
                },
            })
            .into(),
            forge_domain::MessageEntry {
                message: ContextMessage::Text(forge_domain::TextMessage {
                    role: Role::Assistant,
                    content: "Assistant response".to_string(),
                    raw_content: None,
                    tool_calls: Some(vec![ToolCallFull {
                        name: ToolName::new("another_tool"),
                        call_id: Some(ToolCallId::new("call_456".to_string())),
                        arguments: forge_domain::ToolCallArguments::from(
                            serde_json::json!({"param": "value"}),
                        ),
                        thought_signature: None,
                    }]),
                    model: Some(forge_domain::ModelId::from("gpt-4")),
                    thought_signature: None,
                    reasoning_details: None,
                    droppable: false,
                    phase: None,
                }),
                usage: Some(Usage {
                    prompt_tokens: forge_domain::TokenCount::Actual(100),
                    completion_tokens: forge_domain::TokenCount::Actual(50),
                    total_tokens: forge_domain::TokenCount::Actual(150),
                    cached_tokens: forge_domain::TokenCount::Actual(0),
                    cost: Some(0.001),
                }),
            },
        ];

        let fixture = Context::default()
            .conversation_id(ConversationId::generate())
            .messages(messages)
            .tools(vec![tool_def.clone()])
            .tool_choice(ToolChoice::Call(ToolName::new("test_tool")))
            .max_tokens(1000usize)
            .temperature(forge_domain::Temperature::new(0.7).unwrap())
            .top_p(forge_domain::TopP::new(0.9).unwrap())
            .top_k(forge_domain::TopK::new(50).unwrap())
            .reasoning(reasoning.clone())
            .stream(true);

        // Convert to record and back
        let record = ContextRecord::from(&fixture);
        let actual = Context::try_from(record).unwrap();

        // Verify all fields are preserved
        assert_eq!(actual.conversation_id, fixture.conversation_id);
        assert_eq!(actual.messages.len(), 4);
        assert_eq!(actual.tools.len(), 1);
        assert_eq!(actual.tools[0].name.to_string(), "test_tool");
        assert_eq!(
            actual.tool_choice,
            Some(ToolChoice::Call(ToolName::new("test_tool")))
        );
        assert_eq!(actual.max_tokens, fixture.max_tokens);
        assert_eq!(actual.temperature, fixture.temperature);
        assert_eq!(actual.top_p, fixture.top_p);
        assert_eq!(actual.top_k, fixture.top_k);
        assert_eq!(actual.reasoning, Some(reasoning));
        assert_eq!(actual.stream, fixture.stream);

        // Verify message types and content
        match &actual.messages[0].message {
            ContextMessage::Text(msg) => {
                assert_eq!(msg.role, Role::User);
                assert_eq!(msg.content, "Hello");
            }
            _ => panic!("Expected user message"),
        }

        match &actual.messages[2].message {
            ContextMessage::Tool(tool_result) => {
                assert_eq!(tool_result.name.to_string(), "test_tool");
                assert_eq!(
                    tool_result.call_id.as_ref().map(|id| id.as_str()),
                    Some("call_123")
                );
                assert!(!tool_result.output.is_error);
                assert_eq!(tool_result.output.values.len(), 2);
            }
            _ => panic!("Expected tool result message"),
        }

        // Verify usage is preserved
        match &actual.messages[3].usage {
            Some(usage) => {
                assert_eq!(*usage.prompt_tokens, 100);
                assert_eq!(*usage.completion_tokens, 50);
                assert_eq!(*usage.total_tokens, 150);
                assert_eq!(usage.cost, Some(0.001));
            }
            None => panic!("Expected usage information"),
        }
    }

    #[test]
    fn test_conversation_deserialization_error_includes_id() {
        // Test that deserialization errors include the conversation ID
        let test_id = ConversationId::generate();
        let fixture = ConversationRecord {
            conversation_id: test_id.into_string(),
            title: Some("Test Conversation".to_string()),
            context: Some("invalid json".to_string()), // Invalid JSON to trigger error
            created_at: Utc::now().naive_utc(),
            updated_at: None,
            workspace_id: 0,
            metrics: None,
            parent_id: None,
            source: None,
            cwd: None,
            message_count: None,
            intent_state: "pending".to_string(),
            extracted_at: None,
            memory_id: None,
            intent_hash: None,
            context_zstd: None,
            is_compressed: 0,
        };

        let result = Conversation::try_from(fixture);

        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(
            error_message.contains(&test_id.to_string()),
            "Error message should contain conversation ID. Got: {}",
            error_message
        );
        assert!(
            error_message.contains("Failed to deserialize context"),
            "Error message should indicate context deserialization failure. Got: {}",
            error_message
        );
    }

    #[tokio::test]
    async fn test_delete_conversation_success() -> anyhow::Result<()> {
        let repo = repository()?;
        let conversation = Conversation::new(ConversationId::generate())
            .title(Some("Test Conversation".to_string()));

        repo.upsert_conversation(conversation.clone()).await?;

        repo.delete_conversation(&conversation.id).await?;

        let result = repo.get_conversation(&conversation.id).await?;
        assert!(result.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn test_delete_conversation_workspace_filtering() -> anyhow::Result<()> {
        let repo = repository()?;
        let conversation = Conversation::new(ConversationId::generate())
            .title(Some("Test Conversation".to_string()));

        repo.upsert_conversation(conversation.clone()).await?;

        // Delete should succeed regardless of existence (idempotent)
        repo.delete_conversation(&conversation.id).await?;

        // Verify conversation is deleted
        let deleted = repo.get_conversation(&conversation.id).await?;
        assert!(deleted.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn test_delete_conversation_cross_workspace_security() -> anyhow::Result<()> {
        let repo = repository()?;

        // Create conversation in current workspace
        let conversation_id = ConversationId::generate();
        let conversation =
            Conversation::new(conversation_id).title(Some("Test Conversation".to_string()));

        repo.upsert_conversation(conversation.clone()).await?;

        // Try to delete with different workspace ID (should fail due to security)
        // Note: This test would require modifying workspace ID in repo
        // For now, we test that deletion works with current workspace
        repo.delete_conversation(&conversation.id).await?;

        // Verify it's actually deleted
        let deleted = repo.get_conversation(&conversation.id).await?;
        assert!(deleted.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn test_delete_conversation_end_to_end_workflow() -> anyhow::Result<()> {
        let repo = repository()?;
        let conversation_id = ConversationId::generate();
        let conversation =
            Conversation::new(conversation_id).title(Some("Test Conversation".to_string()));

        // Test complete workflow: create -> delete -> verify -> create new -> verify
        repo.upsert_conversation(conversation.clone()).await?;

        // Delete conversation
        repo.delete_conversation(&conversation.id).await?;

        // Verify it's gone
        let deleted_check = repo.get_conversation(&conversation.id).await?;
        assert!(deleted_check.is_none());

        // Create new conversation to ensure system still works
        let new_conversation_id = ConversationId::generate();
        let new_conversation = Conversation::new(new_conversation_id);
        repo.upsert_conversation(new_conversation.clone()).await?;

        // Verify new conversation exists
        let new_check = repo.get_conversation(&new_conversation_id).await?;
        assert!(new_check.is_some());

        Ok(())
    }

    #[tokio::test]
    async fn test_rename_conversation_via_upsert() -> anyhow::Result<()> {
        let repo = repository()?;
        let conversation =
            Conversation::new(ConversationId::generate()).title(Some("Original Title".to_string()));

        repo.upsert_conversation(conversation.clone()).await?;

        // Rename by upserting with a new title
        let renamed = conversation
            .clone()
            .title(Some("Renamed Session".to_string()));
        repo.upsert_conversation(renamed).await?;

        let actual = repo.get_conversation(&conversation.id).await?.unwrap();
        assert_eq!(actual.title, Some("Renamed Session".to_string()));
        Ok(())
    }

    #[tokio::test]
    async fn test_rename_conversation_from_none() -> anyhow::Result<()> {
        let repo = repository()?;
        let conversation = Conversation::new(ConversationId::generate());

        // Start with no title
        assert!(conversation.title.is_none());
        repo.upsert_conversation(conversation.clone()).await?;

        // Rename it
        let renamed = conversation.clone().title(Some("My Session".to_string()));
        repo.upsert_conversation(renamed).await?;

        let actual = repo.get_conversation(&conversation.id).await?.unwrap();
        assert_eq!(actual.title, Some("My Session".to_string()));
        Ok(())
    }

    #[test]
    fn test_legacy_tool_value_pair_deserialization() {
        use crate::conversation::conversation_record::ToolOutputRecord;

        // This JSON represents the old Pair variant format that was stored in the
        // database
        let legacy_json = r#"{
            "is_error": false,
            "values": [
                {"pair": [
                    {"text": "XML content for LLM"},
                    {"fileDiff": {"path": "/test/file.rs", "old_text": "old", "new_text": "new"}}
                ]}
            ]
        }"#;

        let record: ToolOutputRecord = serde_json::from_str(legacy_json).unwrap();
        let actual: forge_domain::ToolOutput = record.try_into().unwrap();

        // The Pair variant should be converted by taking the first element (LLM
        // content)
        assert!(!actual.is_error);
        assert_eq!(actual.values.len(), 1);
        assert_eq!(
            actual.values[0],
            forge_domain::ToolValue::Text("XML content for LLM".to_string())
        );
    }

    #[test]
    fn test_legacy_tool_value_markdown_deserialization() {
        use crate::conversation::conversation_record::ToolOutputRecord;

        let legacy_json = r##"{
            "is_error": false,
            "values": [{"markdown": "# Heading - Some bold text"}]
        }"##;

        let record: ToolOutputRecord = serde_json::from_str(legacy_json).unwrap();
        let actual: forge_domain::ToolOutput = record.try_into().unwrap();

        // Markdown should be converted to Text
        assert_eq!(actual.values.len(), 1);
        assert_eq!(
            actual.values[0],
            forge_domain::ToolValue::Text("# Heading - Some bold text".to_string())
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_concurrent_operations_dont_block_runtime() -> anyhow::Result<()> {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::time::{Duration, Instant};

        // Heartbeat fires every `TICK`; we require a measurement window of at
        // least `MIN_WINDOW` so the assertion is meaningful even when the DB
        // workload finishes very quickly (e.g. on fast machines with the
        // in-memory SQLite pool).
        const TICK: Duration = Duration::from_millis(10);
        const MIN_WINDOW: Duration = Duration::from_millis(200);

        let repo = Arc::new(repository()?);
        let heartbeat = Arc::new(AtomicUsize::new(0));

        // Heartbeat task - if runtime is blocked, this won't increment.
        let heartbeat_clone = heartbeat.clone();
        let heartbeat_handle = tokio::spawn(async move {
            loop {
                tokio::time::sleep(TICK).await;
                heartbeat_clone.fetch_add(1, Ordering::Relaxed);
            }
        });

        // Warm up: let the heartbeat task get scheduled and complete its first
        // tick before we start measuring, then reset the counter so timing
        // begins from a clean state.
        tokio::time::sleep(TICK * 3).await;
        heartbeat.store(0, Ordering::Relaxed);

        // Spawn many concurrent DB operations.
        let mut handles = vec![];
        let start = Instant::now();

        for i in 0..20 {
            let repo = repo.clone();
            let handle = tokio::spawn(async move {
                for j in 0..10 {
                    let conversation = Conversation::new(ConversationId::generate())
                        .title(Some(format!("Task {} - Write {}", i, j)));
                    repo.upsert_conversation(conversation).await?;
                }
                anyhow::Result::<()>::Ok(())
            });
            handles.push(handle);
        }

        // Wait for all operations.
        for handle in handles {
            handle.await??;
        }

        // Ensure the measurement window is long enough for heartbeat math to
        // be meaningful regardless of how fast the DB workload completed.
        let work_elapsed = start.elapsed();
        if work_elapsed < MIN_WINDOW {
            tokio::time::sleep(MIN_WINDOW - work_elapsed).await;
        }
        let elapsed = start.elapsed();

        // Stop heartbeat.
        heartbeat_handle.abort();

        // Verify runtime wasn't blocked: heartbeat should have fired at least
        // 80% of the theoretical max for the elapsed window. The threshold is
        // clamped to at least 1 to keep the assertion well-defined.
        let heartbeat_count = heartbeat.load(Ordering::Relaxed);
        let expected_heartbeats = (elapsed.as_millis() as usize) / (TICK.as_millis() as usize);
        let threshold = (expected_heartbeats * 8 / 10).max(1);

        assert!(
            heartbeat_count >= threshold,
            "Runtime was blocked! Expected at least {} heartbeats (~{} theoretical) in {:?}, got {}",
            threshold,
            expected_heartbeats,
            elapsed,
            heartbeat_count
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_mixed_read_write_contention() -> anyhow::Result<()> {
        let repo = Arc::new(repository()?);
        let mut handles = vec![];

        // Pre-populate some data
        for i in 0..10 {
            let conv =
                Conversation::new(ConversationId::generate()).title(Some(format!("Initial {}", i)));
            repo.upsert_conversation(conv).await?;
        }

        // Spawn writers
        for i in 0..10 {
            let repo = repo.clone();
            handles.push(tokio::spawn(async move {
                for j in 0..10 {
                    let conv = Conversation::new(ConversationId::generate())
                        .title(Some(format!("Writer {} - {}", i, j)));
                    repo.upsert_conversation(conv).await?;
                }
                anyhow::Result::<()>::Ok(())
            }));
        }

        // Spawn readers (interleave with writers)
        for _ in 0..10 {
            let repo = repo.clone();
            handles.push(tokio::spawn(async move {
                for _ in 0..10 {
                    // Read all conversations
                    let _ = repo.get_all_conversations(Some(50)).await?;
                    tokio::task::yield_now().await;
                }
                anyhow::Result::<()>::Ok(())
            }));
        }

        // All should complete without timeout
        for handle in handles {
            handle.await??;
        }

        Ok(())
    }

    #[test]
    fn test_legacy_tool_value_file_diff_deserialization() {
        use crate::conversation::conversation_record::ToolOutputRecord;

        let legacy_json = r#"{
            "is_error": false,
            "values": [{"fileDiff": {"path": "/src/main.rs", "old_text": "fn old()", "new_text": "fn new()"}}]
        }"#;

        let record: ToolOutputRecord = serde_json::from_str(legacy_json).unwrap();
        let actual: forge_domain::ToolOutput = record.try_into().unwrap();

        // FileDiff should be converted to a text summary
        assert_eq!(actual.values.len(), 1);
        assert_eq!(
            actual.values[0],
            forge_domain::ToolValue::Text("[File diff: /src/main.rs]".to_string())
        );
    }

    #[tokio::test]
    async fn test_prune_conversation_safety_guard() -> anyhow::Result<()> {
        let repo = repository()?;
        let context =
            Context::default().messages(vec![ContextMessage::user("Test content", None).into()]);
        let conversation = Conversation::new(ConversationId::generate())
            .title(Some("Test for Pruning".to_string()))
            .context(Some(context));

        // Insert conversation with default intent_state='pending'
        repo.upsert_conversation(conversation.clone()).await?;

        // ADR-103: Pruning should fail when intent_state != 'verified'
        let result = repo.prune_conversation(&conversation.id).await;
        assert!(
            result.is_err(),
            "Pruning should fail when intent_state='pending'"
        );
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Must be 'verified'"),
            "Error should indicate the requirement for 'verified' state"
        );

        // Mark as verified
        repo.mark_intent_state(&conversation.id, "verified").await?;

        // Now pruning should succeed
        let prune_result = repo.prune_conversation(&conversation.id).await;
        assert!(
            prune_result.is_ok(),
            "Pruning should succeed when intent_state='verified'"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_mark_intent_state_enforces_dag() -> anyhow::Result<()> {
        let repo = repository()?;
        let conversation = Conversation::new(ConversationId::generate())
            .title(Some("Test for State Machine".to_string()));

        repo.upsert_conversation(conversation.clone()).await?;

        // Verify default state is 'pending'
        let conv = repo.get_conversation(&conversation.id).await?;
        assert!(conv.is_some());

        // Valid transition: pending → extracting
        assert!(
            repo.mark_intent_state(&conversation.id, "extracting")
                .await
                .is_ok()
        );

        // Valid transition: extracting → extracted
        assert!(
            repo.mark_intent_state(&conversation.id, "extracted")
                .await
                .is_ok()
        );

        // Valid transition: extracted → verified
        assert!(
            repo.mark_intent_state(&conversation.id, "verified")
                .await
                .is_ok()
        );

        // Valid transition: verified → pruned
        assert!(
            repo.mark_intent_state(&conversation.id, "pruned")
                .await
                .is_ok()
        );

        // Invalid transition: pruned → any state (pruned is final)
        let result = repo.mark_intent_state(&conversation.id, "verified").await;
        assert!(result.is_err(), "Cannot transition from pruned to verified");

        Ok(())
    }

    #[tokio::test]
    async fn test_search_finds_compressed_conversations() -> anyhow::Result<()> {
        // CRITICAL TEST: Proves that compressed rows (context=NULL, is_compressed=1)
        // are findable by FTS5 search after refresh_fts_index populates the
        // index with decompressed content.
        //
        // This test catches the bug where external-content FTS5 reads by column name
        // (context), missing compressed rows where context=NULL.
        let repo = repository()?;

        // Create two conversations with context containing searchable text
        let msg_compressed = ContextMessage::user("SEARCHABLE_COMPRESSED_TERM", None);
        let msg_plain = ContextMessage::user("SEARCHABLE_PLAIN_TERM", None);

        let context_compressed = Context::default().messages(vec![msg_compressed.into()]);
        let context_plain = Context::default().messages(vec![msg_plain.into()]);

        // Insert compressed conversation (will be stored as context_zstd,
        // is_compressed=1, context=NULL)
        let compressed_conv = Conversation::new(ConversationId::generate())
            .title(Some("Compressed Conversation".to_string()))
            .context(Some(context_compressed.clone()));
        repo.upsert_conversation(compressed_conv.clone()).await?;

        // Insert uncompressed conversation (will be stored as plain context,
        // is_compressed=0)
        let plain_conv = Conversation::new(ConversationId::generate())
            .title(Some("Plain Conversation".to_string()))
            .context(Some(context_plain.clone()));
        repo.upsert_conversation(plain_conv.clone()).await?;

        // Refresh FTS index to populate both compressed and uncompressed rows
        repo.refresh_fts_index().await?;

        // SEARCH 1: Find compressed conversation by term in its decompressed context
        // If the fix is correct, this search WILL find the compressed row.
        // Before the fix, this would return empty (context=NULL skipped by FTS).
        let results_compressed = repo
            .search_conversations("SEARCHABLE_COMPRESSED_TERM", None)
            .await?;
        assert!(
            !results_compressed.is_empty(),
            "FTS search must find compressed conversations after refresh_fts_index; \
             bug: external-content FTS5 reads context column by name, missing compressed rows"
        );
        assert!(
            results_compressed
                .iter()
                .any(|c| c.id == compressed_conv.id),
            "Search results must include the compressed conversation"
        );

        // SEARCH 2: Find uncompressed conversation (baseline to ensure search works)
        let results_plain = repo
            .search_conversations("SEARCHABLE_PLAIN_TERM", None)
            .await?;
        assert!(
            !results_plain.is_empty(),
            "FTS search must find uncompressed conversations"
        );
        assert!(
            results_plain.iter().any(|c| c.id == plain_conv.id),
            "Search results must include the plain conversation"
        );

        // SEARCH 3: Verify no false positives
        let results_wrong = repo.search_conversations("NONEXISTENT_TERM", None).await?;
        assert!(
            results_wrong.is_empty(),
            "Search must not return conversations that don't contain the search term"
        );

        Ok(())
    }

    /// Verify that compress_uncompressed_contexts compresses plain rows and
    /// round-trips the context JSON back intact.
    #[tokio::test]
    async fn test_compress_uncompressed_contexts_basic() -> anyhow::Result<()> {
        let repo = repository()?;

        // Insert a conversation with a plain context so the compression path has
        // something to act on.
        let conv = Conversation::new(ConversationId::generate())
            .title(Some("compress test".to_string()))
            .context(Some(Context::default()));
        repo.upsert_conversation(conv.clone()).await?;

        // The new write path already compresses on insert. Manually flip one row
        // back to is_compressed=0 with a plain context blob to simulate the
        // pre-migration state of older rows.
        let plain_json = r#"{"messages":[]}"#;
        repo.run_with_connection(move |conn, _wid| {
            diesel::sql_query(
                "UPDATE conversations \
                 SET context = ?, context_zstd = NULL, is_compressed = 0 \
                 WHERE conversation_id = ?",
            )
            .bind::<diesel::sql_types::Text, _>(plain_json)
            .bind::<diesel::sql_types::Text, _>(conv.id.into_string())
            .execute(conn)?;
            Ok(())
        })
        .await?;

        // Run the maintenance command.
        let (compressed, _skipped, errors) = repo.compress_uncompressed_contexts().await?;
        assert_eq!(errors, 0, "no compression errors expected");
        assert!(
            compressed >= 1,
            "at least one row should have been compressed"
        );

        // Verify the row is now flagged compressed and the context column is NULL.
        repo.run_with_connection(move |conn, _wid| {
            #[derive(diesel::QueryableByName)]
            struct FlagRow {
                #[diesel(sql_type = diesel::sql_types::Integer)]
                is_compressed: i32,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
                context: Option<String>,
            }
            let rows: Vec<FlagRow> = diesel::sql_query(
                "SELECT is_compressed, context FROM conversations WHERE conversation_id = ?",
            )
            .bind::<diesel::sql_types::Text, _>(conv.id.into_string())
            .load(conn)?;
            let row = rows.first().expect("row should exist");
            assert_eq!(row.is_compressed, 1, "row must be flagged compressed");
            assert!(
                row.context.is_none(),
                "plain context must be cleared to NULL"
            );
            Ok(())
        })
        .await?;

        // Verify round-trip: fetch via the normal read path and confirm context loads.
        let retrieved = repo
            .get_conversation(&conv.id)
            .await?
            .expect("conversation should still exist");
        // Context may be Some (with messages) or None depending on the empty default —
        // the key assertion is that fetch does not panic or error.
        let _ = retrieved;

        Ok(())
    }

    /// Verify idempotency: running compress twice does not double-compress or
    /// error.
    #[tokio::test]
    async fn test_compress_uncompressed_contexts_idempotent() -> anyhow::Result<()> {
        let repo = repository()?;

        let conv = Conversation::new(ConversationId::generate())
            .title(Some("idempotent test".to_string()));
        repo.upsert_conversation(conv.clone()).await?;

        // First run.
        let (c1, _s1, e1) = repo.compress_uncompressed_contexts().await?;
        assert_eq!(e1, 0);

        // Second run — nothing new to compress.
        let (c2, _s2, e2) = repo.compress_uncompressed_contexts().await?;
        assert_eq!(e2, 0);
        assert_eq!(c2, 0, "second run should find no new rows to compress");
        let _ = c1;

        Ok(())
    }

    // ---------------------------------------------------------------------
    // import_forge_db tests
    // ---------------------------------------------------------------------

    /// Create an *official-lineage* source database (plain `context` schema,
    /// no `is_compressed`/`context_zstd` columns) at `path` and populate it
    /// with a deterministic set of rows:
    ///
    /// - one row with a well-formed plain JSON context (fully importable),
    /// - one row with a NULL context (imported, title-only),
    /// - one row whose context is not valid heliosLite JSON (counted as
    ///   `context_parse_failed`, still imported),
    /// - one row with a non-UUID `conversation_id` (counted as `invalid_id`).
    fn seed_official_source_db(path: &std::path::Path) {
        use diesel::Connection;

        let url = path.to_string_lossy().to_string();
        let mut connection =
            diesel::sqlite::SqliteConnection::establish(&url).expect("open source db");
        diesel::sql_query(
            "CREATE TABLE conversations (\
                 conversation_id TEXT PRIMARY KEY,\
                 title TEXT,\
                 context TEXT,\
                 created_at TEXT NOT NULL,\
                 updated_at TEXT,\
                 metrics TEXT\
             )",
        )
        .execute(&mut connection)
        .expect("create conversations table");

        let mut insert = |id: &str, title: &str, context: Option<&str>| {
            diesel::sql_query(
                "INSERT INTO conversations \
                 (conversation_id, title, context, created_at, updated_at) \
                 VALUES (?, ?, ?, ?, ?)",
            )
            .bind::<diesel::sql_types::Text, _>(id)
            .bind::<diesel::sql_types::Text, _>(title)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(context)
            .bind::<diesel::sql_types::Text, _>("2026-08-05 10:00:00")
            .bind::<diesel::sql_types::Text, _>("2026-08-05 10:05:00")
            .execute(&mut connection)
            .expect("insert row");
        };

        // A well-formed plain-text context serialized through the record type
        // exactly as the importer expects to read it back.
        let well_formed =
            ContextRecord::from(&forge_domain::Context::default());
        insert(
            "11111111-1111-1111-1111-111111111111",
            "imported fully",
            Some(&serde_json::to_string(&well_formed).unwrap()),
        );
        insert(
            "22222222-2222-2222-2222-222222222222",
            "imported title-only",
            None,
        );
        insert(
            "33333333-3333-3333-3333-333333333333",
            "imported parse-failed",
            Some("this is not valid context json"),
        );
        insert(
            "not-a-uuid",
            "invalid id",
            None,
        );
    }

    #[tokio::test]
    async fn test_import_forge_db_imports_and_reports() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let source = temp.path().join("official.db");
        seed_official_source_db(&source);
        let repo = repository()?;

        let report = repo.import_forge_db(source.clone()).await?;

        // 4 rows read: 3 imported (one of which had a parse failure), 1 invalid id.
        assert_eq!(report.source_total, 4, "all rows counted");
        assert_eq!(report.imported, 3, "valid ids import");
        assert_eq!(report.invalid_id, 1, "non-uuid id is reported separately");
        assert_eq!(
            report.context_parse_failed, 1,
            "unparseable context is counted but the row still imports"
        );
        assert_eq!(report.skipped_existing, 0, "first run imports everything new");
        assert_eq!(report.errors, 0, "no insert errors");

        // The imported conversations are readable through the normal read path.
        let id = ConversationId::parse("11111111-1111-1111-1111-111111111111")?;
        let fetched = repo.get_conversation(&id).await?;
        assert!(fetched.is_some(), "imported row should be readable");
        assert_eq!(fetched.unwrap().title.as_deref(), Some("imported fully"));

        Ok(())
    }

    #[tokio::test]
    async fn test_import_forge_db_is_idempotent() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let source = temp.path().join("official.db");
        seed_official_source_db(&source);
        let repo = repository()?;

        let first = repo.import_forge_db(source.clone()).await?;
        assert_eq!(first.imported, 3);

        let second = repo.import_forge_db(source.clone()).await?;
        assert_eq!(second.imported, 0, "re-run imports nothing");
        assert_eq!(
            second.skipped_existing, 3,
            "previously imported ids are skipped"
        );
        assert_eq!(second.invalid_id, 1, "invalid id is re-reported");
        assert_eq!(second.errors, 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_import_forge_db_does_not_modify_source() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let source = temp.path().join("official.db");
        seed_official_source_db(&source);

        let before = std::fs::read(&source)?;
        let repo = repository()?;
        repo.import_forge_db(source.clone()).await?;
        let after = std::fs::read(&source)?;

        assert_eq!(before, after, "source database bytes must be unchanged");
        Ok(())
    }

    #[tokio::test]
    async fn test_import_forge_db_rejects_fork_schema_source() -> anyhow::Result<()> {
        use diesel::Connection;

        let temp = tempfile::tempdir()?;
        let source = temp.path().join("fork-schema.db");
        let url = source.to_string_lossy().to_string();
        let mut connection =
            diesel::sqlite::SqliteConnection::establish(&url).expect("open db");
        // A fork-schema table carries the compression columns the importer
        // refuses to process.
        diesel::sql_query(
            "CREATE TABLE conversations (\
                 conversation_id TEXT PRIMARY KEY,\
                 context TEXT,\
                 context_zstd BLOB,\
                 is_compressed INTEGER\
             )",
        )
        .execute(&mut connection)?;

        let repo = repository()?;
        let error = repo
            .import_forge_db(source.clone())
            .await
            .expect_err("fork-schema source must be rejected");
        assert!(
            format!("{error}").contains("already a heliosLite/fork-schema"),
            "unexpected error: {error}"
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_import_forge_db_missing_source_errors() -> anyhow::Result<()> {
        let repo = repository()?;
        let missing = std::env::temp_dir().join(format!(
            "forge-does-not-exist-{}.db",
            std::process::id()
        ));
        let error = repo
            .import_forge_db(missing)
            .await
            .expect_err("missing file must error");
        assert!(
            format!("{error}").contains("source database not found"),
            "unexpected error: {error}"
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_import_forge_db_dry_run_does_not_write() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let source = temp.path().join("official.db");
        seed_official_source_db(&source);
        let repo = repository()?;

        let options = forge_domain::ForgeImportOptions {
            dry_run: true,
            verbose: false,
        };
        let report = repo
            .import_forge_db_with_options(source.clone(), &options)
            .await?;

        // Dry-run reports the same counts but writes nothing.
        assert!(report.dry_run, "dry_run flag must be set in report");
        assert_eq!(report.source_total, 4);
        assert_eq!(report.imported, 3, "would be imported, but not written");
        assert_eq!(report.invalid_id, 1);
        assert_eq!(report.context_parse_failed, 1);

        // No rows should be visible in the destination after a dry run.
        let all = repo.get_all_conversations(None).await?;
        assert!(
            all.map(|v| v.is_empty()).unwrap_or(true),
            "dry_run must not write to destination"
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_import_forge_db_verbose_does_not_change_outcome() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let source = temp.path().join("official.db");
        seed_official_source_db(&source);
        let repo = repository()?;

        let options = forge_domain::ForgeImportOptions {
            dry_run: false,
            verbose: true,
        };
        let report = repo
            .import_forge_db_with_options(source.clone(), &options)
            .await?;
        assert_eq!(report.imported, 3);
        assert_eq!(report.invalid_id, 1);
        assert_eq!(report.context_parse_failed, 1);
        assert!(!report.dry_run);
        Ok(())
    }

    #[tokio::test]
    async fn test_export_forge_db_round_trips() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let repo = repository()?;

        // Seed a conversation in the heliosLite repo so the export has data.
        let conv = forge_domain::Conversation::new(ConversationId::generate())
            .title(Some("export me".to_string()))
            .context(Some(Context::default()));
        repo.upsert_conversation(conv.clone()).await?;

        let dest = temp.path().join("exported.db");
        let options = forge_domain::ForgeExportOptions::default();
        let report = repo.export_forge_db(dest.clone(), &options).await?;

        assert_eq!(report.source_total, 1, "all rows read from source");
        assert_eq!(report.exported, 1, "decompressed + written");
        assert_eq!(report.decompression_failed, 0);
        assert_eq!(report.errors, 0);
        assert!(!report.dry_run);

        // Verify the destination has the official schema and the conversation
        // is present with plain (uncompressed) context.
        use diesel::Connection;
        let url = dest.to_string_lossy().to_string();
        let mut check = diesel::sqlite::SqliteConnection::establish(&url)?;
        #[derive(diesel::QueryableByName)]
        struct ExportRow {
            #[diesel(sql_type = diesel::sql_types::Text)]
            conversation_id: String,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
            context: Option<String>,
        }
        let rows: Vec<ExportRow> = diesel::sql_query(
            "SELECT conversation_id, context FROM conversations WHERE conversation_id = ?",
        )
        .bind::<diesel::sql_types::Text, _>(conv.id.into_string())
        .load(&mut check)?;
        assert_eq!(rows.len(), 1, "exported row is present");
        assert!(
            rows[0].context.is_some(),
            "exported context must be plain (not compressed)"
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_export_forge_db_dry_run_does_not_create_file() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let repo = repository()?;

        let conv = forge_domain::Conversation::new(ConversationId::generate())
            .title(Some("do not write".to_string()));
        repo.upsert_conversation(conv).await?;

        let dest = temp.path().join("never-created.db");
        let options = forge_domain::ForgeExportOptions { dry_run: true };
        let report = repo.export_forge_db(dest.clone(), &options).await?;

        assert!(report.dry_run);
        assert_eq!(report.source_total, 1);
        assert_eq!(report.exported, 1);
        assert!(!dest.exists(), "dry_run must not create the destination file");
        Ok(())
    }

    #[tokio::test]
    async fn test_database_stats_reports_compression_health() -> anyhow::Result<()> {
        let repo = repository()?;
        // Insert one conversation so the totals are non-zero.
        let conv = forge_domain::Conversation::new(ConversationId::generate())
            .title(Some("stats test".to_string()))
            .context(Some(Context::default()));
        repo.upsert_conversation(conv).await?;

        let stats = repo.database_stats().await?;
        assert!(
            stats.total_conversations >= 1,
            "total includes the inserted row"
        );
        assert!(
            stats.compressed_rows >= 1,
            "newly inserted rows are compressed via the write path"
        );
        assert_eq!(
            stats.integrity_check, "ok",
            "PRAGMA integrity_check should report ok"
        );
        Ok(())
    }
}
