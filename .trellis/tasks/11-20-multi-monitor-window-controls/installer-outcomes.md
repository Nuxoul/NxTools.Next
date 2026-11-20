# R6 installed archive integration — Phase F P206/P207

## Delivered boundary

- Normal Setup calls the hidden, mandatory original-user Desktop entry after trusted preparation and committed file finalization, independent of `/VERYSILENT` or the optional launch. The UUID follows the protected redirect or existing-instance handoff through the generation-bound platform router and original native Apply. Setup waits for the actual terminal disposition: `0` observed Apply/opt-out, `10` applied with `user_choice_locked`, `11` failed or unconfirmed, `12` no proved original user. These outcomes do not roll back package files.
- The dedicated Desktop role uses the existing desktop lock, authenticated pipe and approved PlatformHost. Cold use does not start GPUI or unrelated feature engines. Fresh untouched desired intent uses the supported extensions, cascade/Open With/default claim and recommended menu commands. Upgrades reuse persisted opt-outs, selections and revision. Ordinary launch/reconnect does not apply integration.
- Uninstall waits for the installed PlatformHost's original Revert after owned-process shutdown; PowerShell no longer owns archive Registry mutation or ledger retirement. Revert/replacement record pending choice attempts durably, preserve a different current choice, and require readback before continuing Classes cleanup. A partial write, including delete-then-throw, remains recoverable. Unconfirmed restoration halts uninstall data/file cleanup and preserves evidence. The legacy writer does not remove `UserChoiceLatest`.

## Verification

| Check | Result |
| --- | --- |
| `dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj -- --fixtures 'archive Phase F P206 incomplete default restoration' ...` | The new regression failed on the unchanged service. Final reviewed filters passed 5/5: `incomplete default restoration`, `service set default claims class`, `service enabled apply exact revert`, `product composition explicit apply`, `ledger wire registry target guards` (all prefixed `archive Phase F P206 `). Includes uninstall-native exit mapping over injected effects. |
| `python tests/architecture/installer-uninstall-cleanup.tests.py` | 10/10; fake nonzero native child exit stops the actual cleanup script before browser/personal cleanup, retaining ledger and config. |
| `python tests/architecture/installer-update-lifecycle.py`, `archive-phase-f-shell.py`, `protected-uninstall-entry.py`, `spec-contracts.py` | Passed source/structure gates. |
| Inno 6.7.1 syntax-only `[Code]` compilation with `Output=no`; PowerShell parser on changed scripts | Passed. No Setup binary produced/launched. |
| Scoped `rustfmt --edition 2024 --config skip_children=true` | Passed. |

## Rust verification and acceptance limits

- Core106/106 and IPC73/73 pass, including archive recommendation/opt-out preservation and installed-Apply forwarding. Desktop `installed_archive_apply_distinguishes_terminal_result_warning_and_failure` passes. The final verification report records integrated Desktop failures separately.
- `tests/architecture/installer-runtime.ps1` now provides silent callbacks with an isolated child for `success`, `archive-warning`, `archive-failure`, `archive-wrong-user`. The executable installer fixtures, protected redirect against an installed product, real original-user/elevation behavior and Windows Default Apps/Explorer acceptance were not run. A pre-elevated or service launch without the original interactive user is refused, not redirected into another user's profile.
- No package, install, product launch, live Registry, COM registration, Explorer or service operations were run.
