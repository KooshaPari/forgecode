# SDK source bundle disposition

Compared against main999b5c5046c047b7041fdfe0104e1e0135d3adb2.

| Source ref (Windows) | Exact tip | Disposition |
|---|---|---|
| feat/helios-agent-sdk-wiring | f0879ebfd23d171809e33dd14d43b741826a1703 | Source functionality represented; refinements superseded |
| feat/helios-bot-sdk | 4e4bd9457b3a2c3787a3efc773712e913cb3440a | Alternate duplicate bot history |
| fix/main-ci-fmt-clippy | 7f8950b91bb1665bdcc225592e119565a1834f5d | Bot SDK duplicate; mock-provider lint intent superseded |

Evidence: `git diff 4e4bd945 7f8950b9 -- crates/helios-bot` is empty. f0879ebf and4e4bd945 also have identical bot trees. Main differs from those bot trees only by converting Vec references to explicit byte slices for bstr and formatting. Cargo dependencies are retained. Thus SDK-first execution, stream collection, binary fallback and response parsing are already represented; replaying these source alternatives would duplicate code.

f0879ebf additionally includes plugin source91b052f74. Main contains the full HeliosAgentHook/HeliosAgentPlugin registration and exports. Differences are bstr dependency/error conversion, unwrap simplification, formatting and replacement of stale documentation example. No missing plugin integration hunk identified.

7f8950b9 mock-provider changes: main retains the let-chain error condition and `map(Content::full)`; source direct guard equality is superseded by main's `is_some_and` predicate. Other main mock-provider differences are later substantive improvements, not missing source.

Residual: this is source-behavior coverage, not certification of the existing SDK fallback design or live provider operation. Existing broad SDK-error fallback, binary command compatibility and bot tool execution deserve separately scoped runtime review; these defects are shared with source and do not justify reapplying alternate implementations. No SDK code mutation or synthetic source PR created.

Attempted existing plugin/bot test run failed during dependency compilation with ENOSPC before tests ran. Fresh owned failed target was removed (411files/176.2MiB). No test pass claimed. Source comparisons are read-only, all original refs preserved by their owners. Unrelated Windows recovery81546 remains separate owner/scope.
