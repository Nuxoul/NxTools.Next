# Floating assistant implementation plan

Status: implementation approved by the user on2026-10-23; task activated. Canonical phase titles and order below remain fixed. Research/fixture stabilization stays in its current phase rather than renumbering later work.

## Phase A — Native receiving feasibility
- [x] Refresh dirty baseline, owning specs and exact current sources. Preserve all unrelated work; no implicit commit/archive.
- [x] Define protocol/session/correlation/native diagnostic events and bounded admission before product integration; partial boundary spec and checkpoint record limitations.
- [x] Add a red-capable self-owned multi-process fixture for the actual C# native receiving boundary, not just format-parser helpers. Foreign-renderer invisible-corner red/green recorded in phase-a-checkpoint.md.
- [x] Prototype one C# STA/OLE input layer for the hidden rendering surface, with no vendor patch/injection/global hooks/cross-process SetParent. Actual GPUI integration remains after the OLE prerequisite.
- [x] Owned native receiving, passive focus/hit/circle/foreign renderer and real Windows OLE source-to-target route passed; actual GPUI/DPI/user acceptance remains separate.
- [x] Classify safe/native effects; user separately authorized exact owned OLE route. Targeted13safe+1nativepassed.
- [x] Record Phase A results and carry actual-GPUI integration limits into Phase B.
- Current phase: **Phase D — Integrated verification and handoff**. Settings compilation blocker resolved; see current implementation-checkpoint.md for compiled/native results and remaining acceptance limits.

## Phase B — Settings, controller and media
- [x] Added red-capable Core section/default/roundtrip/stale-merge/failure tests, production tile/configure tests and controller hidden/reveal/retire tests.
- [x] Add dedicated serde-default settings/enums and atomic section update;6new tests/fullCore82pass and strictCore/ProtocolClippy pass.
- [x] Compose one optional controller/runtime binding and hidden renderer/input-layer lifecycle, with pending-before-dispatch, current HWND/session/generation and deferred effects.
- [x] Add official180x160 tile and existing460x320 feature shell using real Form/Field,32DIP NumberInput/Select and controlled Switch. Stable inputs, one scroll owner and visible validation/error state; production control/persistence/last-scroll regressions pass.
- [x] Enable only required codecs on existing pinned image version in desktop; Cargo.lock unchanged. Bounded off-render decode, generation cancellation and one monotonic frame scheduler have compiled regression coverage.
- [x] Implement default ball/custom image/GIF, size/FPS and window preferences; no hidden6FPS cap. Native readbacks distinguish saved intent from applied state. Static/hidden/disabled/reduced playback schedules no frames.
- Previous five Desktop settings compile errors are resolved; final focused41pass and workspacecheckpass. Earlier fullDesktop1088pass is superseded by final1091pass/1unrelatedarchivefail. Actual GPUI opacity, physical pointer/DPI and user visual acceptance remain in Phase D.

## Phase C — Existing-owner actions and download synchronization
- [x] Add red-capable action/router tests: exact click choices/None, native drag/capture/lock, stale/duplicate drop, text/image selection and unsupported source; real adapters and copy command/lifetime covered.
- [x] Expose bounded immutable DownloadApplication projection from guarded live tasks/latest_progress. Filter current Downloading tasks for speed; preserve count/weighted known bytes and unknown progress.
- [x] Implement approved0.25..4x mapping,1MiB/s1x,0speed pause, no-downloading baseline and sync-off; no engine scheduling changes or per-byte/per-frame diagnostics.
- [x] Reuse ordinary Add Download prefill/probe/confirmation (click opens blank); never silently submit.
- [x] Add internal Quick Launch summon-with-query preserving original session/query-generation/input/target lifecycle; stale prefill cannot overwrite live edits.
- [x] Reuse main restore commands with matching visibility->activation acknowledgement and native copy-only image write. Bounded pending, once-only drop, cancellation-before-dispatch and normalized-image ownership through native result/deadline covered.
- [x] Integrate edge/fullscreen/position persistence, explicit edge_hidden intent, display-signature recovery and click-through cancellation/disable/recovery across renderer/input layer. Owned HWND/controller checks pass; physical mixed-DPI and installed acceptance remain separate.

