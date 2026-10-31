# Parent review — audit #9 only

Reviewed 2026-10-25 by parent session `7faedce5-fc3c-4ab5-bf6d-95910fa4f600` after single-item delivery from `da6a2ca3-52f0-41f1-a333-3b5548154f0b`.

## Decision

**Accepted for this slice's source, compiled and synthetic application/window/input/typed-result behavior only.** No blocking defect found during the one directed parent review. This is not full Desktop, native picker, real HWND/modal lifetime, physical keyboard/IME, DPI, reader, screenshot, installed-product or user acceptance.

The implementation conversation stopped after this item; parent independently reviewed and verified before dispatching any subsequent item. No reviewer subagent or recursive delegation was used.

## Scope and evidence reviewed

- Read the item PRD/outcomes, complete 299-line private picker implementation and all 854 lines / 14 production-boundary tests, the original application dispatch/notification/disconnect/window-close paths, both view request/Change subscriptions, and updated existing Apply/Cancel test.
- Received complete own-delta texts for existing source files through session communication; did not assume access to another conversation's scratch. All nine delivered file hashes matched its manifest before parent verification. `downloads.rs` matched `badd97e84410b64e9b6b155ebce6d94615fd0b29eb2ee7d8bc2408a168a7499f`; picker implementation matched `85c4d20a2a5cdd929563359e6e594f832635cba883a32b18440743caa3a91adb`; picker tests matched `7e5ab2de7cbea0d829ced2bc73e0ee72360a90791052ed2f70e9c7c3b377f7ab`.
- The private module adds methods to the existing DownloadApplication, not a new entity/controller. One pending slot remains. Original view/window + local operation UUID + directory Change revision and current value replace target-type broadcasting.
- Local token and application pending are registered before correlated dispatch. Result claim and final application revalidate identity/draft; duplicate or old IDs cannot take a newer pending. Success applies once, does not save, and retains NewTask explicit-directory and original validation policy.
- Read the independently hash-verified gpui-pre0.3.9 `App::push_effect` / `flush_effects` source in parent scratch: effects enqueue at the back and are consumed from the front. The two defers let already-queued Change effects settle before final checks; same-update result-before-ABA and ABA-before-result cases have actual InputState/typed-result regressions.
- Reviewed close/removal/registered native-close identity, fixed-confirmation replacement, disconnected/send-failure/cancel paths, weak-owner/token release and deferred cross-owner cleanup. Old close notifications cannot clear replacement window/pending; no native cancellation/host termination was introduced.
- Original C#/protocol/IPC/engine/shared controls, geometry and persistence policy are not modified by the own-delta. Accepted audit #1 keyboard handler/captures are unchanged. The long confirmed-Form diff is local rustfmt expansion surrounding the picker callback change, not a new layout.
- Existing tests replace artificial DirectoryPicked broadcast injections with a real origin request and typed result. Original Apply/Cancel assertions remain; actual Settings/NewTask separation is covered by the new production-boundary test.
- No added payload logging: lifecycle diagnostics use IDs, target kinds, reasons, elapsed and native-code presence/code. Paths/current values are request state/IPC payload, not diagnostic fields.

## Parent-executed checks

All Rust commands inherited `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`, ran serially after checking for natural idle, and did not interfere with other conversations/processes.

| Check | Actual result |
| --- | --- |
| `cargo test -p nxtools-desktop -- --test-threads=1 downloads::directory_picker_tests:: new_download_keyboard_` | PASS: 22 passed / 0 failed / 0 ignored, 9.27s; 14 picker plus 8 accepted keyboard regressions |
| `cargo test -p nxtools-desktop downloads:: -- --test-threads=1` | PASS: 182 passed / 0 failed / 0 ignored, 44.31s; final delivered source |
| `cargo clippy -p nxtools-desktop --tests -- -D warnings` | PASS |
| GPUI component catalog / release / Markdown editor / spec Python gates | PASS |
| Download UI / taskbar overlay / UI token PowerShell gates | PASS |
| Scoped `git diff --check` | PASS |

Parent logs: `item09-review-focused.log`, `item09-review-downloads.log`, `item09-review-clippy.log` in parent scratch. Implementer red RUN5 (1 pass / 4 fail), workspace check, fmt and its full integration remain separately identified delivery evidence, not parent reruns.

## Integration status and limits

Implementer's one final full Desktop run remains **FAILED: 1173 passed / 1 failed / 0 ignored**, exit101. Remaining failure: `archive::application::tests::production_password_application_original_dialog_ticket_and_deferred_grant_never_start_ole`, `archive/application_tests.rs:954`, missing official unlock Button. Parent had already independently reproduced this exact failure during #1 review; that Archive-only fixture does not instantiate the modified DownloadApplication/views. No Archive code is in this own-delta. This item neither repairs nor grants acceptance to that owner. No additional full-suite run or repeated green hunting was performed by parent. Extension passing in this delivery does not prove its previous integrated reentry issue was repaired.

No native picker/PlatformHost/product/service launch, real user data/config/registry operation, physical input, real HWND handoff/modal/focus acceptance, DPI, reader, screenshot, Release/package/install, commit or push occurred in this review.

## Sequential handoff

Audit #9 is accepted only within the evidence levels above. The next authorized work requires a new single-item delivery for original audit #10 (Clipboard stable domain-based row/focus/action identity). No other backlog item is authorized, and that next item must stop for another parent review before further dispatch.
