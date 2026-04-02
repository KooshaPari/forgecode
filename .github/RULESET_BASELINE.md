# forgecode Ruleset Baseline

This repository needs a bootstrap governance surface before strict branch rules can be enabled.

GitHub rulesets for protected branches should require:

- pull request required before merge
- no force push
- no branch deletion
- linear history
- `CODEOWNERS` review
- conversation resolution before merge
- required checks:
  - `ci`
  - `policy-gate`
  - `Semgrep Scan`
  - `Secret Scanning`

## Branch Policy

- `stack/*`, `layer/*`, `feat/*`, `fix/*`, `docs/*`, `refactor/*`, `ci/*`, and `chore/*` are
  valid PR head branches.
- `fix/*` must not target `main` or `master` unless the PR carries `layered-pr-exception`.
- Merge commits in PR branches are disallowed.
- Local `--no-verify` is not an accepted reason to bypass server-side workflow checks.

## Exception Policy

- Only documented billing or quota failures may be excluded from required checks.
- Review threads and blocking comments must be resolved before merge.
