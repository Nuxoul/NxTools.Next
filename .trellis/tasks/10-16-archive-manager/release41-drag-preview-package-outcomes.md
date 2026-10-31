# Current installer revision — running-product upgrade completed

The Release41 signed program payload is unchanged. Current normal installer26,727,788bytes,
SHA256`0605FF2CD0B90E4B134D5EC9273C0051F728C716B5A8E53D0321F1387B5F2137`.
Explicitly authorized actual upgrade from a running protected Desktop completed with installer
exit0 and installed/protected file readback. See [running upgrade repair](installer41-running-upgrade-outcomes.md).
The original build-only package record below is retained unchanged as historical evidence.

---

# Release41 — accepted drag/preview changes and formal installer

2026-10-30. [Scoped acceptance](release38-drag-preview-repair-acceptance.md) covers the delivered drag,80B preview,automatic preview and obsolete-test cleanup. The user's explicit menu requirement is **View Details last**. No archive/download product correction was required by acceptance; the normal formal installer contains these changes.

## Artifact

- `artifacts/installer/NxTools.Next-setup-0.7.0.exe`, version0.7.0 / release_sequence41.
- Release ID:`282031ce-891b-4747-8f5b-faf29740983a`.
-26,721,669bytes;SHA256`B45355EED2612341959053AE152CEDDABB553772EE20E96F5F60DCA18716E9FF`.
- Original public release identity and internal RSA3072-PSS/SHA256 independently verified. Manifest SHA256:`ea848040cbf98f154fbcb2f9dbe3f3afd54350a5be829fe32d8a29eced9ec902`.
-77members/complete79-file set verified,10actual build outputs match,5archive/helper approvals and actual helper-Desktop binding verified. Installer checksum matches the adjacent sidecar. Authenticode remainsNotSigned.

## Verification reuse and focused behavior

The successful final invocation uses original `package-local.ps1 -SkipVerification` to reuse completed verification rather than repeat full suites. Product source was compared byte-for-byte with the earlier verification baseline; only two test-only renderer files changed. The final successful invocation contains zero Rust test summaries and zero C# full-suite summaries. Release binaries/host/helper, original signing and normal Inno installer still built; installer-only11scenarios and BrowserHost replacement5scenarios passed.902source inputs were unchanged during packaging.

Relevant acceptance actually executed:347related Desktop tests,20engine/extraction-plan contracts and exact12original native fixture LIST/RUN/PASS with zero skips. These prove the represented original row/menu/preview/controller/native-stream boundaries, not physical Explorer behavior.80BZIP/TAR output, real over-limit refusal, same-name ZIP/two-block7z original bytes, unknown size, budget separation and cancellation/cleanup are covered. Ordinary unsafe/colliding extraction still refuses a writable plan. One bounded read-only review found no concrete defect; no repeat review loop.

The default full-build invocation's unrelated shared-screen fixture exposed a duplicate WinForms renderer. Input and foreign-process tests now reuse the existing passive NativeWindow renderer in FloatingAssistantRendererFixture. The duplicate Form is removed; every hit/pixel/pass-through/noactivate/gesture assertion and test selection remains. The complete C# catalog then executed in original order to terminal500PASS/100typedSKIP/0FAIL/0not_run. That result is reused, not claimed as a new run in the final package invocation. The100optional-environment skips are not credited as passes.

## Evidence and preserved boundaries

- Final package: `.diagnostics/packaging/20261030-155219-5872049a/`, stateCompleted/exit0.
- Independent signature/build/source proof: `C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\formal-package-reuse-20261030-155217/package-verification.json`.
- Scoped acceptance: parent `accept-drag-preview-20261030-142716` logs and source identities.
- Reused verification and full-C# renderer follow-up: parent `formal-package-20261030-143810` logs. Original sequence39 failed shared-screen result and user-cancelled40 remain consumed/retained, not overwritten as successful releases.
- The exact installer helper hash remains`16062B20D23495D67F4BD1905E3E3DE7CF10251333DFF9ACB451BF6BC0C189BD`, preserving the process identity correction with prior realRelease38 installation evidence. This does not establish real installation of41.

## Limitations

No new package installation, installed-product launch, physical foreign-target drag, mixed-DPI/focus or user visual acceptance. Original archive unavailable; fixtures establish supported reachable behavior, not the unknown original format. No HZ5, real-user source reclamation, key replacement, ledger reset, process-name termination, dependency upgrade or commit/push. Existing dirty changes remain preserved.
