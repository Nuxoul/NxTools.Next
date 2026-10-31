# Installer31 process-close code5 — repaired normal installer delivered

2026-10-27. The user reported actual Preparing to Install error5 from the normal sequence31 installer. The prior delivered package failed; its build success did not establish installability. This repair does not reclassify the historical unresolved error1.

## Actual failure and causal differential

Persistent logs: `.diagnostics/installer/preparation31-code5-20261027/`. The supplied truncated directory resolves to correlation `20261027132404-is-IU4AGOWKJ6-tmp`. Launch started1/exit5; preparation JSONL reports 32-bit `process_close`, Win32Exception/native5/scriptline121, before file preparation.

The inaccessible same-name Host is the independent LocalSystem updater, SCM service `NxToolsUpdateV1`, PID5332, fixed `Updater/Bootstrap/NxTools.PlatformHost.exe --trusted-update-service`. Read-only registration/path/account/hash inspection identifies it; opening its process for a query also returns5. Ordinary upgrade preparation must not terminate this service. Old Stop-UpgradeTargets throws before distinguishing that role.

A read-only actual differential uses the real Stop-UpgradeTargets with an EMPTY target set: no termination authority and no file preparation. Both x86 and x64 delivered helpers reproduce native5. Both architectures with the fixed helper succeed/count0. The same differential was repeated against the exact compiler-input copy after compilation; old fails5/new succeeds, updater remains Running/PID5332. This proves the observed classification trigger, not complete live installation.

## Narrow repair and invariants

`installer/close-owned-processes.ps1` reuses read-only SCM_CONNECT1/SERVICE_QUERY_STATUS4/QueryServiceStatusEx authority from the original TrustedUpdatePeer boundary. Validate original fixed service path/arguments, LocalSystem, plain path, Bootstrap actual-vs-registered SHA256, key-hash format, Running4/ownprocess0x10/live PID. Only an unreadable Host whose PID matches that verified service is excluded.

Unknown unreadable processes still fail closed; exact owned Host stopping and same-handle image/termination verification remain unchanged. No SeDebugPrivilege, name-wide taskkill, blanket error5 suppression, recovery deletion, trust replacement, or new service mutation. Original enrollment/migration remains publisher-key authority; this exclusion is not a new trust system. Reference evidence: `docs/reference-code/archive-shell.md`.

## Regression and checks

- Original helper red-capable regression captured before repair. Process identity22 checks, original upgrade processes18, actual preparation entry7 scenarios PASS, including verified updater success and unknown Host native5.
- Recovery100 assertions, owned-set rollback632, exact-file rollback2 PASS.
- All12 Inno runtime and all5 BrowserHost replacement scenarios PASS, including expected nonzero failure/cancellation cases.
- Source lifecycle gate, spec-contracts, PowerShell parser and git diff --check PASS. Existing CRLF warning output caused two PowerShell command interruptions; captured separately with cmd and verified exit0 without altering unrelated files.
- No temporary DEBUG probes added to production. Query-only probes and reproducible reseal verifier retained in evidence. Tests use isolated/fake mutation boundaries; no actual installed mutation.

## Corrected delivery

- Normal `artifacts/installer/NxTools.Next-setup-0.7.0.exe`, compiled13:52+08:00, 25,987,628 bytes.
- SHA256 `4AEF44B2C4F71EC832C509CDFDCA8D8036FD1CB61B59B19FA1AE6AA4AC77C9A4`.
- Supersedes failed31 hash `9348882377E680A08BFBA53641DF3C042CD7EBACF83F5000708A4D1F299CBA1C`; exact old installer/checksum/guide/evidence backed up under `.diagnostics/packaging/installer31-code5-20261027/previous-installer/`.
- Original Inno entry compiled against unchanged signed sequence31/releaseID `2bb892a8-51d0-4316-a3e7-33d9fe755621`. No Rust rebuild/private-key use/new sequence. Compiler input helper SHA256 `2D3C102A7095B03D7F64BAEECB8C2040946EF19574C1F394408D543315B17EC5` captured and unchanged throughout compile; compile log names this source.
- Before/after raw-manifest RSA3072-PSS/SHA256 verification PASS; all77 exact members plus2 sidecars unchanged, archive approvals5/helper Desktop binding PASS. Manifest SHA256 `7a02520a3431b8b3798fcaca1d884240962e0297af8990fed22be6488ba4210a`. Public key hash unchanged. Historical diagnostic/installfix artifacts unchanged.
- Compiler successful41.360s; existing FileCopy rename hint retained. Default delivery copied only after offline checks, matching compiled output and SHA256 sidecar. Windows Authenticode NotSigned is distinct from internal release signature.
- Persistent reseal evidence: `.diagnostics/packaging/installer31-code5-20261027/` includes compile inputs/log, verifier, before/after verification, delivery identity and actual readonly x86/x64 results. INSTALL.md/evidence.json updated.

Actual full installation and GUI/native/user acceptance remain NOT RUN. No assistant installation, product launch, real service/process stop, Registry/COM change, recovery deletion, key/ledger change, commit or push. User should close the old failed wizard and use the corrected normal file, preserving diagnostic/recovery files. Do not infer that every historical installation failure is repaired.
