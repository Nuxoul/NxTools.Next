# U3 acceptance follow-up — outcomes

## Result

All four accepted U3 gaps are corrected at their existing owners.

| Boundary | Final behavior |
| --- | --- |
| Full native event queue | The current session forwards the latest retained keyboard health when its reader resumes. Queued stale health cannot restore an older state; identical handshake/reader facts are deduplicated. |
| Automatic configuration pending | The existing outlined re-enable Button is loading/disabled for automatic or manual pending configuration, matching retry admission. |
| Fault during hotkey recording | The native recording-stop ACK reevaluates the retained fault. The original Alert and one deferred NotificationCenter notice become available; repeated faults do not restart the same episode. Recovery permits a later episode to notify. |
| Disconnect during re-enable | The pending retry emits one completed diagnostic with its original correlation ID, success=false, failure_kind=disconnected and original elapsed time. A matching result already consumed before disconnect cannot produce a second terminal. |

## Native-event delivery

`LowLevelKeyboardMonitor.HealthSnapshot` remains the immutable latest fact. Its monitor and EventWindow nonblocking TryWrite callback are unchanged from the follow-up baseline. `PlatformPipeClient.ReadEventsWithKeyboardHealthSnapshots` is the production reader projection: before each queued event it samples accepted health, emits a changed snapshot, skips queued health facts, then preserves the ordinary event order. The same snapshot sent after HelloAck/queue discard becomes the initial last-sent fact.

The bounded512slot native queue and256slot outgoing queue remain. The hook never waits for capacity; there is no timer, heartbeat, extra queue or protocol change. The existing serialization/frame-size/clipboard-failure paths remain in ForwardEventsAsync. Snapshot replay preserves native correlation/code and does not create another native operation or lifecycle log.

`KeyboardMonitorTests.HealthSnapshotSurvivesFullQueue` fills the actual512slot capacity, confirms rejected health TryWrite, and drives this production reader. It also covers stale queued failure versus retained recovery, identical handshake deduplication, old-peer capability rejection and ordinary event ordering. Program.cs explicitly registers the test and TestEffects.cs classifies its bounded in-memory effects as Fixture.

## Desktop feedback and diagnostics

The existing QuickLaunchApplication owns the busy projection, notification episode and pending retry identity. The released Kit0.8.0 Small Warning Alert and standalone Medium/32DIP outlined Button remain in the original Settings summon row; the corrected busy value feeds its existing loading/disabled controls. Notification mutation stays deferred to the next application turn. Command configuration, recording, health revision and matching-result owners are retained.

The disconnected terminal is `quick_launch.keyboard_reenable.completed`, using the original ID and start clock. Disconnection has no native error code. It logs fixed metadata only. Production TestApp regressions inspect the actual Button, click path, Alert, notice generation and captured terminal events. Exact Rust commands and owner details: [followup-rust-outcomes.md](followup-rust-outcomes.md).

## Verification

| Check | Follow-up result |
| --- | --- |
| C# test-project build --no-restore | 0 warnings, 0 errors |
| Reviewed affected C# fixtures | 18 passed, 0 skipped, 0 failed |
| Desktop quick_launch:: | 173 passed |
| Desktop main_window::tests::quick_launch | 11 passed |
| Desktop hotkey_reset | 3 passed |
| New Rust owner / real Settings filters | 3/3 and1/1; included in the Desktop groups above |
| Strict protocol+Desktop Clippy --tests -- -D warnings | Passed |
| Changed Rust files' scoped rustfmt | Passed |
| Spec, component catalog, GPUI release and Markdown editor gates | Passed |
| Quick Launch lifecycle and git diff --check | Passed |

The nonoverlapping follow-up groups total205tests. Five new regressions are included in those totals. C# uses bounded/fake native boundaries; Rust runs offline with CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0, RUSTFLAGS=-C codegen-units=256, desktop test debug=0 and one test thread.

Behavior-red evidence reached the intended assertions before the fixes: the full-queue reader omitted retained health; the official Button was not disabled during automatic configuration; recording-stop ACK produced zero notices instead of one; disconnect produced zero correlated terminals instead of one. The final fixtures pass, including completed-before-disconnect deduplication.

### C# commands and logs

```powershell
$testProject = 'csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj'
$filters = @(
    'keyboard monitor fixture',
    'extension runtime fixture detached reader responsiveness',
    'quick-launch configuration', 'Quick Launch ', 'double-control gesture',
    'paste retries transient', 'paste diagnostics preserve',
    'pressed modifiers keep', 'incomplete input keeps',
    'paste result correlates', 'unverified fallback'
)
dotnet build $testProject --no-restore
dotnet run --project $testProject --no-build --no-restore -- --fixtures --list @filters
dotnet run --project $testProject --no-build --no-restore -- --fixtures @filters
```

Session scratch logs: `u3-followup-csharp-build-red.log`, `u3-followup-csharp-list-red.log`, `u3-followup-csharp-red.log`, `u3-followup-csharp-build-green.log`, `u3-followup-csharp-list-green.log`, `u3-followup-csharp-green.log`; the baseline comparison is `u3-followup-csharp-source-diff.log`. Integrated static checks are recorded in `u3-followup-final-gates.log` and `u3-followup-diff-check.log`.

## Acceptance boundary

No remaining implementation or selected-check blocker. Physical keyboard, installed global Hook, real clipboard delayed rendering/paste, UAC/service, real HWND/DPI, accessibility-provider and user visual acceptance remain separate. Windows silent hook removal is not detected. The earlier download-calendar and Archive-format cases remain outside this slice, as recorded in [outcomes.md](outcomes.md).

Unrelated working-tree changes are retained. This follow-up does not alter the extension, protocol shape or IPC source. No product launch, packaging, installation or commit was performed.
