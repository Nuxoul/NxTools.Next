# Download window recurrence — source correction and acceptance boundary

Date: 2026-10-09. Owner: `.trellis/tasks/10-08-sol-window-remediation`. Scope: limited download-window lifecycle repair, not visual redesign or release acceptance. Existing unrelated working-tree changes are preserved; no commit/archive.

## User-visible outcome is still unaccepted

The user screenshot shows the confirmed directory/inspector clipped inside a compact client, with bottom actions absent. The installed fixed Validation five-binary hashes matched the 09:19 package; this was not explained by an old installation. Earlier dimension-only diagnostic projection recorded requested height 278, final height 176, `success=true`, `reason=client_size_settled`.

**The installed package does not contain this follow-up.** There has been no new package/install/product launch or live pointer/probe/release/DPI acceptance. Current automated tests demonstrate corrections at identified native seams, not that the user's screenshot symptom has turned green. A later native shrink after terminal publication is not prevented merely by improving readback diagnostics.

UI standards delivery was documentation, not product polish. No Foundation or Downloads visual refinement was implemented here. The existing canonical sequence remains **Foundation -> Downloads -> Quick Find -> Clipboard + Tools -> Global settings + supporting surfaces**; these tests do not advance it.

## Reference and invariants

Before changing the interaction, rechecked AB Download Manager `AddDownloadPage.kt`, then legacy `D:\Code\NxTools\Views\AddDownloadWindow.axaml.cs` constructor, Open/Closed subscriptions, queued content measurement, stage/error changes and SizeChanged live-center correction. Avalonia native/base implementation remains unavailable; no native reference-source claim.

Pinned GPUI 0.2.2 Windows `events.rs::handle_size_msg` trusts WM_SIZE lParam for logical viewport and rendering. Windows move-loop/window resize boundaries were traced. Its Windows Rust source has no ReplyMessage occurrence, so the current receiver does not deliberately release a synchronous sender before finishing pointer processing. Microsoft documentation was checked directly:

- https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendmessage
- https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendmessagetimeoutw

Preserved contracts: ordinary 176/278 logical-pixel targets; Rust owns state/layout, C# Windows APIs, GPUI rendering; one native resize owner; live user-moved center; no activation/Z-order change; immutable request generation/CAS retirement; fixed two-worker/64-HWND budget; reader/EventWindow enqueue only. No large-window workaround, extra recovery loop or engine-policy change.

## Red evidence and correction

| Actual seam | Pre-fix failure | Correction / regression |
| --- | --- | --- |
| Pending replay retirement on a real HWND | Before-commit restoration produced native height 176 but success=true and pending=false | Capture final native client after commit interleavings, then decide stable/CAS retirement. Replay carries requested and readback dimensions. |
| Native client synchronization on a custom-client HWND | Grow 278 -> shrink 176 -> message pump observed one stale synthetic WM_SIZE | Remove posted requested-size WM_SIZE. Refresh through owner-thread Windows processing of the live client. Same-size notification count guards against deleting synchronization entirely. |
| Pointer-bearing refresh with a delayed HWND receiver | SendMessageTimeoutW returned while the receiver still owned WINDOWPOS; exact regression failed: `client refresh returned while the receiver still owned its WINDOWPOS parameter` | Use SendMessageW and retain the marshalled parameter through receiver return in the existing fixed workers. Delayed receiver safely reads it after release and produces current, non-stale WM_SIZE. |

Terminal recovery publication separately compares the current client with the retired target. A mismatch reports failure rather than success/final176; this is truthful diagnostics, not a new automatic recovery mechanism.

The delayed-receiver test intentionally skips late pointer reads if the old timed sender returned early, instead of provoking undefined memory access. An initial test through outer SetWindowBoundsCentered did not reach the intended refresh condition and was not counted as causal red evidence. A narrow internal test entry now calls the actual refresh function. The temporary HWND's initial client is read, not assumed to equal its requested outer dimensions; refresh asserts unchanged actual client/placement/foreground plus a real current-size notification. Existing integration tests still assert exact production 176/278 targets.

SendMessageW is not a wall-clock timeout guarantee. A permanently hung HWND retains its worker and slot, just like existing uninterruptible SetWindowPos. No unmanaged allocation, leaked buffer or timeout replacement worker was introduced. If both fixed workers hang, work remains bounded/coalesced/rejected; it is not magically recovered. Blocking must never move into the serial reader or EventWindow. A future receiver that uses ReplyMessage early would need a fresh pointer-lifetime design/review.

## GPUI test interpretation

Production summary regression exercises 278 -> 176 -> 278 viewport transitions, pending/guidance/footer and action hit geometry. Pinned GPUI Frame::clear does not clear cached debug_bounds selectors, so an absent-selector assertion was not proof of a newly painted inspector. The speculative Rust production-condition change was withdrawn. Current checks use real viewport/current gate and rendered footer/action geometry, not cached selector absence. No GPUI registry/vendor patch.

## Verification performed

Final native candidate:

- `dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore`: 0 warnings, 0 errors.
- Exact pre-inspected/direct-argv safe window group: **11/11**, then final fixed **three rounds of 11/11**. Includes delayed receiver, current-client same-size delivery, retirement, moved center, fixed workers and delayed native HWND acceptance.
- Exact `old centered replay cannot retire or overwrite a newer request`: **1/1**. Every successful run compares selected labels with actual RUN labels; zero-match or any unexpected case is failure. Both real DOpus cases excluded.
- Download UI lifecycle, UI token boundaries, Quick Launch lifecycle and Browser Native Messaging source gates: all pass. Source gate rejects timed temporary WINDOWPOS and stale posted-size helpers while preserving retirement readback ordering.
- `python tests/architecture/spec-contracts.py`: 87 documents / 12 owners pass. Documentation/source navigation only, not native or UI acceptance.
- `git diff --check`: passes; pre-existing CRLF normalization warnings are not business failures.

Earlier same-follow-up Rust evidence was re-read from the saved log, **not rerun after the final C#-only change**: focused GPUI regression 1/1, full desktop 440/440, strict desktop Clippy, desktop check --tests and cargo fmt check pass. Rust production condition was restored before that verification. No new Cargo run in the final interop correction.

Session scratch evidence: `window-retirement-exact-red.log`, `window-stale-size-red.log`, `window-lifetime-exact-red.log`, `window-lifetime-final-build.log`, `window-lifetime-generation-green.log`, `window-safe-native-final.log` (append-only historical batches; last three rounds are the final 11-case group), `window-lifetime-static-gates.log`, `window-resize-final-rust.log`.

## Operational incident and restrictions

An earlier cmd-quoted test filter was parsed incorrectly, widened the harness, and actually ran the forbidden real DOpus navigation case with PASS. This is unrelated to download behavior. The agent stopped, disclosed the incident and asked; the user authorized continued download-only repair. No claim that the operation had no effect on the user's DOpus window, and no unsolicited navigation to restore it.

Subsequent C# checks preinspect labels, use direct PowerShell argument arrays/test executable, explicitly exclude both real DOpus cases and compare actual RUN sets. No further real DOpus operation, user clipboard read, package/install/product launch or service mutation in this follow-up. Download URLs, names, headers, cookies and credentials are not copied into this report or diagnostics.

Remaining acceptance: separately authorized package/live pressed-pointer probe/release tests at supported DPI must prove matching native client, GPUI viewport, form/inspector/footer and visible actionable controls. UI polish remains unimplemented. Indexer LocalSystem/service release gates are separate, unchanged and unauthorized; no service operation is needed to investigate this download clipping.