## Phase D — Integrated verification and handoff
- [x] Run focused owner tests and source/lifecycle gates, fix in-scope defects and retest. Final focused41/Core82/Protocol74pass; finalDesktop1091pass/1archivefail. Quick Launch source gate still fails unrelated elevated-navigation contract; do not hide either failure.
- [x] Run scoped strict lint/workspace/format: workspace and task-owned fmt pass; latest global strictlint/fmt are blocked only by concurrent archive changes (details in checkpoint). Earlier whole-worktree strict/fmt passed before those changes.
- [ ] Complete actual production visual/keyboard/pointer/LightDark/mixed-DPI acceptance; headless controls/bounds/scroll/cancellation are covered, not full physical/user acceptance.
- [x] One optional bounded read-only review attempted and cancelled after prolonged no-report; not a passed review. No second broad review. Concrete parent cache/cancellation findings fixed with regressions.
- [x] Update reference/owning specs, current checkpoint and diagnostic limits; final doc/source/diff checks, no DEBUG markers. Late decode-disable/re-enable cache regression passes.
- [x] Report source/compiled/owned HWND separately from physical/installed/user acceptance and external blockers. No package/install/service/normal product launch/commit/push performed.

## Regression matrix
| Acceptance | Required production seam |
| --- | --- |
| A1,A7 | Real catalog pointer/Enter/Space, Configure no-bubble/keyed window; actual Form/Field descendants/keyboard/last-field scroll within460x320; no unrelated saves. |
| A2,A6,A8 | Actual native receiver/renderer and acknowledged readback; drag-capture terminal, lock click, edge/fullscreen/dpi/monitor change, pass-through to owned third target, old phase result/disconnect/source destruction. |
| A3 | Real decoder GIF disposal/transparency/zero-delay/oversize/corrupt/stale fixture; deterministic monotonic scheduler0.25/1/4x/0speed/no-timer; live aggregate rejects stale/paused/completed rates. |
| A4,A5 | Production C# format admission->typed event->Rust classification->deferred existing-owner action; pending/once-only/retired session; exact Add Download/Search/Image copy commands and original full data. |
| A9 | Core/Protocol/Desktop suites, C# audited fixture run/pass counts, scoped lint/check/fmt and all relevant source gates below; unperformed native/visual clearly listed. |

## Verification commands
Planning/document gate only (run before final review):
```powershell
python tests/architecture/spec-contracts.py
python tests/architecture/gpui-component-catalog.py
python tests/architecture/gpui-release-contracts.py
python tests/architecture/markdown-editor-contracts.py
python .trellis/scripts/task.py validate .trellis/tasks/10-23-floating-assistant
git diff --check -- docs/reference-code/assistant.md docs/reference-code/README.md .trellis/tasks/10-23-floating-assistant
```

After implementation, every direct/background Rust invocation inherits:
```powershell
$env:CARGO_BUILD_JOBS='1'
$env:CARGO_INCREMENTAL='0'
$env:RUSTFLAGS='-C codegen-units=256'
cargo test -p nxtools-core --lib
cargo test -p nxtools-protocol --lib
cargo test -p nxtools-desktop -- --test-threads=1
cargo clippy -p nxtools-core -p nxtools-protocol -p nxtools-desktop --tests -- -D warnings
cargo check --workspace --tests
cargo fmt --all -- --check
```
One Cargo process only, full-operation timeout and scratch log. After timeout inspect surviving cargo/rustc and never start a duplicate. Current memory-pressure refusal, if any, forbids Cargo until separately authorized. Existing unrelated formatting must be reported, not globally rewritten.

C# (only classified audited safe fixtures):
```powershell
dotnet build csharp/NxTools.PlatformHost/NxTools.PlatformHost.csproj --no-restore
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore -- --list
# Exact new fixture names selected only after they exist/classification is reviewed.
# Then one --fixtures full audited set; native/restricted needs separate authorization.
```

Source gates after changed owner integration:
```powershell
powershell -NoProfile -File tests/architecture/download-ui-lifecycle.ps1
powershell -NoProfile -File tests/architecture/quick-launch-lifecycle.ps1
powershell -NoProfile -File tests/architecture/desktop-shell-lifecycle.ps1
powershell -NoProfile -File tests/architecture/ui-state-lifecycle.ps1
powershell -NoProfile -File tests/architecture/ui-motion-policy.ps1
powershell -NoProfile -File tests/architecture/ui-consistency.ps1
# Plus new tests/architecture/floating-assistant-lifecycle.py at the actual source boundary.
# Always rerun the4planning documentation/provenance gates above.
```
Do not treat source-token tests as physical native behavior or visual acceptance. Tests operate on temporary synthetic data, not the user's real clipboard/history/settings/network/service.
