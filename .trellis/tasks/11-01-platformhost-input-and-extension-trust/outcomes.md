# PlatformHost input isolation and extension gesture trust — outcomes

## Result

H5, H4, M1, U3 and U5 implementation and the four U3 acceptance follow-up corrections are complete with their recorded checks. Follow-up results: [followup-outcomes.md](followup-outcomes.md). Existing unrelated work is retained, including the task-entry videoPosterUrl/pageThumbnailUrl content-script changes. No commit, packaging, installation or product launch was performed.

## H5: Detached process and service commands

The serial command reader previously awaited launch_process and ensure_indexer_service, including synchronous ShellExecute/UAC work before a Task could be returned. Both names now use PlatformCommandSequenceDispatcher's original detached task tracking. WindowsPlatformAdapter.LaunchProcess disposes the returned Process handle without terminating the application or changing the command result.

The existing named indexer installation semaphore still serializes installers. Its acquisition deadline is30seconds; TrustedUpdateBroker retains its5minute request/10second connection budgets. Rust records its bootstrap command identity before dispatch and matches that result, without depending on independent later commands waiting behind it. DrainAsync tracks these tasks within the original2second teardown budget; it does not force an uncooperative UAC/native operation to terminate.

Implementation and source-traced ordering: [h5-outcomes.md](h5-outcomes.md).

## H4/U3: Dedicated native owner and recovery UI

The low-level hook previously shared EventWindow's clipboard/paste STA. LowLevelKeyboardMonitor now owns one background thread for installation, blocking GetMessage, callback delivery and final uninstallation. Dispose posts WM_QUIT and joins within2seconds. The delegate remains rooted, and a retry cannot replace a still-live prior thread.

QuickLaunchKeyboardGesture exclusively owns both original double-tap detectors on that thread. EventWindow publishes an immutable enabled/recording/trigger/reset-generation snapshot through Volatile; the next callback applies reset. Callbacks filter injected input, update local detectors and post the original gesture to the captured HWND, with no IO/logging/lock wait. Existing foreground/fullscreen, placement, cancellation, IPC and focus owners remain unchanged. ConfigureQuickLaunch retries unavailable double Ctrl through the original command path.

The typed quick_launch_keyboard_health_changed event reports install_failed/thread_exited/reinstalled with one correlation ID and the native error code. Outer protocol remains1; quick_launch_keyboard_health_v1 is opt-in through HelloAck because an unknown PlatformEvent would otherwise end an old Rust receiver's session. Hello retains the existing current-availability capability; after acknowledgement/queue discard, an accepted peer receives the current immutable health fact. Normal forwarding also projects the latest snapshot, so full native-event backpressure cannot lose the current-session state after reading resumes.

QuickLaunchApplication owns health, one deferred NotificationCenter notice per unavailable episode and re-enable pending identity. Its existing summon settings row uses official Kit0.8.0 Small Warning Alert and a standalone Medium/32DIP outlined Button. Automatic and manual pending configurations both make retry busy/disabled. Recording-stop ACK reevaluates a retained fault and notifies once when relevant. Re-enable installs its ID before dispatch, refuses duplicates and defers cross-entity work. Health revisions keep newer native failure authoritative over older configuration results. Matching recovery removes the Alert; disconnect closes pending retry with its original correlated terminal, and reconnect reads fresh Hello capabilities. No second configuration, save or notification owner was added.

Native/thread evidence and ReleaseActivationModifiers: [h4-outcomes.md](h4-outcomes.md). Rust protocol/UI evidence and exact commands: [rust-outcomes.md](rust-outcomes.md).

## U3 acceptance follow-up verification

All four accepted gaps have behavior-red evidence followed by passing regressions. Current follow-up groups: C#18, Desktop Quick Launch173, Settings11 and hotkey reset3, totaling205nonoverlapping tests. New regressions are included, not added again. Strict Clippy, changed-file rustfmt, spec/component/GPUI/Markdown and Quick Launch lifecycle/diff gates pass. Exact commands, counts and boundaries: [followup-outcomes.md](followup-outcomes.md).

## ReleaseActivationModifiers conclusion

