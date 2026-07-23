//! FR-DH-001: forgecode shared-3task.v1 process_smoke (no model serving).

use dual_harness::{load_fixture, run_forgecode_fixture};
use std::path::PathBuf;

fn fixture_path() -> PathBuf {
    if let Some(p) = std::env::var_os("DUAL_HARNESS_FIXTURE") {
        return PathBuf::from(p);
    }
    // Prefer sibling pheno-harness worktree fixture when present (pre-merge),
    // else default path used by the library.
    let candidates = [
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(5)
            .unwrap()
            .join("worktrees/pheno-harness/fixture-forgecode-adapters/plans/2026-07-22-dual-harness-matrix/fixtures/shared-3task.v1.json"),
        dual_harness::default_shared_3task_path(),
    ];
    for c in candidates {
        if c.exists() {
            return c;
        }
    }
    dual_harness::default_shared_3task_path()
}

#[tokio::test]
async fn shared_3task_all_pass() {
    let path = fixture_path();
    assert!(
        path.exists(),
        "fixture missing at {} (set DUAL_HARNESS_FIXTURE)",
        path.display()
    );

    let tmp = tempfile::tempdir().expect("tempdir");
    // SAFETY: test process; isolate workdir for t2.
    unsafe {
        std::env::set_var("DUAL_HARNESS_WORKDIR", tmp.path());
    }

    let fixture = load_fixture(&path).expect("load fixture");
    let outcomes = run_forgecode_fixture(&fixture).await.expect("run fixture");
    assert_eq!(outcomes.len(), 3, "expected 3 shared tasks");
    for o in &outcomes {
        assert!(o.passed, "{} failed: {}", o.task_id, o.detail);
    }
}
