# REF-05 — Capture rules and session pause

## Original chain

`background.js` real Chrome/Firefox download events → `interceptBrowserDownload` → existing stable handoff ID → shared automatic decision → original Cookie/request/native receipt/cancel/compensation. Automatic direct/Blob substitutes share admission; explicit drop/menu/Popup retain manual authority. `sanitizeSettings/getSettings/storage.local` and existing `storage.session` remain the only settings owners.

Rules are bounded extension allowlists, minimum-known-size KiB, exact excluded hosts and per-line case-insensitive address substrings. Pause/persistent disable precedes exclusions, which precede type/size. Unknown0/-1 length retains confirmation without extension network work. Session writes precede success publication; recreation restores pause before admission. Read failure retains browser ownership and can recover through another state request. No permission, transport, store, limiter or scheduler is added.

## Behavioral evidence

All logs are under `D:\.pi-desktop\data\scratch\0ffe6ffa-9e52-420b-af61-a29fdd0f37b4`.

| Boundary | Result | Log |
| --- | --- | --- |
| Initial real-event RED | 6 passed / 24 failed; ignored rules transferred and pause message unhandled | `b2-ref05-red.log` |
| Pause metadata RED | Original native frame omitted elapsedMs | `b2-ref05-diagnostics-red.log` |
| Targeted production-event/runtime suite | 42/42 passed | `b2-ref05-boundaries-final.log` |
| Recursive extension suite including B1 | 225/225 passed | `b2-extension-final3.log` |
| Host framed diagnostic route RED | New fixed event rejected by original route | `b2-ref05-host-diagnostics-red.log` |
| Host library including B1 real extension/pipe/SQLite bridge | 48/48 passed | `b2-ref05-host-full.log` |
| Strict all-targets Host Clippy / scoped rustfmt | Passed | `b2-ref05-host-clippy.log`, `b2-ref05-host-format.log` |
| Popup production-script boundary |14/14 included in current full255/255; element/API doubles, not browser DOM | `b2-acceptance-extension-final.log` |

Targeted cases cover Chrome onDeterminingFilename/Firefox onCreated, one-shot suggestions, type and original/final/page exclusions, exact nonmatching hosts, threshold equality, MIME/Content-Disposition, unknown lengths, session pause/resume/recreation/read/write failure and recovery, settings write barrier/failure/retry/reload, automatic substitutes/manual drop, same-ID duplicates and browser ownership. Firefox waits for the current event's new terminal rather than a previous lastHandoff snapshot. Diagnostic failure paths do not open an idle native host.

Popup validates a complete nonnegative minimum within the safe KiB conversion range before sending the original settings request. Invalid input leaves applied intent and the local draft intact. Rule drafts survive failed saves and refresh; a successful reply releases only fields whose original text still matches that submission, including numeric normalization. The original refresh records the page render revision and cannot roll back newer successful settings/pause state. Busy pause rendering remains disabled. Unavailable session state offers a state-read retry without toggling pause. Original change autosave and explicit Save retry remain. Script-boundary RED logs are `b2-ref05-popup-script-red.log`, `b2-ref05-popup-stale-red.log` and `b2-ref05-popup-normalization-red.log`; final13/13 pass.

Commands:

```powershell
node --test .\browser_extension\nxtools-interceptor\tests\capture-rules.behavior.test.js
$testFiles = @(Get-ChildItem '.\browser_extension\nxtools-interceptor\tests' -Recurse -Filter '*.test.js' | ForEach-Object { $_.FullName })
node --test @testFiles
cargo test -p nxtools-browser-host --lib -- --test-threads=1
cargo clippy -p nxtools-browser-host --all-targets -- -D warnings
```

Rust commands used the required single Cargo lane, build jobs1, incremental0, codegen-units256 and scratch TEMP/TMP.

## Diagnostics and privacy

Pause start/applied/failed retain one UUID through extension request construction, native framing, typed Host validation and the original log router. Only fixed reason and elapsed fields are added; arbitrary message/URL/path/Header/Cookie/error text is not logged. Diagnostics do not change storage/admission results or start an idle host. Rule exclusions retain B1 fixed reason/status and read no credentials.

## Verification limits

Production Popup fields/actions passed the coordinator's real isolated DOM22checks, including autosave/invalid/failed drafts, keyboard pause/resume, delayed refresh and retry. [Coordinator acceptance](coordinator-acceptance.md) records the exact executed boundary and retained evidence. B2 is completed at the accepted source/owned-fixture and isolated-browser level. This implementation session performed no browser operation; installed MV3/native/physical/IME/DPI/provider and human visual acceptance remain separate.