EventWindow.DrainPasteRequests → WindowsPasteCoordinator.Execute → IWindowsPasteOperations.ReleaseActivationModifiers → NativeMethods.ReleaseActivationModifiers sends modifier-up input, then sleeps3ms for up to12attempts without pumping EventWindow messages. Injected-event filtering happens only after hook delivery. The dedicated hook loop removes that shared-thread self-wait dependency. The source/isolated fixture evidence does not establish whether the hypothesized failure occurs with a physical keyboard or real paste target.

Reference inspection followed Listary's managed constructor/base/show/hide/foreground boundary, then SwiftList's detector/hook, first-party hotkey documentation and real QuickSearchWindowController show/hide/focus boundary. Listary native hook source was unavailable. Verified call chains and preserved invariants are recorded in [quick-launch-research.md](../../../docs/reference-code/quick-launch-research.md).

## M1/U5: Trusted content gestures

The direct HTTP(S) link, drop/drag-over and two download-producing media-overlay click entries now return immediately for untrusted events. Synthetic gestures send no download message and do not consume the browser default. Script-triggered ordinary downloads therefore use native browser download/onDeterminingFilename handling; the marked synthetic browser replay remains functional. No UI, permissions, background router or page-hook behavior changed.

Cookie lookup is conditional on includeCookies and absence of an already captured Cookie header. chrome.cookies.getAll({url}) serializes URL-matching returned cookies without filtering HttpOnly or SameSite=Strict, including the existing context/referrer and YouTube-specific fallback queries. Cookie contents were neither read live nor logged.

The manual drop target remains intentionally independent of automatic interception and per-site disablement, matching legacy/current behavior. page-hook.js remains page-realm Blob instrumentation accepting programmatic/click paths under its existing Blob/capability/policy checks, not a user-gesture trust boundary.

Detailed findings, exact poster/thumbnail preservation comparison and extension commands: [extension-outcomes.md](extension-outcomes.md).

## Initial delivery verification

| Check | Final result |
| --- | --- |
| C# test-project build --no-restore |0warnings,0errors |
| Reviewed affected C# fixtures |17passed,0skipped,0failed |
| cargo test -p nxtools-protocol |101passed:99unit+2integration |
| cargo test -p nxtools-ipc |45passed |
| Desktop quick_launch:: |170passed |
| Desktop main_window::tests::quick_launch |10passed |
| Desktop hotkey_reset_tests |3passed |
| New Desktop keyboard_health_ filter |4passed; included in the Desktop groups above |
| Strict protocol+Desktop Clippy --tests -- -D warnings |passed |
| Scoped rustfmt on7changed Rust files |passed |
| All11extension *.test.js files |65passed |
| Real DOM content/refine/layout/redesign |27/27,20/20,38/38,34/34 |
| Spec/component catalog/GPUI release/Markdown editor gates |passed |
| Quick Launch lifecycle and git diff --check |passed |

Desktop commands used the required profile.test.package.nxtools-desktop.debug=0 override and --test-threads=1. Rust invocations set CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0, RUSTFLAGS=-C codegen-units=256 and CARGO_NET_OFFLINE=true.

Behavior-red evidence: H5 ran1/pass0/fail1 when launch blocked the reader, then passed; H4 ownership ran1/pass0/fail1 on caller-thread installation, then passed. The post-ack snapshot regression likewise failed its intended assertion before publication and passed afterward. Extension real DOM was23/27 before the guards and27/27 afterward. Rust ordering regression reached and failed the new-thread-exit preservation assertion before the revision condition, then the final health filter passed4/4. Full command/log indices are in the linked owner outcomes.

## Limitations and known unrelated cases

- Isolated native-queue and GPUI/TestApp fixtures are not physical keyboard, real clipboard delayed rendering/paste, installed hook, UAC/service, real HWND/DPI, accessibility-provider or user visual acceptance.
- Windows silent hook removal remains unobservable; no heartbeat, polling or heuristic detector is claimed.
- DOM verification used temporary headless Chrome profiles, mocked extension APIs and blocked networking. Trusted evidence is CDP mouse input, not physical drag/drop, a real profile, live cookies or Native Messaging acceptance.
- The known date-dependent downloads calendar case was outside the selected tests and is not counted passed. Existing Archive workspace-format differences were left unchanged; formatting verification was scoped to changed Rust files.
