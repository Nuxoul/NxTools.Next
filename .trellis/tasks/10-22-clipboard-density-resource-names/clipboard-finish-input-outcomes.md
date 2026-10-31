# Clipboard feather, shadow and repeated input repair

## Current outcome
Implemented the user-approved14DIP overflow-only body feather, a very small static history-card shadow, primary popup native activation after reveal, and current-state hover admission during keyboard/paste operations. Notification hovered Ctrl+C was explicitly cancelled by the user; no notification dependency/hook/copy implementation was added.

Source and compiled verification passed. This is not physical Windows keyboard/IME, installed paste, mixed-DPI, screenshot or user visual acceptance. No package/install/service/formal product launch, real clipboard/history/config operation, delegation/contact, commit/push/archive was performed. The existing task remains in progress for acceptance; no release claim.

## Reproduced input defects and repair
1. The old NxTools constructor/Open/Activated/PrepareForPopupShow restores search focus; Hide cancels preview before native/base Hide. Current GPUI hidden creation had focus:true/show:false, then native bounds/topmost/visibility ACK, but no explicit native activation. C# SetWindowVisibility uses SW_SHOWNOACTIVATE. A GPUI search FocusHandle cannot make an inactive HWND receive Windows keys.
2. Existing input tests used an already-active test window, so they missed this native boundary. A new real application/result-queue regression failed because reveal success produced no ActivateWindow command.
3. A separate production pointer/event regression failed when hover entered after Ctrl+digit was armed: cancellation removed the old candidate, but the new deferred hover could queue another preview. Direct platform text forwarding from a control already passed and was preserved, not rewritten.

`clipboard/popup_activation.rs` now registers a UUID/session/window/HWND pending record before the existing typed ActivateWindow dispatch after matching reveal ACK. The existing C# adapter owns Windows activation and focus verification; no new native API or C# source change was needed. Successful current completion defers original search focus, rechecking identity/open/editor/overlay state on the next turn. Hide/replacement/disconnect retire pending; failure/dispatch failure/three-second logical timeout clears it and uses the original notification owner with fixed retry guidance. No GPUI activate_window on the hidden prepared HWND, no initial-placement replay and no change to passive preview policy.

Row hover entry now rechecks current panel state in its deferred callback. Armed number gestures, pending original paste/full-item reads, query work or editors cannot schedule a new candidate. Pointer leave and ordinary idle hover retain the existing preview owner. Original keyup-once/Ctrl-first release/repeat/IME/menu/editor/generation/full-by-ID payload and target rules remain.

## Feather and shadow
- One cached passive Markdown document, one original layout. The existing document canvas supplies natural bounds. Only its visible wrapper uses the previously calculated remaining body budget.
- Prepaint compares that same-frame natural bottom against the visible bottom with0.5DIP tolerance. Actual overflow paints at most one14DIP vertical gradient; short/fitting content paints none.
- Fade is confined to the prose column, not source/count/border/feedback/actions. It adds no action/focus/occlusion handler; clicking it still dispatches the full original paste.
- Normal, selected and hover endpoint colors follow the actual official ListItem fill over the original canvas, including alpha. Existing group/group_hover drives hover style; no duplicate hover controller, timer or state. No new palette, dependency patch, parsing, second layout, offscreen mask or text scroll owner.
- Each mounted official card gets one public BoxShadow: black8% alpha, offset(0,1DIP), GPUI sigma1DIP, zero spread, no inset or animation. Card dimensions and external8DIP gap/hit geometry remain.

The actual long-body regression went red with0new quads instead of1. Early color-test failures were observation errors, not palette workarounds: GPUI Style paints fill and border separately, and the border quad has a transparent background; observe the nontransparent filled quad at the exact card bounds. Hsla JSON serializes to8bit hex, so reconstructing it quantizes colors; direct typed Background equality matches the independently observed card fill without relaxing tolerance.

## Performance evidence
The production path has no new animation/timer/frame request. A test-only switch, constant true outside test builds and never persisted, compares the same document/list with both effects off/on. The test alternates order across12samples of8forced Debug draws, retaining exact document identity and row height.

Observed sample:
- Off median:11624.06us per forced draw.
- On median:11803.74us.
- Difference:+179.68us, approximately1.55%.
- Two visible overflowing bodies add exactly two ordinary gradient quads. The shadow adds its own small shadow primitive, not another ordinary fill quad.
- Initial controls had three one-shot mount callbacks; after draining those, both off/on states schedule zero frame callbacks. No new idle loop.

This is bounded synthetic Debug CPU/scene evidence, not GPU timing, presented frame rate, scrolling smoothness, all-hardware assurance or installed acceptance. It supports keeping the shadow small/static rather than removing it for an unmeasured performance concern.

## Commands and results actually run
Every Cargo command inherited CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0, RUSTFLAGS=-C codegen-units=256. Shared-build occupancy was observed/waited, never terminated; exit75 attempts launched no build and are not test results.

