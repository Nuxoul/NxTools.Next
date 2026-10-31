# Release41 installer — running-product upgrade completed

2026-10-30. The repaired normal formal Release41 installer completed an explicitly authorized upgrade while the existing protected Desktop was running. Installer exit0; process/file preparation, installation, trusted update and finalization completed. This is actual running-product installation evidence, not a package build or an idle-only installation.

## Artifact

- Normal filename:`artifacts/installer/NxTools.Next-setup-0.7.0.exe`;release41/ID`282031ce-891b-4747-8f5b-faf29740983a`.
- Revision:`same-handle-pending-exit1`;26,727,788bytes.
- SHA256:`0605FF2CD0B90E4B134D5EC9273C0051F728C716B5A8E53D0321F1387B5F2137`.
- Helper SHA256:`4CDFEAC77D33715CD6800AD27CDCC3D60F58E05198500FEB8CECAAD36A76FC1A`.
- The77signed program members and both signature sidecars are unchanged. Original RSA3072-PSS/SHA256 verified; no new key/sequence/Cargo or full Rust/C# test invocation. Windows Authenticode remainsNotSigned.

## Actual running upgrade

The existing Next AppId remains installed at `C:\Users\Administrator\AppData\Local\NxTools.Next-clipboard-fix-verification`; that is the pre-existing registered directory. The verified installed38Desktop launched PID19312 and its protected restart PID39488 at `C:\ProgramData\NxTools\NxTools.Next\Service\NxTools.exe`. The running family/path was recorded before installer invocation.

Successful actual correlation:`20261030170301-is-GKXP7M49VC-tmp`.32-bit helper ran elevated and completed process_close/file_preparation with native0. Inno installed files, original updater accepted41 and finalization completed. Installer exit0 in45.92seconds including UAC wait. No postinstall automatic Desktop launch.

Independent readback verifies all77installed members and manifest/signature against formal41, plus13deployed protected files. Original journal/pending retired normally, without manual deletion. Indexer Running/PID42272 after update; updater Running/PID5164 preserved. Other legacy NxTools AppIds were not changed.

Persistent evidence:`.diagnostics/packaging/installer41-pending-exit-20261030-162828/`.
`live-running-upgrade-confirmed/running-proof.json`, `result.json`, `setup.log`, `user-diagnostic/` and root `installed-verification.json` retain actual process identity, helper lifecycle and byte readback. The failed original installer, original user log and cancelled-elevation attempt remain preserved.

## Failure boundary and repair

The reported41correlation`20261030160936-is-LYS9I7BVR4-tmp` reports elevated=true/process_bits32/process_close/native5/native_api=query_image/scriptline237 at the native Stop call. It is not the earlier CIM-null branch or missing elevation. Original query-failure handling checked only `WaitForSingleObject(handle,0)`, so a terminating object whose image query already fails but whose exit signal is pending still produced access denied.

Original native Stop-body regression with query5 + zero-timeout258 + bounded-wait0 goes red. After query/terminate5or31, use the same already acquired handle and original5000ms bound; continue only if actual exit becomes signalled. Real live denial/timeout retains the original native error, wait failure remains an error, and unknown image is never terminated. No retry loop, process-name termination, extra privilege or recovery-authority change.

The candidate query/target/native Stop block now shares its original PID/fixed-name failure attribution. Native Stop failures therefore retain process_id as well as native_api, rather than losing PID after successful CIM discovery. [Reference chain](../../../docs/reference-code/archive-shell.md#installer41-native-image-query-while-closing-a-running-tuple-2026-10-30) and [executable contract](../../spec/platformhost/installer-preparation.md) capture these requirements.

## Focused verification and prevention

- Extended original native process regression:47checks pass on x86/x64, including pending query/terminate5, already exited, live denied, gone, wait failure, exact foreign-image rejection and query-only rights. Same checks pass against exact compiler-input helper bytes.
- Original identity22, upgrade18 and diagnostic8scenarios pass. The new known-image Stop/query5 diagnostic verifies actual PID/fixedname/API; true access denial still exits5.
- Original Inno owned-process and exited-CIM-null callbacks pass with the current helper; real installed running-product upgrade above is the installation acceptance layer.
- The earlier real38installation occurred without a running Desktop and could not close this report. `PACKAGING.md` now requires the actual running state and installed readback for this boundary, not unrelated test totals. Current shared-screen/frontend work and archive tests were not rerun for the installer-only repair.

## Limitations

Original user41log lacks PID, so the precise historical object state cannot be reconstructed. The native pending-exit defect and corrected actual running upgrade are verified separately. Startup readiness uses the retained verified-launch handle/protected descendant rather than requiring CIM image availability; it grants no termination authority. A cancelled UAC returned1223 before helper execution and was not auto-retried; only renewed user authorization allowed the successful installation. Product visual/Explorer interactions and indexer search/idle-CPU performance were not tested by this installer repair. All earlier negative installation records remain immutable.
