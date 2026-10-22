# W6–W9 implementation plan

Status: approved W6–W9 source implementation and isolated joint acceptance completed; development-only script gate removed after full gates, explicit trust/enable preserved. Work remains in dirty refactor/ui-gpui-kit with unrelated changes retained. No reset/commit/push/package/install/native user operations. Parent coordinated all serialized Cargo runs and one optional bounded review; exact results and unperformed installed/release checks are in outcomes.md.

Context validation warns that official component index46KiB, download reference240KiB and Desktop quality41KiB exceed32KiB auto-injection. This is not complete reference review: manually Read the full component index before UI selection and locate/read applicable download/Desktop sections before each slice. Any delegate brief must explicitly require these reads; do not change global injection settings or treat truncation as unsupported capability.

## Ordered deliverables
1. W6 child: common package/manifest/catalog/identity/config interfaces, declarative action interpretation, preview/import/conflict/enable UI and real Finder/native-action compatibility. No script process launch. Record safe source tests and W6 outcomes before moving on.
2. W7/W8/W9 child in one enablement-gated slice:
   - Versioned duplex invocation/host-call models + pending/cancel/terminal contracts.
   - Demand-start ordinary-user C# extension broker; assign-before-resume Job ownership/handle allowlist/output/deadline/tree cleanup/parent-exit and native privacy.
   - Settings/trust/content revalidation and credential Save/Cancel compensation; scripts cannot be enabled until these pass.
   - Download injected resolver at common Probe/lower acceptance and GUI/background composition; original/effective request receipt/provenance; single-handler and explicit chain; localhost browser/CLI/headless probes/submits/duplicate replay/cancel.
   - Clipboard selected-item actions and worker committed-event seam; download committed event subscription independent of view; Tools manual actions; causality/loop/rate/queue guards.
   - Quick Launch keyword and opt-in ordinary provider merge, stable IDs/selection, stale action tests and no ordinary-provider latency hostage.
   - Actual manager/settings/events/chain UI using official compounds and original feature-local controls; all feature host calls route through existing owners.
3. Parent integration: approved interfaces/examples (Python/PowerShell/Node/EXE plus W6 JSON), docs and code-spec; strict scoped checks, full safe suites, one optional directed review, concrete fixes/retest. Report source completion separately from unperformed live/release acceptance. No packaging as implicit follow-up.

## Test seams / vertical red-green
Each vertical slice first adds observable regression at the real public/caller seam, proves intended red, then smallest implementation and green. Do not write a horizontal collection of helper-only imagined tests.
- Package import preview/commit and actual temp Core config: wrong identity/version/path/oversize/conflict/update rollback vs valid reopen and unrelated manual command retention.
- Real C# fixture executable processes with temp pipes: suspended marker proves no code before Job admission; malformed/slow/flooding/child-held handles/death/revoke/interpreter missing; actual non-elevated token, active child count and ordinary tree cleanup.
- DownloadEngine common injected resolver at Probe/direct submit/enqueue/idempotent/browser/batch; actual IPC and background host fixture without real data paths: old no-op oracle, receipt reuse, scheduler responsiveness, no task on fail, origin credentials, media/Blob/schedule duplicate compatibility.
- Actual Clipboard worker temp DB/assets commit notification: success only, error/no event, duplicate/derived-output loop; GPUI selected-item inputs/actions/cancel preserve existing history.
- Actual QuickLaunch InputEvent/new generation/current native command queue: stable IDs, late replacement selection protection, W5 action revalidation, original normal/index/homophone result rules unchanged.
- Real GPUI manager at1060x650 and920x480: official Settings/Form/Alert slots, pointer/Tab/Enter/Space, long permission/settings/error list, reachable Save/Cancel/chain stage reorder, no cross-owner lease. Screenshots/native DPI not inferred from bounds tests.
- Secret save adapter/store fixture: Cancel0writes, mutation-then-throw compensation at each stage, no config/argv/env/diagnostic secret values. No real Credential Manager writes without authorization.

## Risk files / change reasons
- Cargo.toml/lock and new rust/crates/nxtools-extensions: real host-neutral extension module, only existing workspace dependencies; no toolchain upgrades/forks.
- Core config/paths: default-empty extension section and atomic package registry/trust/settings save, preserve old config.
- Protocol/IPC plus C# Program/Windows adapter and tests: bounded current-user extension transport and process/credential mode; do not alter elevated updater/indexer authorization or native event loop.
- Download engine/message/storage/http-policy and download IPC/runtime: common resolver injection/provenance/receipts/default compatibility; no separate transfer engine or UI-only parsing.
- Desktop feature composition/Tools manager/QuickLaunch/Clipboard worker: original feature owners with deferred adapter effects, actual commit notifications and stable result publishing.
- Finder/custom-command: reusable existing argv/URL/result preparation for packages; unchanged rank/query/history/manual elevated behavior.
- New docs/examples/gates: author usable interfaces, effects classifications and reproducible evidence; no reference source copying.

## Verification commands and execution policy
All direct Rust commands set toolchain PATH1.96.0, CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256 per command; one Cargo,21600s full timeout, complete scratch logs. No unrestricted Indexer tests or real scan.
- `cargo test -p nxtools-extensions -- --test-threads=1` once crate exists.
- Core/Finder/Protocol/IPC/Download-IPC safe library/contract suites; full Download lib and engine_contract serial, affected request/persistence/network/media fixtures.
- `cargo test -p nxtools-desktop -- --test-threads=1` full suite.
- `cargo clippy -p <all affected owners> --tests -- -D warnings`; `cargo check --workspace --tests`; `cargo fmt --all -- --check`.
- C# normal build and safe `--fixtures` harness plus exact newly audited synthetic process/credential effects. Tests creating process trees own temp executable/path/data; do not register themselves native/live merely to suppress failure. RUN/PASS/skip counts and zero-selection failures checked.
- QuickLaunch/download/clipboard/desktop-shell/IPC/source new-extension privacy/lifecycle gates; `python tests/architecture/spec-contracts.py`; scoped diff.
- Final doc schemas/examples exercised by parser and actual fake scripts; no fetching interpreter/script/website.

## Rollback and stopping rules
Fix concrete in-scope defects locally. No broad reopen/review loop. Unexpected failed native admission never resumes unowned script; stuck cleanup remains tracked and prevents replacement. Import/config failure keeps old selected version; secret compensation incomplete is explicit failure. Resolver error preserves original input and no task; committed feature event error remains extension failure only. If required guarantee cannot be realized through supported Windows/Kit APIs, stop that slice and report exact evidence instead of silently weakening it.

## Final handoff
Record exact test counts, red/green evidence, interfaces/version/budget compatibility and unperformed installed/user/native checks in outcomes.md/code-spec.md. Both children keep status truthful; parent complete only when W6 plus joint W7/W8/W9 and cross-feature gates pass. No commit/archive/package automatically.
