# Implementation Strategy

Use a fresh worktree rooted at the active `fix/pull-request-target` head. Keep
the change badge-only and avoid canonical integration while unrelated Rust,
deny policy, and plan edits are present in the canonical checkout.
