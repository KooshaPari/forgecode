//! Mutation testing utilities.
//!
//! ## xDD Methodology: Mutation Testing
//!
//! Mutation testing evaluates test quality by introducing small
//! changes (mutations) to the code and verifying tests catch them.
//!
//! ## Metrics
//!
//! - **Mutation Score**: Percentage of killed mutations
//! - **Coverage**: Lines/branches executed by tests
//! - **Equivalent Mutations**: Mutations that don't change behavior
//!
//! ## Usage
//!
//! ```rust,ignore
//! let tracker = MutationTracker::new();
//! tracker.record_execution("src/lib.rs", 42);
//! let score = tracker.mutation_score();
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Mutation coverage tracker.
#[derive(Debug, Default, Clone)]
pub struct MutationTracker {
    /// File execution counts.
    files: HashMap<String, FileCoverage>,
    /// Total mutations introduced.
    total_mutations: usize,
    /// Mutations killed by tests.
    killed_mutations: usize,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct FileCoverage {
    lines_executed: usize,
    /// Total lines of code in the file (set via `record_file_loc` or
    /// derived from `max_line_seen` if the caller never supplied it).
    #[serde(default)]
    total_loc: usize,
    /// Highest line number passed to `record_line_execution`.
    #[serde(default)]
    max_line_seen: usize,
    branches_executed: usize,
    mutations: Vec<Mutation>,
}

impl FileCoverage {
    /// Effective LOC used for coverage math. Prefers the explicitly
    /// recorded total; falls back to `max_line_seen` when the caller
    /// never invoked `record_file_loc`.
    fn effective_loc(&self) -> usize {
        if self.total_loc > 0 {
            self.total_loc
        } else {
            self.max_line_seen
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Mutation {
    id: String,
    line: usize,
    status: MutationStatus,
    kind: MutationKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationStatus {
    /// Mutation was killed by a test.
    Killed,
    /// Mutation survived all tests.
    Survived,
    /// Mutation is equivalent to original.
    Equivalent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationKind {
    /// Arithmetic operator flipped (e.g., + to -)
    Arithmetic,
    /// Comparison operator changed (e.g., == to !=)
    Comparison,
    /// Boolean operator negated (e.g., && to ||)
    Boolean,
    /// Value replaced with default/null
    ValueReplacement,
    /// Statement removed
    StatementRemoval,
}

impl MutationTracker {
    /// Create a new mutation tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a line execution.
    ///
    /// `total_loc` should be the total lines of code in `file`. The coverage
    /// calculation divides lines_executed by total_loc, so the caller must
    /// supply the canonical LOC count for the file (e.g. via `walkdir` +
    /// `std::fs::read_to_string` at instrumentation time).
    pub fn record_line_execution(&mut self, file: &str, line: usize) {
        let entry = self.files.entry(file.to_string()).or_default();
        entry.lines_executed += 1;
        // Track the highest line we have seen so the per-file total can be
        // derived when the caller does not supply a LOC count up front.
        if line > entry.max_line_seen {
            entry.max_line_seen = line;
        }
    }

    /// Record a file's total LOC up front (preferred for accurate coverage).
    pub fn record_file_loc(&mut self, file: &str, loc: usize) {
        let entry = self.files.entry(file.to_string()).or_default();
        if loc > entry.total_loc {
            entry.total_loc = loc;
        }
    }

    /// Record a mutation introduction.
    pub fn introduce_mutation(&mut self, file: &str, line: usize, kind: MutationKind) -> String {
        let id = format!("{}:{}:{:?}", file, line, kind);
        self.total_mutations += 1;
        self.files
            .entry(file.to_string())
            .or_default()
            .mutations
            .push(Mutation {
                id: id.clone(),
                line,
                status: MutationStatus::Survived,
                kind,
            });
        id
    }

    /// Mark a mutation as killed.
    pub fn kill_mutation(&mut self, id: &str) {
        for file in self.files.values_mut() {
            if let Some(m) = file.mutations.iter_mut().find(|m| m.id == id) {
                m.status = MutationStatus::Killed;
                self.killed_mutations += 1;
                return;
            }
        }
    }

    /// Mark a mutation as equivalent.
    pub fn mark_equivalent(&mut self, id: &str) {
        for file in self.files.values_mut() {
            if let Some(m) = file.mutations.iter_mut().find(|m| m.id == id) {
                m.status = MutationStatus::Equivalent;
                self.total_mutations = self.total_mutations.saturating_sub(1);
                return;
            }
        }
    }

    /// Calculate mutation score (0.0 to 1.0).
    pub fn mutation_score(&self) -> f64 {
        if self.total_mutations == 0 {
            return 1.0;
        }
        self.killed_mutations as f64 / self.total_mutations as f64
    }

    /// Get coverage percentage for a file (0.0..=1.0).
    ///
    /// When the file's LOC was never recorded via `record_file_loc`, the
    /// highest line number seen is used as the denominator. If nothing
    /// was ever executed for the file, returns 0.0.
    pub fn coverage(&self, file: &str) -> f64 {
        self.files
            .get(file)
            .map(|f| {
                let total = f.effective_loc();
                if total == 0 {
                    0.0
                } else {
                    f.lines_executed.min(total) as f64 / total as f64
                }
            })
            .unwrap_or(0.0)
    }

    /// Get all tracked files.
    pub fn files(&self) -> impl Iterator<Item = (&str, usize)> {
        self.files.iter().map(|(k, v)| (k.as_str(), v.lines_executed))
    }
}

/// Coverage report for a mutation run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageReport {
    pub total_lines: usize,
    pub executed_lines: usize,
    pub line_coverage: f64,
    pub total_branches: usize,
    pub executed_branches: usize,
    pub branch_coverage: f64,
}

impl CoverageReport {
    /// Create from a tracker.
    ///
    /// Total lines are the sum of each tracked file's effective LOC
    /// (recorded via `record_file_loc`, or derived from the highest
    /// line number seen).
    pub fn from_tracker(tracker: &MutationTracker) -> Self {
        let (total_lines, executed_lines) =
            tracker.files().fold((0usize, 0usize), |(t, e), (_, exec)| {
                // `files()` yields (file, lines_executed) but we need the
                // full entry to get total_loc. Re-fetch to be precise.
                (t, e + exec)
            });
        // For a more accurate per-file sum, walk the underlying map.
        let precise_total: usize = tracker
            .files
            .values()
            .map(|f| f.effective_loc())
            .sum();
        let total_lines = if precise_total == 0 {
            total_lines
        } else {
            precise_total
        };
        Self {
            total_lines,
            executed_lines: executed_lines.min(total_lines),
            line_coverage: if total_lines > 0 {
                executed_lines.min(total_lines) as f64 / total_lines as f64
            } else {
                0.0
            },
            total_branches: 0,
            executed_branches: 0,
            branch_coverage: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracker_creation() {
        let tracker = MutationTracker::new();
        assert_eq!(tracker.mutation_score(), 1.0);
    }

    #[test]
    fn test_record_line_execution() {
        let mut tracker = MutationTracker::new();
        // Without recording LOC, coverage falls back to max_line_seen / max_line_seen = 1.0
        tracker.record_line_execution("src/lib.rs", 10);
        assert_eq!(tracker.coverage("src/lib.rs"), 1.0);

        // When the file has more lines than were executed, coverage is partial.
        let mut tracker = MutationTracker::new();
        tracker.record_file_loc("src/lib.rs", 200);
        tracker.record_line_execution("src/lib.rs", 10);
        assert!((tracker.coverage("src/lib.rs") - (1.0 / 200.0)).abs() < 1e-9);

        // Clamps to 1.0 if more lines were executed than recorded.
        let mut tracker = MutationTracker::new();
        tracker.record_file_loc("src/lib.rs", 100);
        tracker.record_line_execution("src/lib.rs", 50);
        tracker.record_line_execution("src/lib.rs", 150);
        assert_eq!(tracker.coverage("src/lib.rs"), 1.0);
    }

    #[test]
    fn test_mutation_introduction() {
        let mut tracker = MutationTracker::new();
        let id = tracker.introduce_mutation("src/lib.rs", 42, MutationKind::Arithmetic);
        assert_eq!(tracker.mutation_score(), 0.0);
        tracker.kill_mutation(&id);
        assert_eq!(tracker.mutation_score(), 1.0);
    }

    #[test]
    fn test_mutation_killing() {
        let mut tracker = MutationTracker::new();
        let id1 = tracker.introduce_mutation("src/lib.rs", 1, MutationKind::Arithmetic);
        let _id2 = tracker.introduce_mutation("src/lib.rs", 2, MutationKind::Comparison);
        tracker.kill_mutation(&id1);
        assert_eq!(tracker.mutation_score(), 0.5);
    }

    #[test]
    fn test_equivalent_mutation() {
        let mut tracker = MutationTracker::new();
        let id = tracker.introduce_mutation("src/lib.rs", 42, MutationKind::ValueReplacement);
        tracker.mark_equivalent(&id);
        // Equivalent mutations don't count toward total
        assert_eq!(tracker.mutation_score(), 1.0);
    }
}
