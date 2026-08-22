# Changelog

All notable changes to ForgeCode are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). This baseline was
reconstructed from the ten most recent GitHub releases and the latest 100 commits.

## [2.11.0](https://github.com/KooshaPari/forgecode/compare/v2.10.8...v2.11.0) (2026-08-22)


### Features

* **a11y:** add accessibility module with WCAG 2.1 AA helpers, screen-reader descriptions, contrast checking, and semantic validation ([c7e9749](https://github.com/KooshaPari/forgecode/commit/c7e974963360ea0a50c02efb20778a75d31f4baf))
* add i18n locale files and perf baselines ([2c58e18](https://github.com/KooshaPari/forgecode/commit/2c58e18615b91ae2f40f699cf0d10d401b31b89f))
* add integration tests, release-please, SBOM workflow, i18n locales, and a11y CLI flag ([4aeecc9](https://github.com/KooshaPari/forgecode/commit/4aeecc90ecc5c8012814a540f670baca67b1646a))
* **agileplus:** bootstrap full AgilePlus setup with 31-pillar scorecard, sprint tracking, quality gates, branch cleanup ([2ad45f6](https://github.com/KooshaPari/forgecode/commit/2ad45f67239d7a757fa658ea6ee55a69f219ad1e))
* **dbd:** daemon lifecycle - idle shutdown and spawn-on-first-write ([f167fa2](https://github.com/KooshaPari/forgecode/commit/f167fa2807b5c784f65b0ea9d0ac587b8e7cc355))
* **dbd:** handle --version and --help without starting the daemon ([550c63a](https://github.com/KooshaPari/forgecode/commit/550c63a7578a297fbd35ae3af1c755a1533aef95))
* **dbd:** ship forge_dbd in release matrix and CI ([#176](https://github.com/KooshaPari/forgecode/issues/176)) ([02ee69a](https://github.com/KooshaPari/forgecode/commit/02ee69a9454eb2a9a6ff731c48f9149792c9dcf8))
* **desktop:** M1 - Tracera desktop shell with Tauri 2, project board, and system tray ([b98ad13](https://github.com/KooshaPari/forgecode/commit/b98ad133ea5a68c87c5bd55dce378618e4adad36))
* **devex:** add ADRs, DORA metrics, Docker dev env, and incident response playbook ([747fd4e](https://github.com/KooshaPari/forgecode/commit/747fd4e90de0c05aeb55d06c9d8464cf065a4caf))
* **docs:** add SLA/SLO documentation and perf trend tracking ([8e918ab](https://github.com/KooshaPari/forgecode/commit/8e918ab9a488e72c47e41f8add24fbc83e63bac0))
* **forge_dbd:** real conversation writes, lib+bin, named-pipe transport ([44cb74a](https://github.com/KooshaPari/forgecode/commit/44cb74a320cf9d1f046ef8259a2cf604161bae6c))
* **forge_dbd:** serve concurrent named-pipe clients and honor client workspace id ([72793d8](https://github.com/KooshaPari/forgecode/commit/72793d8d6cca4b30973f72e247ef5bb54a1aac8d))
* **forge_repo:** stamp client workspace id on daemon-routed upserts ([800d9af](https://github.com/KooshaPari/forgecode/commit/800d9af1efbf031d94f4120f85c99468af52c02c))
* **i18n:** add internationalization scaffolding with English locale ([e18a890](https://github.com/KooshaPari/forgecode/commit/e18a89060900d8440f8dd8009ddf619d9bff2a2c))
* **infra:** add OpenTelemetry, chaos testing, perf dashboard, and multi-region docs ([eb13f30](https://github.com/KooshaPari/forgecode/commit/eb13f30ac31a50e42acfac425d9df2806d07f958))
* **repo:** route hot-path conversation writes through forge_dbd (opt-in) ([bfaf147](https://github.com/KooshaPari/forgecode/commit/bfaf1479fff287b71e780810a53e6a639e414a5f))
* **search:** M3 - Full-text search (FTS5), GitHub webhooks, notifications, markdown rendering, custom fields, and export ([e0816d9](https://github.com/KooshaPari/forgecode/commit/e0816d9d1338abfe5084afe7f5e4b5e2075f1788))
* **sprint:** M2 - Sprint management, burndown chart, velocity tracking, label management, and enhanced filters ([8fb8c02](https://github.com/KooshaPari/forgecode/commit/8fb8c02364f42836a11adca35532e8847451ee2f))
* **sre:** add chaos CI gate, Terraform IaC, SLO burn rate alerting, and OTel collector config ([d39762f](https://github.com/KooshaPari/forgecode/commit/d39762f49240aeda61bf165e3489c2b2749d8afe))
* **sre:** add SLO alerting, OTel deployment scripts, terraform validate CI ([10e2c90](https://github.com/KooshaPari/forgecode/commit/10e2c90901280facae4e5501c617b2d75d45a495))
* **sync:** continuous forge → helioslite upstream sync ([6f60cdd](https://github.com/KooshaPari/forgecode/commit/6f60cddd9ebaec656143ca7e550a9bee410a2bc0))
* **testing:** add fuzz tests, 3 new locales, and codeowners verification ([c72c64c](https://github.com/KooshaPari/forgecode/commit/c72c64cd519e2077d2c46e4be9c0e5d88aee943a))


### Bug Fixes

* **anthropic:** scope refusal handling to Anthropic ([#166](https://github.com/KooshaPari/forgecode/issues/166)) ([cc15029](https://github.com/KooshaPari/forgecode/commit/cc150298e6d1527ffb505194d6e2c66597217e0d))
* **ci:** correct deny.toml RUSTSEC-2026-0258 ignore for cargo-deny 0.19 ([62ef21a](https://github.com/KooshaPari/forgecode/commit/62ef21ae156f1379fea0daf3923acc2cc9f05d91))
* **ci:** import OpenTelemetry memory exporter from supported module ([ab399e2](https://github.com/KooshaPari/forgecode/commit/ab399e2502217a72ee3e4fe36cc1ae76592797cd))
* **ci:** integrate workflow reliability repairs ([4118870](https://github.com/KooshaPari/forgecode/commit/41188704cf9d17d86fedb2d06e765bdc33121745))
* **ci:** pin release-drafter version to 2.13.22 ([0febb7a](https://github.com/KooshaPari/forgecode/commit/0febb7aaa63a806f9c4e9b70641193a7f3671b70))
* **ci:** pin release-drafter version to fork scheme v2.13.21-h.0.1.1 ([e59b637](https://github.com/KooshaPari/forgecode/commit/e59b637f51affec77f24fbe5bf8a294009418aa7))
* **ci:** publish helioslite release assets from the forge_ci model ([4b2af40](https://github.com/KooshaPari/forgecode/commit/4b2af40e7d90093efb06eb83b3c499bb70c4118f))
* **ci:** remove incompatible Trunk action ([#171](https://github.com/KooshaPari/forgecode/issues/171)) ([129182e](https://github.com/KooshaPari/forgecode/commit/129182ea8ca7dabda7bf0511929b50c07e7571e4))
* **ci:** resolve cargo-deny failures surfaced by the v2.13.21 sync ([cf3a3ef](https://github.com/KooshaPari/forgecode/commit/cf3a3eff1b47af471344cf035b563dddb8d7f702))
* **ci:** restore coverage and pre-commit prerequisites ([#169](https://github.com/KooshaPari/forgecode/issues/169)) ([df506d6](https://github.com/KooshaPari/forgecode/commit/df506d662110f6830c3989a9a01f89cb7db17191))
* **ci:** restore immutable action pins ([#173](https://github.com/KooshaPari/forgecode/issues/173)) ([1cbfda6](https://github.com/KooshaPari/forgecode/commit/1cbfda6c28061139a4ee6653dafc41f83b30a480))
* **ci:** validate chaos workflow dispatch ([#174](https://github.com/KooshaPari/forgecode/issues/174)) ([0ca5423](https://github.com/KooshaPari/forgecode/commit/0ca54237394aea4e581a8f2bc133683874895f5f))
* **clippy:** satisfy -D warnings in forge_app, forge_infra, forge_repo, forge_main ([ae4096a](https://github.com/KooshaPari/forgecode/commit/ae4096a365ba5846b1f9c935ee9be84efcef8413))
* **dbd:** honor FORGE_DBD_SOCKET in the daemon and forward it on spawn ([df42f1d](https://github.com/KooshaPari/forgecode/commit/df42f1d4874d35bdd39e1a4d0b3d2ecd04b3e54a))
* **doctor,test:** CRLF-normalize embedded templates, gate Windows-path test, dedupe mock ([b6706f2](https://github.com/KooshaPari/forgecode/commit/b6706f2ddf842b4fed42b7c790e6effdceaccb38))
* **doctor:** forward database_integrity through infra wrappers ([a3d5b1e](https://github.com/KooshaPari/forgecode/commit/a3d5b1e881738396d6d7bebc1bfe3120a445d2cf))
* **dual_harness:** run pwd tasks via cmd /c cd on Windows ([9bb315f](https://github.com/KooshaPari/forgecode/commit/9bb315f4a1ad3acc895e49ec23f383cdc675d93b))
* **forge_main:** make paste/editor path tests Windows-safe ([c275105](https://github.com/KooshaPari/forgecode/commit/c275105647f0a9bdebc6fc1f1481564660389bd0))
* **forge_pheno_shell:** make install-target path tests Windows-safe ([44adf66](https://github.com/KooshaPari/forgecode/commit/44adf6680e97e2444f0a170a9df44ad92a862665))
* **forge_pheno_winterminal:** gate NotWindows detection test per platform ([0c9f994](https://github.com/KooshaPari/forgecode/commit/0c9f994e0540fc0b5d92d6daf6f9664703bf121e))
* **forge_repo_map:** make rust-file discovery test Windows-safe ([65d1e14](https://github.com/KooshaPari/forgecode/commit/65d1e148f10ed9453de25cd079078acd819620d9))
* **forge_walker:** make tests Windows-safe and gate unix-only symlink tests ([4f3d339](https://github.com/KooshaPari/forgecode/commit/4f3d339e08e97f48c60b1f87870efed499ba6635))
* **forge3d:** gate unix-socket cancellation test on unix ([77d8f09](https://github.com/KooshaPari/forgecode/commit/77d8f09f0a6350d37d226cfd27abb18642523b26))
* **helioslite:** harden Forge snapshot import boundary ([#160](https://github.com/KooshaPari/forgecode/issues/160)) ([d5136de](https://github.com/KooshaPari/forgecode/commit/d5136deb1dab32e0d3e6b5518eaa3a8441f016e1))
* **orch:** remove duplicate database stats stub ([#157](https://github.com/KooshaPari/forgecode/issues/157)) ([d930d59](https://github.com/KooshaPari/forgecode/commit/d930d59b0de43376e430ec2ae429c89024af6f79))
* **provider:** retry transient OpenAI server errors ([#161](https://github.com/KooshaPari/forgecode/issues/161)) ([a533b3d](https://github.com/KooshaPari/forgecode/commit/a533b3dc9be09622f437ba3427fbfb822d8caab7))
* **release:** align authoritative version metadata to 2.10.9 ([#155](https://github.com/KooshaPari/forgecode/issues/155)) ([7469c73](https://github.com/KooshaPari/forgecode/commit/7469c731cd6afe4418da7ea1a3285fbf7df4ed19))
* **release:** align authoritative version metadata to 2.13.21 ([3a2a86b](https://github.com/KooshaPari/forgecode/commit/3a2a86b0b55abcb7eb53748e7c3a91338047cf99))
* **repo:** harden daemon write lifecycle ([#165](https://github.com/KooshaPari/forgecode/issues/165)) ([9581782](https://github.com/KooshaPari/forgecode/commit/958178298aa14053b61857383cd49cf5809cef8b))
* **repo:** prevent daemon write replay after ack loss ([#163](https://github.com/KooshaPari/forgecode/issues/163)) ([0cb8478](https://github.com/KooshaPari/forgecode/commit/0cb8478518b87d0fff40bafee2dd92949425664d))
* **security:** bump h2 0.4.13 -&gt; 0.4.16 for RUSTSEC-2026-0258 and adjust deny.toml ([72ec463](https://github.com/KooshaPari/forgecode/commit/72ec4631938fdb299e821e38289fd79064189374))
* **test:** remove duplicate database_stats in orch_runner Runner mock ([1f0d519](https://github.com/KooshaPari/forgecode/commit/1f0d519922cd33a47c5f4216b49349f06895afed))
* **test:** use bstr decoding in split_db_cli to satisfy clippy gate ([d7dc206](https://github.com/KooshaPari/forgecode/commit/d7dc20623df4ada1b81078ee50b3960838c4e9dc))
* **tool-macros:** normalize CRLF in embedded tool descriptions ([827d42b](https://github.com/KooshaPari/forgecode/commit/827d42b897338312aa6f7374594dc7f56cc987f4))
* **zsh:** bound zsh execution so doctor/setup can never hang the CLI ([b114801](https://github.com/KooshaPari/forgecode/commit/b1148014a23d9608d14c5a46b58f346a2d58d3bc))

## [Unreleased]

### Added
- Weekly AgilePlus 31-pillar scorecard publishing to a GitHub issue.
- Nightly cleanup policy for stale and merged, unprotected remote branches.
- AgilePlus sprint, backlog, quality-gate, velocity, and ownership tracking.

### Changed
- Repository governance now includes Contributor Covenant v2.1.

## [v2.13.21-h.0.1.1] - 2026-08-19

### Added
- Multi-client named-pipe support with client workspace IDs.
- Workspace identity on daemon-routed repository upserts.

### Fixed
- Release version metadata and release-drafter pin alignment.
- Windows path, cancellation, and test-gate failures in repository and shell components.
- Zsh setup bounded so interactive configuration cannot hang.
- Cargo-deny failures and Windows-safe discovery tests.

### Changed
- Added guidance to preserve interactive Forge and HeliosLite sessions.

## [v2.10.8] - 2026-08-10

### Fixed
- Restored and repaired Rust 1.96 quality gates.
- Completed environment trait mocks and removed an unused base-path resolver.
- Retired obsolete merge dependency lineage.

### Added
- Deterministic SBOM generation policy and verified HeliosLite snapshot import.

## [v2.10.7] - 2026-08-08

### Added
- ForgeDB daemon with real SQLite operations, named-pipe transport, and spawn-on-first-write lifecycle.
- Split-database CLI integration coverage and doctor integrity checks.

### Fixed
- Restored the `indexmap` serde feature, hardened doctor and installer verification, and updated Infisical workflow permissions and pins.
- Hardened cross-platform path tests and removed stale CI workflow generators.

## [v2.10.6] - 2026-08-05

### Fixed
- Defaulted Zsh dispatch to ForgeCode.

### Changed
- Refreshed CycloneDX SBOM artifacts and removed a deprecated Handlebars type stub.

## [v2.10.5] - 2026-08-04

### Added
- Attestation for published release assets.

### Fixed
- Pinned updater matrix assets and hardened shell installation.

## [v2.10.4] - 2026-08-03

### Fixed
- Skipped scheduled bounty pull-request synchronization.

## [v2.10.3] - 2026-08-03

### Fixed
- Ran release checksums under Bash, disabled unsupported fork package channels, and escaped updater PowerShell braces.
- Corrected native Windows auto-update, PATH length handling, and Ctrl+C terminal restoration.
- Bounded automatic continuation after interrupts.

## [v2.10.2] - 2026-08-02

### Fixed
- Made doctor and shell setup portable, restored the release workflow, and compiled the Windows doctor skip guard.
- Removed an unsafe Infisical secret workflow and stale RustSec ignores.

### Changed
- Scoped Scorecard SARIF permission to its job and linked private vulnerability reporting.

## [v2.10.1] - 2026-08-01

### Fixed
- Used a modern AWS HTTPS client and promoted workflow-gate repairs.
- Hardened Scorecard, pinned the CodeQL upload action, and restored fork behavior.

## [v2.10.0] - 2026-07-29

### Added
- Fork release with deterministic compaction and workspace handling.
- Generated workflow/schema parity and nine-platform build artifacts.

[unreleased]: https://github.com/KooshaPari/forgecode/compare/v2.13.21-h.0.1.1...HEAD
[v2.13.21-h.0.1.1]: https://github.com/KooshaPari/forgecode/releases/tag/v2.13.21-h.0.1.1
[v2.10.8]: https://github.com/KooshaPari/forgecode/releases/tag/v2.10.8
[v2.10.7]: https://github.com/KooshaPari/forgecode/releases/tag/v2.10.7
[v2.10.6]: https://github.com/KooshaPari/forgecode/releases/tag/v2.10.6
[v2.10.5]: https://github.com/KooshaPari/forgecode/releases/tag/v2.10.5
[v2.10.4]: https://github.com/KooshaPari/forgecode/releases/tag/v2.10.4
[v2.10.3]: https://github.com/KooshaPari/forgecode/releases/tag/v2.10.3
[v2.10.2]: https://github.com/KooshaPari/forgecode/releases/tag/v2.10.2
[v2.10.1]: https://github.com/KooshaPari/forgecode/releases/tag/v2.10.1
[v2.10.0]: https://github.com/KooshaPari/forgecode/releases/tag/v2.10.0
