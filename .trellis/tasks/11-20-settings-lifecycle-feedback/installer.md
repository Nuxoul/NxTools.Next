# Installation authorization

## Reference and boundary

Legacy NxTools installer uses LocalAppData Programs, lowest privileges and Restart Manager; its main-window X can hide to tray while explicit Exit ends the application. Next's normal Setup also defaults to lowest privileges, with selectable scope. Maintenance Setup, initial LocalSystem indexer/update enrollment and protected-directory/SCM writes are independent privileged operations. Original signed broker owns enrolled protected upgrades. Application exit releases owned runtime resources but does not remove machine authority requirements or service registration.

## Ordinary preparation

`installer/nxtools-next.iss::PrepareToInstall` executes ordinary preparation as the current user. Existing service registration does not request administrator. Actual helper access-denied exit5 retains the original explicit runas fallback; explicit same-release Bootstrap maintenance/repair/rotation retains its authorization. Cancelled elevation never allows file replacement. The existing helper still verifies exact paths/process identity and transaction/recovery authority. Bootstrap migration is invoked once per preparation with both original repair/rotation switches.

The installer currently closes verified targets through its existing same-handle termination/wait boundary; this slice does not replace it with a new cooperative cross-process shutdown protocol. Tray Exit is the current explicit complete Desktop-exit action; an ordinary X can hide. PlatformHost supervisor and browser Native Messaging ownership remain distinct, and the indexer is service-owned. Protected writes, first enrollment, actual access denial and explicit maintenance may still need authorization. Routine preparation no longer elevates merely because an indexer registration exists.

## Verification

`python -B tests/architecture/installer-update-lifecycle.py` passed the source-only authorization/transaction/dependency/original-user gates. The registration-presence assertion reproduced the old routing before repair. PowerShell Parser.ParseFile accepted close-owned-processes.ps1 and installer-runtime.ps1 without executing either. The original isolated Inno fixture now expects ordinary preparation success without elevation, and genuine access-denied fallback with cancellation/failure preventing replacement. No installer compilation/execution, product termination, Registry/service mutation or real running-upgrade acceptance was performed.
