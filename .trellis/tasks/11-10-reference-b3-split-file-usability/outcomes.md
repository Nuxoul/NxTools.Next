# B3 — Split-file usability delivery

Status: **accepted at source, production TestPlatform and owned-native fixture boundaries**. [Coordinator acceptance](coordinator-acceptance.md) closes B3-01/B3-02 and E1/E2 after independent repair verification. [Repair evidence](acceptance-repair.md) and tables below retain implementation results. Canonical archive status remains **Phase E — Format expansion and ZIP editing / in_progress**. Accepted [B2](../11-10-reference-b2-intake-control/coordinator-acceptance.md) is retained in place; B4–B7 remain paused.

## Delivered behavior

### REF-17 — Precise split sizes

Original Create form → stable official NumberInput/Select → checked exact byte snapshot → original ArchiveJob/worker → distinct ZIP disk-coordinate and continuous7z writers → original all-candidate/source/readback verification → native per-volume publication/journal/cleanup.

Presets 不分卷、自定义、100MiB、700MiB、2GiB、4GiB and positive integer KiB/MiB/GiB are available. Exact unit switching preserves bytes or retains the prior value/unit with a reason. Unsupported format retains intended split size and blocks Submit until explicit correction. Optional settings default missing unit to MiB and absent preference to no split; opening does not save. Original locked baseline/changed-leaf transaction preserves concurrent untouched preferences. Quick ZIP/7z keeps original no-volume semantics. Original geometry, IME, source/options scrolling and footer remain. Writers, required integrity checks and truthful partial/late-cancel publication are unchanged.

