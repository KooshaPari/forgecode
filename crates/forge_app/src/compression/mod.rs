//! Programmatic, semantic, and AI-driven compression & summarization hooks.
//!
//! These modules integrate into the existing compaction pipeline to provide
//! richer context-reduction strategies beyond simple position-based truncation.

pub mod strategy;
pub mod semantic;
pub mod ai_driven;

use forge_domain::{Compact, Context, MessageEntry, Role, TextMessage, TokenCount};

/// Describes what happened during a compression pass.
#[derive(Debug, Clone, Default)]
pub struct CompressionReport {
    /// Messages removed (by programmatic rules)
    pub removed: Vec<usize>,
    /// Messages summarized (semantic / AI)
    pub summarized: Vec<usize>,
    /// Whether the context was truncated (hard cap)
    pub truncated: bool,
    /// Tokens saved
    pub tokens_saved: usize,
    /// Remaining token count
    pub remaining_tokens: usize,
}

/// Run all enabled compression strategies against `context`.
///
/// Returns a report and the (possibly modified) context.
pub fn compress(
    context: impl Into<Context>,
    config: &Compact,
) -> (Context, CompressionReport) {
    let mut ctx: Context = context.into();
    let mut report = CompressionReport::default();

    let total_tokens = ctx.token_count_approx();
    let budget = config.token_threshold.unwrap_or(80_000) as usize;
    let level = config.context_compression_level.unwrap_or(0);

    if total_tokens <= budget {
        return (ctx, report);
    }

    // 1. Programmatic strategy (always on for level >= 1)
    if level >= 1 {
        let (c, r) = strategy::compress_programmatic(ctx, config);
        ctx = c;
        report.removed.extend(r.removed);
        report.tokens_saved += r.tokens_saved;
    }

    // 2. Semantic strategy (level >= 2)
    if level >= 2 && ctx.token_count_approx() > budget {
        let (c, r) = semantic::compress_semantic(ctx, config);
        ctx = c;
        report.summarized.extend(r.summarized);
        report.tokens_saved += r.tokens_saved;
    }

    // 3. AI-driven strategy (level >= 3)
    if level >= 3 && ctx.token_count_approx() > budget {
        let (c, r) = ai_driven::compress_ai(ctx, config);
        ctx = c;
        report.summarized.extend(r.summarized);
        report.tokens_saved += r.tokens_saved;
    }

    report.remaining_tokens = ctx.token_count_approx();
    report.truncated = report.remaining_tokens > budget;

    (ctx, report)
}

/// Compress a single message to a shorter form.
pub fn summarize_message(msg: &MessageEntry, max_chars: usize) -> Option<String> {
    let text = msg.to_text();
    if text.len() <= max_chars {
        return None;
    }
    // Simple truncation with ellipsis at word boundary
    let truncated: String = text.chars().take(max_chars.saturating_sub(3)).collect();
    Some(format!("{}...", truncated.trim()))
}
