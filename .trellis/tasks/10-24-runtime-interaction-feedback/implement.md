# Execution checklist

Canonical order approved in `.pi/plan/runtime-interaction-feedback-20261024.md`:
1. [x] Establish task/spec/reference/scoped baseline; curate context and record explicit approval.
2. [x] Clipboard R1-R3: actual regressions red/green, fixed-toggle click/target/IME/menuoverlay/previewlateACK verified.
3. [x] R4 human-wait false failure reproduced and repaired; R5 investigation recorded, **unresolved** pending causal native startup/DPI replay.
4. [x] R6-R10: OCRidentifiedsort+actual2DIPred/green, sharedofficialtoolstatus, blacklisttop/reachablefooter, exactFirefoxclick, syntheticproxyABA/manualdraft.
5. [x] Verification recorded in outcomes.md: firstfull1113pass5fail, own4repaired with targeted34+5, nofullrerun; strictlint/workspace/protocol76pass; knownbrowsergatefailure. No deployment.

## Verification policy
Before every build check Cargo/rustc/clippy/MSBuild/relevant tests/non-Unity dotnet (exclude VBCSCompiler); do not kill/contact peers. Exit75 means no build launched. Every Rust command inherits CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 RUSTFLAGS=-C codegen-units=256. Full-operation timeout; inspect survivors before new builds. Scoped rustfmt edition2024 skip_children=true only. C# registrations audited in TestEffects, synthetic data/owned HWND only. No real clipboard/proxy/registry/native user acceptance without separate authorization.

## Risk points
Leased GPUI entity writeback; pending menu retired before source callbacks; delayed preview paint/ACK; pin target after external focus; picker timeout while native still running; startup display units and hidden show; proxy responses after mode/view replacement. No broad refactor/vendor edits. Preserve unrelated test failures and peer source; no full-suite green hunting.

## R5 continuation — current status supersedes step 3's initial investigation result
The later user-authorized repair is source-complete. Live first-show evidence and an actual owned native HWND red/green test identified hidden CW_USEDEFAULT placement bypassed by native restore. One-time hidden preparation now requires exact native client/DPI acknowledgement before publishing restore entrypoints; later user resize/hide/restore is preserved. See `r5-repair-design.md` and `r5-repair-outcomes.md`.

Final compiled verification: native 3/3, startup 6/6, Protocol 77/77, strict Clippy and workspace checks pass. Latest full Desktop run is 1123 passed / 1 unchanged date-test failure; the existing browser literal source gate also remains failing. No full-suite rerun to hunt green. No package, install, service operation, live-window mutation, commit, push or archive; installed reboot and physical mixed-DPI acceptance remain pending.