[REF-17 source chain, decisions and evidence](ref17-outcomes.md); [Core persistence contract](../../spec/nxtools-core/backend/database-guidelines.md#scenario-exact-archive-create-split-size-preferences).

### REF-18 — Explicit ordinary numbered-file join

Original Other Tools official180×160 ListItem tile → deferred original window map/BaseRoot/caption → slice-local stable Form inputs and explicit official Dialog → origin-bound original router → original serial reader/detached dispatcher → per-pipe native session/existing bounded STA → original protected source/ordinary fresh-output APIs.

Later-part selection discovers ordered same-directory `.001/.002/...` metadata and previews names/order/count/total/output before explicit confirmation. Detectable gaps/aliases, `.z01`, changed sources/family and source-family numeric output reject; the unknown-final-part limitation remains visible. Metadata is bounded to1024parts/262144UTF-8 name bytes/32GiB total; one64KiB buffer streams content once. Output is fresh no-replace, parents must exist and sources are retained. No content hash, second content scan, whole-family buffering, decoder/repair/conversion or source deletion is added.

Cancel ACK does not release UI pending state, IPC reservation or native work. Original worker/handle finally precedes terminal/resource release. Close waits for actual cancelled+cleanup-complete terminal; late success/cleanup failure stays visible; disconnect reports unknown output. The original reader remains responsive during held transfer. Low-noise related operation/session/generation diagnostics carry accepted/start/state/terminal, elapsed/fixed/native code and cleanup facts, without names/paths/content or per-chunk events.

[REF-18 source chain, real-boundary RED/GREEN and native effects](ref18-outcomes.md); [cross-layer executable contract](../../spec/platformhost/raw-numbered-join.md).

## Initial delivery checks

All test selections were LISTed before identical RUN filters. Counts overlap and are not a distinct-test total. Rust commands used one lane with CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0, RUSTFLAGS=-C codegen-units=256 and scratch TEMP/TMP in the same PowerShell process. LIST uses `-- --list`; Rust RUN uses `-- --test-threads=1` with `--nocapture` where recorded. Native LIST/RUN uses the existing reviewed fixture catalog, never arbitrary system execution.

Log root: `D:\.pi-desktop\data\scratch\0ffe6ffa-9e52-420b-af61-a29fdd0f37b4`.

| Exact selection / command | Observed result | Log names under the absolute root |
| --- | --- | --- |
| `cargo test -p nxtools-core --lib archive` | LIST20 / RUN20 / PASS20 / FAIL0 / ignored0 | `b3-final-core-archive-list.log`, `b3-final-core-archive.log` |
| `cargo test -p nxtools-protocol --lib` | LIST115 / RUN115 / PASS115 / FAIL0 / ignored0 | `b3-final-protocol-list.log`, `b3-final-protocol.log` |
| `cargo test -p nxtools-ipc --lib` | LIST62 / RUN62 / PASS62 / FAIL0 / ignored0 | `b3-final-ipc-list.log`, `b3-final-ipc.log` |
| Desktop `--bin nxtools-desktop ref17_` | LIST5 / RUN5 / PASS5 | `b3-list-ref17_.log`, `b3-final3-ref17_.log` |
| Desktop `--bin nxtools-desktop archive::forms::tests` | final LIST46 / RUN46 / PASS46 | `b3-final-desktop-forms-list.log`, `b3-final-desktop-forms.log` |
| Desktop `--bin nxtools-desktop archive::draft::tests` | LIST12 / RUN12 / PASS12 | `b3-list-archive__draft__tests.log`, `b3-final3-archive__draft__tests.log` |
| Desktop `--bin nxtools-desktop archive_polish_quick` | LIST3 / RUN3 / PASS3 | `b3-list-archive_polish_quick.log`, `b3-final3-archive_polish_quick.log` |
| Desktop `--bin nxtools-desktop tools::polish_tests` | LIST11 / RUN11 / PASS11 | `b3-final-desktop-tools-polish-list.log`, `b3-final-desktop-tools-polish.log` |
| Desktop `--bin nxtools-desktop ref18_` | final LIST9 / RUN9 / PASS9 / FAIL0 / ignored0 | `b3-ref18-ui-final-list.log`, `b3-ref18-ui-final.log` |
| `cargo build -p nxtools-archive --features isolated-codecs --bin nxtools-archive-worker` | matching original worker build passed | `b3-worker-build.log` |
| `dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore` | final native build passed,0warnings/0errors | `b3-ref18-preflight-green-build.log` |
| Original native split catalog: ZIP seven volumes old family; partial volume race facts; B3 explicit split sizes count cancellation | LIST3 / RUN3 / PASS3 / SKIP0 / FAIL0 / not_run0 | `b3-split-native-list-final.log`, `b3-split-native-run.log` |
| Exact split count-limit fixture, same third selection | LIST1 / RUN1 / PASS1 / SKIP0; overlaps native3 | `b3-split-limit-build-list.log`, `b3-split-limit-final.log` |
| `NxTools.PlatformHost.Tests.exe --fixtures "raw numbered join"` | final LIST4 / RUN4 / PASS4 / SKIP0 / FAIL0 / not_run0 | `b3-ref18-preflight-green-list.log`, `b3-ref18-preflight-green.log` |
| `cargo clippy -p nxtools-core -p nxtools-protocol -p nxtools-ipc --lib -- -D warnings` | passed | `b3-final-production-clippy.log` |
| `cargo clippy -p nxtools-core -p nxtools-ipc --all-targets -- -D warnings` | passed | `b3-final-core-ipc-clippy.log` |
| Scoped `rustfmt --edition 2024 --check --config skip_children=true` |21owned Rust files passed; final raw-join file passed after its final edit | `b3-final-rustfmt.log`, `b3-final-raw-join-rustfmt.log` |
| Spec/component/release/editor source gates |129documents/14owners; historical79pages/78mappings with current0.8authority; Kit0.8/pre0.3.9 and two carried patches;57editorfiles | `b3-final-spec-contracts.log`, `b3-final-gpui-component-catalog.log`, `b3-final-gpui-release-contracts.log`, `b3-final-markdown-editor-contracts.log` |
| Trellis context validation |13implement/10check entries passed; existing large-file injection warnings retained; exact relevant source/API windows were read | `b3-final-context.log` |

Representative full log paths:

- `D:\.pi-desktop\data\scratch\0ffe6ffa-9e52-420b-af61-a29fdd0f37b4\b3-ref18-ui-final.log`
- `D:\.pi-desktop\data\scratch\0ffe6ffa-9e52-420b-af61-a29fdd0f37b4\b3-ref18-preflight-green.log`
- `D:\.pi-desktop\data\scratch\0ffe6ffa-9e52-420b-af61-a29fdd0f37b4\b3-split-native-run.log`
- `D:\.pi-desktop\data\scratch\0ffe6ffa-9e52-420b-af61-a29fdd0f37b4\b3-final-rust-results.json`
- `D:\.pi-desktop\data\scratch\0ffe6ffa-9e52-420b-af61-a29fdd0f37b4\b3-final-followup-results.json`
- `D:\.pi-desktop\data\scratch\0ffe6ffa-9e52-420b-af61-a29fdd0f37b4\b3-final-doc-results.json`

Native split fixtures create real ZIP/7z65KiB volumes and use the existing approved stock7Zip reader for byte-exact independent readback. Existing-family/count limits, prepublication cancellation, partial publication and late committed facts remain. Raw native4 proves320123byte later-part join, >2MiB actual64KiB streaming/one pass, writer refusal/source-handle reopen, no-overwrite/source/family preflight, controlled native write112/cleanup, cancellation before/after Commit and original reader responsiveness/physical drain. Outputs are never executed. These are concrete worker/native/readback facts, not a performance or total-process peak-memory claim.

## Real-boundary regressions

REF-17 production format switch erased the intended split size and original save replaced a concurrent untouched Create preference: LIST2/RUN2/FAIL2 in `b3-ref17-red*.log`, then the original form/save boundaries passed.

REF-18 original Windows join failed `raw_join_unavailable`; original router rejected the full-queue operation; same-family output failed its zero-content-IO assertion; actual Discover pointer action left the original queue empty. Their exact RED logs and corresponding native4/IPC62/UI9 GREEN are retained in [REF-18 evidence](ref18-outcomes.md). Compile receipts, DTO-only checks or screenshots are not substituted for behavioral RED.

## Initial delivery paths and resource closure

[Complete B3 path list](changed-paths.txt). Baseline `b3-initial-status.txt`/`b3-initial.diff` remains under the log root. Direct textual comparison found only B3 source/spec tracked sections changed; baseline unrelated code is retained. New B3 files and original task/reference records remain in place; no directory archive/move, reset or repository-wide formatting.

Final background wrapper35207a66-ce1f-4b44-86b6-91db43c08dc0 exited0 after the final9/9 RUN and formatting check; all B3 build/native wrappers are settled. Process inspection found no remaining Cargo/rustc/clippy-driver/linker/Desktop-test/native-fixture/archive-worker process. Native fixtures await real reader/worker/handle drain and exclusive source reopen before return. Resource audit: `D:\.pi-desktop\data\scratch\0ffe6ffa-9e52-420b-af61-a29fdd0f37b4\b3-final-resources.json`.

Current repair closure is recorded separately in [acceptance repair](acceptance-repair.md): REF17 8/8, REF18 11/11, original forms48/draft12/Job129/quick3/tools11, Core REF17 2/Protocol REF18 1/IPC62; native split3/raw4 plus two original Job-export fixtures pass with no skips. Six repair Rust files and four required source gates pass; strict Desktop lint retains only its two unrelated download baseline errors. Current2026-11-10T14:17:22+08:00 audit records0verification processes/0repair wrappers in `b3-repair-final-resources.json`.

## Limitations and retained failures

- Strict Protocol all-target lint remains exit101 at pre-existing `archive_browser.rs:563 unnecessary_map_or`; the exact expression is present in B3's saved initial diff. Strict Desktop bin/tests lint remains exit101 at `downloads/browser_handoff_impl.rs:436 nonminimal_bool` and `downloads/settings_task_rem_tests.rs:612 unwrap_used`. No suppression or unrelated repair is included. Final logs `b3-final-library-clippy.log`, `b3-final-desktop-clippy2.log` preserve the failures; no package-wide strict-lint pass is claimed.
- Generic raw sets cannot prove an unknown final part is present.32GiB/1024part/native path limits are retained.
- Physical IME/DPI/provider/OS-close/hotkey/Shell, human visual, installed/Release and full-workspace acceptance are unperformed. Existing message8/9/source-token/historical-export failures are not recertified or silently cleared. Owned TestPlatform and original-reader/native evidence are reported only at their executed boundaries.
- No actual product/reference/browser launch, packaging/install/deploy replacement, registry/service/power mutation, commit or push occurred. Coordinator acceptance and any next batch remain separate.
