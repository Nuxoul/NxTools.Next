# Elevated foreground input

## Delivered

NxTools.exe and ordinary PlatformHost remain unprivileged. The existing authenticated same-session elevated directory helper also owns administrator-foreground double Ctrl and input-only paste. No new executable/service role, manifest/uiAccess, public Desktop/Indexer protocol or Rust change.

- Helper callbacks only detect/post. Its hidden message window checks foreground elevation/session and authenticated destination ownership before forwarding. Ordinary/helper messages partition by confirmed ownership and deduplicate the exact native key timestamp, including after summon changes foreground.
- Enabled/recording/trigger/reset configuration coalesces and replays after authentication. PID/generation-qualified failures dominate late configuration ACKs. Hook/window/process exit clears ownership; connected-channel failures share a finite automatic recovery budget. EOF/disposal retires hook, hidden HWND and directory context owners.
- Ordinary clipboard writing remains once-only. Copy-only and write failures never delegate. Elevated input receives only target/focus HWND and command correlation; it validates same-session elevated target/focus ancestry and reuses original activation/modifier/input operations. Foreground is rechecked before Ctrl+V. Unconfirmed WM_PASTE and dispatched unknown outcomes remain failures; unknown input is never replayed.
- Helper absence before dispatch preserves the ordinary path. Existing protected-copy/linked-token prerequisites and bounded rendezvous diagnostics remain. Input lifecycle diagnostics retain correlation, role, state, elapsed and native-error metadata without clipboard content or key streams.

## Verification

| Check | Result |
| --- | --- |
| `dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore` | Pass; 0 warnings, 0 errors |
| Reviewed fixture LIST | 31 selected, 0 executed |
| Same fixture RUN | 31 passed, 0 skipped, 0 failed |
| `python tests/architecture/spec-contracts.py` | Pass; 129 documents, 14 workspace owners |
| `tests/architecture/platformhost-test-selection.ps1` | Pass; source-only fixture/native selection boundary |
| Scoped `git diff --check` | Pass |

Exact fixture filter union: `elevated input fixture`, `keyboard monitor fixture`, `elevated directory helper request`, `elevated helper`, `double-control gesture`, `paste `, `both triggers`, `Quick Launch WndProc`. Both LIST and RUN use `--fixtures`; no native authorization switch.

Ten new entries cover foreground loss, once-only clipboard/input ownership, source duplication, configuration replay/process exit, shared paste/directory channel and dispatched timeout, unavailable fallback, message-thread routing/owned HWND cleanup, target/destination validation, failure-before-ACK and bounded recovery. Existing keyboard, fullscreen, paste and directory protocol/session fixtures remain green. The foreground-loss and failure-before-ACK regressions each fail against their prior behavior and pass with the delivered implementation. Evidence is in `evidence/`.

One bounded correctness review found no additional concrete defect beyond the covered ACK ownership case.

## Native acceptance remaining

Protected-install/linked-token readiness, actual administrator-foreground double Ctrl, real activation/paste across text/files/image/native representations, recording/configuration switching and actual helper teardown require a separately authorized installed Windows run. Synthetic pipes, fake hook/input operations and self-owned hidden HWNDs do not establish those results.

No packaging, installation, service operation, product launch, real global keyboard hook, system clipboard mutation, administrator-window interaction or commit was performed. Unrelated workspace changes are preserved.
