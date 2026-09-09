# Mac source coverage and first semantic bundles (2026-09-09)

Baseline captured before this ledger worktree was created. Main: `999b5c5046c047b7041fdfe0104e1e0135d3adb2`.
Historical PR257 ledger: 2652 source/ref records from three Mac/Windows checkouts, 1116 distinct tips. This lane refreshes Mac only, not Windows.

| Measure | Historical Mac | Current Mac |
|---|---:|---:|
| All refs |1195|1195|
| Local heads |31|44|
| Remote-tracking refs |753|739|
| Registered worktrees |16|29|
| Stashes |0|0|

Current Mac has 1093 unique tips; 31 refs added, 31 removed, 6 moved since historical snapshot. Counts include 395 tags,16 keep refs,1 tmp ref. Branch-only subset:783 records/704 tips (44 local,337 fork tracking,2 origin,400 upstream). Live fork had327 heads/283 tips before additive integration restoration; tracking is not live remote state.

## Squash-aware coverage

`git diff 06bf223d564cf30c3db3bfd4461bf798542b0a22 61ddccfd5530910e39a5cf7766be58708fb337af` is empty. The latter is main's PR254 squash. Main999 adds only the three h.0.1.7 version lines afterward. Therefore source ancestry maps through that verified squash, but does not assert every superseded historical behavior still exists.

Historical1116 tips:465 direct ancestors +12 verified squash-source ancestors =477 contained,625 pending,14 unknown. This precisely reconciles original477/625/14; no false regression from squash ancestry.
Current Mac1093 tips:457 direct +21 squash-source +615 requiring patch review.
All615 received `git cherry MAIN TIP`:71 have only patch-equivalent nonmerge commits;542 have unmatched patches;2 merge-only tips require aggregate-tree review. Patch equivalence does not prove merge-resolution equivalence or semantic uniqueness.

Dirty status only (no contents):4 worktrees/8 status entries. Root1; checksum worktree3; signing worktree2; historical coverage worktree2. All29 status commands succeeded.

## Preservation correction

Live integration source ref was absent. Verified commit locally, then restored additively via SSH without force:
`git push git@github.com:KooshaPari/forgecode.git 06bf223d564cf30c3db3bfd4461bf798542b0a22:refs/heads/integration/forgecode-h0.1.7`.
Live ls-remote confirmed exact SHA. This adds one remote ref, no new tip; not a merge or consolidation completion. No refs deleted/pruned/reset.

## First semantic bundles

1. Legacy schema visibility: `fix/forge-reasoning-retry-20260908` / `39c597afda0e43b90ebef6b2476320570eef67f9` (also remote repair/pr238-conversation-record-lints-20260904). Four unmatched commits in branch, four equivalent. Source tip adds missing-workspace sentinel and discovery read filters absent main. Must port semantically: source also widens DELETE/prune queries and reverts newer projection fixes if copied wholesale; retain strict mutation scope and current pragma/intent defaults. Regression-first port authorized.
2. Signing pipeline `fix/signing-pipeline` / `81cc94cb4a4bb62b61019df530de1ba508b126ba`: four unmatched patches, existing PR271 owner; do not duplicate.
3. Sandbox portability `fix/forge-sandbox-windows-config-20260901` / `ec13b571401e9617f5d7314bd5157b5d83ac96f4`: source tip changes /tmp to temp_dir, but current main uses TempDir fixture at affected test. Superseded semantic behavior; no mechanical reapplication.
4. `fix/stable-rustfmt-contract-20260829` / `dd4e1131f8cfe0e2a84402ddb58b7cce8069c0de`: one unmatched patch touching rustfmt and helper formatting; requires current policy comparison before bundling.
5. F3 `feat/forge-f3-semantic-memory-20260830` / `a9929f235d84e9f8ca26352908c213bbbfbb18c5`: two unmatched patches; existing reconciled F3 bundle exists, so semantic equivalence remains required.

## Five additional source dispositions

Read-only comparisons against main `999b5c5046c047b7041fdfe0104e1e0135d3adb2`; no build or runtime claim. These five exact tips close from the fixed 542-tip pending baseline, separately from the three SDK tips recorded in `SDK-SOURCE-DISPOSITION-20260909.md`.

| Exact source tip | Disposition | Evidence |
|---|---|---|
| `8547d257c19f010e5d19ac9e85c556c5bf1ae029` | Represented | Entire retained SBOM merge tree equals parent `99cb7a0c08c997d28188aeefd7b9a420a9f3afb5`, an ancestor of audited main. |
| `c183f10d20b377b31e9f07afe9caa700292c7c3a` | Represented through retained resolution | Original SBOM change retained in the `8547d257` lineage; its resolved aggregate tree is already contained. |
| `cd677cee82d017002e3e4808d8fb2b949c67a983` | Represented through retained resolution | Intermediate SBOM merge is parent of `8547d257`; final merge resolutions select contained implementation, not an additional missing bundle. |
| `a9929f235d84e9f8ca26352908c213bbbfbb18c5` | Represented and extended | All three touched paths exactly equal integration `c38aa7364` (PR216), including Cargo.lock. Main only adds serialization derives in semantic_memory.rs through `8837923f9` (PR247). All seven original contract tests remain. |
| `ec13b571401e9617f5d7314bd5157b5d83ac96f4` | Superseded | Source temp_dir fixture is superseded by owned TempDir plus path assertion in `5ffdc3eda` (PR222), `crates/forge_sandbox/src/config.rs:216`. |