| Verification | Result |
| --- | --- |
| New input red |3run:2failed (missing native activation/new hover candidate),1passed (platform text forwarding) |
| Feather red |1run/1failed: expected one overflow gradient, observed zero |
| input_followup_ |4/4PASS, including hide/replace/disconnect/failure/timeout retirement |
| input_repair_ |8/8PASS |
| text_cap_ |4/4PASS |
| style_feedback_ |3/3PASS |
| feather_followup_ behavior |2/2PASS: actual filled-card state matching, no short fade and full-original fade click |
| Static finish cost/idle test |1/1PASS, measured values above |
| Strict Desktop Clippy --tests -- -D warnings |PASS |
| cargo check --workspace --tests |PASS |
| Full Desktop final, serial |1029/1029PASS,0failed/0ignored,146.81s |

Logs in current session scratch: clipboard-followup-input-red.log, clipboard-followup-feather-red.log, clipboard-followup-*-verified.log, clipboard-followup-finish-cost.log and clipboard-followup-final-{clippy,workspace,desktop}.log. Earlier full-run failures remain in the previous ledger as historical evidence. This new full run passed the unchanged Extensions cancellation test; no Extensions repair was performed and it does not retroactively turn the earlier failed run green.

## Bug analysis / prevention
### 1. Root cause category
Cross-layer contract, implicit assumption and test coverage gap: GPUI focus intent/native visibility were mistaken for actual HWND keyboard activation. A one-time preview cancellation was mistaken for protection against later hover admission.
### 2. Why the previous fix was incomplete
The earlier tests exercised active windows and delegated text handlers, not hidden creation through the native nonactivating reveal/result chain. Preview tests seeded an old candidate, cancelled it and stopped, without injecting a new hover while the gesture/paste still owned input.
### 3. Prevention mechanisms
- DONE: real reveal ACK -> exactly one typed primary activation regression, pending-before-dispatch and delayed current search focus.
- DONE: retirement/failure matrix and late-result no-refocus test.
- DONE: armed/pending operation -> new pointer hover cannot queue preview; retain original idle-hover/input tests.
- DONE: executable seven-section Desktop contracts with native visibility/activation and same-frame paint rules.
### 4. Systematic expansion
Quick Launch already has explicit native activation and remains unchanged. Passive Clipboard/Quick Launch/notification surfaces must NOT inherit the primary activation rule. No broad native policy or framework refactor is warranted.
### 5. Knowledge capture
Updated task PRD/design, clipboard reference and Desktop contracts. No template tree or commit was created: those skill defaults are outside this project task's authorization.

## Scope review and remaining acceptance
Direct baseline/diff self-review completed without delegation. This turn changes only clipboard.rs, its new popup_activation module, clipboard_resize_tests.rs, ui/mod.rs, ui/history.rs, ui/markdown_document.rs and ui/efficiency_tests.rs, plus task/spec/reference records. SearchInputFallback/efficiency action logic, preview native presentation, clipboard worker and C#/protocol/vendor sources were not changed by this turn. Parallel Archive/editor/browser source remains untouched.

Remaining: separately authorized package/deployment, actual summon->Ctrl+digit full original paste and direct typing/Windows IME, supported/mixed DPI and visual review of the light shadow/fade. Current installed software was not updated, so these source repairs are not claimed to have already fixed the user's running binary.

## Final static closeout
Scoped rustfmt --check7Rustfiles, scoped whitespace diff, taskJSON/cancellednotificationstate and contextJSONL validation PASS. SHA256 baseline comparison confirms ui/search_input.rs,ui/efficiency.rs,ui/tests.rs and clipboard_preview_presentation.rs unchanged by this continuation.
Fifteen of sixteen static gates PASS (sixPython provenance/docs and ninePowerShell Clipboard/UI/motion/isolation/download/token/consistency gates). `quick-launch-lifecycle.ps1` failed at its broad elevated-navigation condition, precisely `$buildScriptSource -notmatch 'write-release-binary-manifest\.ps1'` atline1762. Read-only diagnosis confirms scripts/build.ps1 is concurrently modified and now delegates publication to publish-release-hosts.ps1; the gate still requires the old literal in build.ps1 itself. No QuickLaunch/native/publishing source or gate was edited to bypass it. Remaining four gates after the fail-fast boundary were explicitly run and passed. This is a retained static verification blocker outside the Clipboard slice, not a failed Clipboard test; fullDesktop1029/1029 and strict/workspace results above remain actual but do not imply allchecksPASS. Log: clipboard-followup-final-static.log; follow-on gate output in sessiontools.
No later product changes or additional builds/fullsuite runs followed this closeout. Task remainsin_progressforactualkeyboard/visual/installedacceptance andtheunrelatedbuildgate; no packaging/install/commit/push/archive.
