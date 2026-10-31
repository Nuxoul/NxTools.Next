# Parent review — audit #1 only

Reviewed 2026-10-25 by parent session `7faedce5-fc3c-4ab5-bf6d-95910fa4f600` after independent delivery from `da6a2ca3-52f0-41f1-a333-3b5548154f0b`.

## Decision

**Accepted for this slice's source, compiled and synthetic GPUI keyboard behavior only.** No blocking defect found in the delivered own-delta. This does not certify the complete Desktop suite, native/physical input, Windows IME, DPI, reader, screenshots, installed product or user acceptance.

The implementer stopped after #1. Parent performed this review and the checks below before authorizing a separate next-item delivery. No recursive reviewer/subagent was used.

## Reviewed boundary

- Read the single-item PRD/outcomes, entire new keyboard regression file, actual production Enter/category/root handlers, and the changed schedule lifecycle/source-gate assertions.
- Received both own-delta texts and the delivery manifest through session communication, rather than reading another session's scratch. They isolate this change from existing dirty work.
- SHA256 of all six delivered files still matched the delivery manifest at review. Production `new_download_view.rs`: `29e313ff5fb08bc43efef363291c48e2f69cbfd014f541a6083b553a0e2e020f`; new keyboard tests: `7c2b43e20fe054049ef8ca37a81e51bbb73b798e88e045fcbf061ebf1715cc29`; existing tests: `c763af9258403886d858aea327c2140e420f9db367ffed3a8a5c537b65089cca`.
- Independently downloaded published gpui-pre 0.3.9 to parent scratch, verified SHA256 `34f61c1cfd36fc9c436af09308c963822ae27a45a84feb21a77ba5c9e051f1c0`, and read `Window::dispatch_key_event` / `finish_dispatch_key_event` / raw capture dispatch. Focused bound Actions run before the fallback raw capture. Category interceptor ownership is marked before focus restoration, preventing the same Enter from becoming an outer submission.
- Confirmed the local handler restricts default Enter to original URL/directory/file-name focus, blocks child layers/composition/modifiers/character-preferred/held/repeated/busy/native gates, and preserves original probe/submit owners. Key-up and activation retire the press latch. No shared helper, dependency, engine, native or layout implementation changed by the slice.
- Existing schedule test now scrolls into the actual viewport before clicking; its original draft/commit/clear/dismiss assertions remain. No production calendar change.

## Parent-executed verification

All direct Rust commands inherited `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256` and ran serially. When Cargo/rustc was occupied, parent waited for natural idle; no other conversation or process was paused, stopped or asked to reserve time.

| Check | Actual result |
| --- | --- |
| `cargo test -p nxtools-desktop new_download_keyboard_ -- --test-threads=1` | PASS: 8 passed, 0 failed, 0 ignored; 4.61s |
| `cargo test -p nxtools-desktop downloads:: -- --test-threads=1` | PASS: 168 passed, 0 failed, 0 ignored; 38.66s |
| `python tests/architecture/gpui-release-contracts.py` | PASS |
| `python tests/architecture/markdown-editor-contracts.py` | PASS |
| `powershell -NoProfile -ExecutionPolicy Bypass -File tests/architecture/download-ui-lifecycle.ps1` | PASS |
| Scoped `git diff --check` | PASS |

Parent logs are in its own scratch as `item01-review-keyboard.log`, `item01-review-downloads.log`, `item01-review-archive-failure.log`, and `item01-review-extension-failure.log`.

The implementer's strict Clippy, workspace check and fmt results are separately reported delivery evidence, not parent reruns. The implementer's pre-fix 1-pass/5-fail behavioral red and its stated fixture limitations remain as recorded in outcomes.md.

## Full-suite failures remain open

Implementer final complete Desktop result remains **FAILED: 1158 passed / 2 failed**. Parent did not rerun the full suite or treat a later isolated pass as an integrated green.

Parent performed exactly one targeted diagnostic run of each failing name, with `RUST_BACKTRACE=1`:

1. `production_password_application_original_dialog_ticket_and_deferred_grant_never_start_ole`: FAILED again at `archive/application_tests.rs:954`, missing official unlock Button; 0 passed / 1 failed. Its setup creates ArchiveApplication/browser only, not NewDownloadView. Stack points to the Archive fixture's missing-bounds assertion.
2. `remediation_usage_settings_opens_editable_settings_directly`: isolated PASS, 1 passed / 0 failed. This does not erase its integrated ExtensionManagerView reentry failure. Its setup creates ExtensionApplication/ExtensionManagerView only, not NewDownloadView.

Association assessment is based on inspected own-delta, unchanged shared initialization/helper code, independently owned test fixtures, and the targeted results, not merely different file names. These failures are not attributed to this local Enter repair; neither is claimed proven pre-existing from a before/after baseline. No unrelated owner was repaired or granted acceptance.

## Limits and next dispatch

No product/PlatformHost/service launch, physical keyboard/IME, real HWND/native handoff, DPI, accessibility reader, screenshots, packaging, installation, real user data, commit or push was performed by parent review.

The next permitted work is a new **single-item** delivery for original audit #9 (directory-picker stale-result identity), not the remaining backlog as a bundle. It must stop for a new parent review before any subsequent item.