No source port or synthetic PR is justified for these groups. Accounting delta is five covered and five fewer pending, without changing the denominator or authorizing ref retirement. Retained branch aliases are local/fork `fix/release-sbom-20260826`, `feat/forge-f3-semantic-memory-20260830`, and `fix/forge-sandbox-windows-config-20260901`.

## Publication-lineage source dispositions

Read-only static comparison against the same audited main closes eight further **exact frozen pending tip** keys. Each SHA is a direct `.refs[][1]` key in `/tmp/forgecode-542-unmatched-baseline-20260909.json`; the captured ref is recorded here to avoid treating a reachable ancestor as a tip. This is source-only representation evidence, not a build, runtime, release, or readiness claim.

| Exact source tip | Captured frozen ref | Disposition and static evidence |
|---|---|---|
| `8ff6fcbe1d2e5490664ddc0a7d4fe126c1c1c56e` | `fork/wip/20260803T0034-18c82463a0b25670` | Represented: current `install.sh` retains repo/version validation, target allowlisting, mandatory checksum, and staged atomic install. |
| `f6aa83f97fa67ec70db3a7b76c83d39be461e69f` | `fork/preserve/helioslite-snapshot-contract-20260807-f6aa83f97` | Represented: current snapshot contract retains schema, row, and provenance validation. |
| `e4ac8ff09f9a6d51e6f328cc00e4e5897f93d5ae` | `fork/preserve/helioslite-wal-sidecar-20260807-e4ac8ff09` | Represented: current snapshot export rejects live SQLite WAL/SHM sidecars before reading. |
| `5343fefbb8485ae2fa324706ca748bdc32410530` | `fork/fork/preserve/helioslite-publication-20260808` | Represented: current snapshot publication retains verified idempotent atomic bundle handling. |
| `12029daf67f62555c92e29798bb608631a74a474` | `fork/fork/preserve/helioslite-publication-20260808-v2` | Represented: its two `content_sha256` explanatory lines remain in the current manifest contract. |
| `b3aad92590d851ef8d10fe217a8ac973e202652f` | `fork/fork/preserve/helioslite-root-guard-20260808` | Represented: current UI validates canonical session roots and rejects symlink and `~/.forge` overlap. |
| `ea43377a8792018310fdae9f6d053af6b18534ad` | `fork/preserve/helioslite-cli-parser-20260807` | Represented: current CLI preserves `sessions import-forge` explicit source/destination shape and UI dispatch. |
| `f39ae6a1f46666630c0d72e0148dcec5e24431c1` | `fork/preserve/runtime-fallback-fix-20260806` | Superseded: the lifetime-only updater correction was removed by the later binary-aware updater redesign, which owns the replacement path. |

Metadata and preservation wrapper tips are excluded from this source accounting, without independently closing their frozen-tip keys: `0af562c0`, `467eee76`, `a35bc3c7`, `64c9a337`, `6d7ca126`, `a0da285f`, `ab49d702`, `beef43f9`, `dccf42de`, and `dd03d085`. Source-bearing WIP tips `4fe8aaf0`, `5a75bf32`, `771a2bcd`, `aa25f50e`, and `ab34dbfb` remain pending review; no broad publication lineage is treated as covered.

Reconciled aggregate for the independently deduplicated five-tip and publication eight-tip increments: **564 represented / 529 pending / 1093 fixed Mac tips** (prior baseline 551 / 542). Independent SDK reconciliation adds **zero**: its three exact Windows tips are absent from both frozen Mac candidate partitions and are not ancestors of either baseline source; see `SDK-SOURCE-DISPOSITION-20260909.md` for evidence and direct-snapshot limitations. Three other Windows dispositions are also distinctly recorded without changing Mac counts. Runtime readiness and the PARTIAL legacy source are not promoted by these source-only closures.

## Earlier audit baseline and limits

Independent 73-tip merge-effect audit completed: 67 have no uncovered merges, four have empty remerge diffs, and two stash resolution deltas are present in baseline. Evidence: `/private/tmp/forgecode-73-candidate-independent-audit-20260909.md`. Historical source coverage is therefore 551/1093 Mac tips; 542 remain unmatched. This is not a guarantee of current runtime semantics.

Exact remaining queue: `/tmp/forgecode-542-unmatched-baseline-20260909.json`, 586 source/ref aliases for542 baseline tips. Newly authored ledger and port branches are excluded from this fixed denominator. Frontier/shared-lineage audit delegated to independent reviewer.

First actual semantic port: PR272, commit `9f19f1b92`, source39c597af partial. New legacy-discovery regression failed main before port; full forge_repo414 passed/1ignored after port; strict clippy passed. Internal TEMP VIEW provenance flag distinguishes missing-schema workspace from real local/legacy workspace0. Source DELETE/prune/local-only FTS broadening deliberately excluded; source branch remains PARTIAL, other unmatched changes unclassified. See PR272's `LEGACY-DISCOVERY-PORT-20260909.md` for disposition and exact proof.

Read-only inventory script `/tmp/forgecode-mac-triage.py` uses for-each-ref, rev-list, commit peeling, full git cherry per pending tip, worktree porcelain/status and stash list. Raw historical snapshot remains under original coverage worktree. No private contents committed. No branch is approved for deletion or bulk merge. Windows live delta remains separate owner.
