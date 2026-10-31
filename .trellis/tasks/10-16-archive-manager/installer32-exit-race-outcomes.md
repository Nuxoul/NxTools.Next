# Installer32 recurrent error1 / native31 — matching exited-process trigger repaired

2026-10-28 user actual08:57 installation failed again. The previously delivered32package is FAILED in the user's environment; prior build/tests did not prove installability. Actual user JSONL:32bit process_close, MethodInvocationException -> Win32Exception31, scriptline185 (NxToolsUpgradeProcess.Stop), before file_preparation. No actual failure PID/API substage state was captured, so exact historical process state cannot be reconstructed.

## Real matching repro, not a generic return-code stub

Original helper SHA2562D3C102A7095B03D7F64BAEECB8C2040946EF19574C1F394408D543315B17EC5 calls QueryFullProcessImageName before checking same-handle exit. Real owned cmd child exits while its process object is retained: OpenProcess succeeds but image query returns31. Both x86/x64 originalhelper reproducibly FAIL31. Formal installer-process-exit-race.ps1 fails this actual boundary before fix. No installed process or archive file was touched.

Original Inno PrepareToInstall + stale owned discovery + original helper reproduces helper1/setup7; fixed helper with identical real exited-child fixture completes isolated installation. Original code1 trigger is thus red-capable at actual native AND installercallback boundaries, not invented synthetic success. This is a proven matching cause, not proof every older code1 was identical.

## Narrow repair / retained safety

Same acquired native handle first checks WaitForSingleObject0. Only signalled handle means already exited/no termination. Query/terminate failure saves original native code then rechecks same handle; only newly observed exit is harmless. Live image query/permission/termination31or5 and waitfailure6 remain errors; image-path comparison before termination retained, PIDreuse cannot redirect. Acquired handles close once. No blanket31ignore/extraelevation/namewidekill/unknownidentitybypass/recoverydeletion/servicechange. Safe fixed native_api open/query_image/terminate/wait is added to failure diagnostics, no path/secret/rawmessage logging.

Tests add real exited/live children and controlled production-native-body race answers, including stale discovery→original Stop-UpgradeTargets;25checks each x86/x64. First managed error-injection fixture failed because SetLastError did not update CLRcache; corrected ONLY fixture error observation, production remains real Marshal.GetLastWin32Error; failures retained. scripts/package.ps1 now mandates both architectures and exited-process Inno scenario in future packaging (even SkipVerification), preventing this previously missing gate.

## Executed verification

27commandsPASS: x86/x64race25each, processidentity22, originalupgrade18, preparationdiagnostic7, recovery100, ownedset632, exactfile2, Inno13, BrowserHostreplacement5, lifecyclesource. Parser4paths and scopeddiffPASS. Saved compiler-input helper rerun x86/x64PASS. Tests operate owned children/tempfiles/fake discovery/nativeanswers only; no actual installed preparation replay. No temporaryDEBUGproductionprobes.

## Corrected delivery

Normal `artifacts/installer/NxTools.Next-setup-0.7.0.exe`26,213,543bytes, SHA256 `85B850532C013FD2CEE4945875CB5DF4BE16599C8286385980876DD1FA5281D7`.
Replaces failed32 SHA256F8E3FEAE7D12FCEA1FD62C3E2F658A1138B6849CCBA5039845C20C5AA4AAB179, preserved in previous-installer.
Same signed32/releaseID73b6a27d-d4ad-4c8a-95da-b4eea608183e/77members+2sidecars unchanged, rawRSA3072-PSS/publicanchor/5approvals/helperbindingPASS before/after. NoCargo/privatekey/newsequence or productbinarychange. AuthenticodeNotSigned. Original Setup script/layout unchanged; only embedded helper contents differ. Original source wrapper gains regression gates only.

Evidence `.diagnostics/installer/code31-20261028-085957/` preserves userlogs/reproRED/InnoRED/GREEN/allchecks. `.diagnostics/packaging/installer32-exit-race-20261028-091814/` preserves exactcompilerinputs/compilelogs/verification/failedsetupbackup. Finaldeliverymetadata INSTALL/evidence/checksum updated.

No real installation/productlaunch/service/Registrymutation/recoverydeletion executed. User failed before filepreparation according to actualdiagnostics; no recoveryworkaround applied. Live fullinstallation remains NOTRUN. Earlier installationerror1history is not overwritten/markedallresolved. This revision fixes the demonstrated exited-object race and adds preciseAPI diagnostics for any different native failure.
