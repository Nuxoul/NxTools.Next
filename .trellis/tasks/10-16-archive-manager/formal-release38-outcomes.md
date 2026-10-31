# Current installer revision — real upgrade completed

The Release38 signed payload remains unchanged. The current normal installer is26,724,105bytes,
SHA256`E3ACD0E9CD773A6BC3E1CA93E14EF26F44BCFA140ECF2093132E9C78416A51CE`;
explicitly authorized actual installation/trusted update/finalization completed with exit0.
See [installer repair and installed readback](installer38-code5-repair-outcomes.md).
The initial full-build record below remains the evidence for that original signing invocation.

---

# Formal Release38 — completed

2026-10-30. Normal `scripts/package-local.ps1`, full verification enabled, actual Release Rust/PlatformHost/helper builds, original release key and monotonic ledger, internal signing and Inno installer completed. No automatic installation or product launch. Archive implementation remains **Phase E — Format expansion and ZIP editing**; original steps1–6 scoped acceptances remain, HZ5 is not started.

## Delivered artifact

- Installer: `artifacts/installer/NxTools.Next-setup-0.7.0.exe`, version0.7.0, sequence38.
- Release ID: `53692ed3-7f88-4678-88d8-3275441d4a71`.
- Size:26,720,959bytes /25.48MiB.
- SHA256:`7733A5504A37E10AC05CD265190D194BA463E6848883EF48837242203B4BC5A7`; adjacent checksum independently matched.
- Internal signature:RSA3072-PSS/SHA256, independently verified with checked-in public SPKI only. Anchor SHA256:`03e979d5c6babda29aa217b7ce72268aab12870bda46a9072397fe52ba083b69`.
- Manifest SHA256:`16677650809a73da2b7bab9acf646e7c37d94e8e38282cf884d1e4abee410282`.
- Windows Authenticode:`NotSigned`; distinct from the verified internal release signature.

## Verification

| Boundary | Actual result |
| --- | --- |
| Original formal entry | Completed/exit0; sequence38; SkipVerification=false;1972.15seconds |
| Architecture, fmt, workspace strict Clippy | Passed in formal invocation |
| Full Rust workspace |2994PASS/0FAIL/1ignored/450filtered; ignored/filtered not credited as execution |
| Actual Release capability/terminal gates |4RUN/4PASS/0SKIP/0FAIL |
| Full matching Debug C# fixture collection |600selected/600RUN/500PASS/100SKIP/0FAIL/0not_run, original catalog order |
| Additional actual Release native regression |4RUN/4PASS/0SKIP/0FAIL: CLI cleanup ownership, original five-format readmission chain, source-family protection and ZIP finite-source admission |
| Inno owned runtime cases |10PASS; expected failure/cancel exits retained as passing negative scenarios |
| BrowserHost replacement/rollback |5PASS |
| Signature and package |77 signed members verified, exact79-file set including two sidecars;10 actual build outputs match |
| Embedded approvals |5 actual packaged archive/helper hashes and original public anchor verified in Host; helper binds actual Desktop |
| Product icons |18 frames from actual packaged Desktop PE match approved assets |
| Source preservation during formal run |903 tracked-by-controller source inputs unchanged; no inferred ownership from full dirty Git state |

Persistent formal evidence: `.diagnostics/packaging/20261030-100128-5d8c4366/`.
Independent evidence: `C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\formal-package-20261030-100127` (`package-verification.json`, exact counts, Release regression log, source baseline/drift and original invocation).

## Final contracts

- `build.ps1` retains the original single top-level production publisher call and binds tests to that build's generated manifest and Desktop identity. `test-platformhost-fixtures.ps1` owns matching Debug behavioral verification, bounded low-memory Cargo preparation and restoration of caller fixture variables. Release capability checks remain active; low Risk thresholds never become a Release feature. Build-argument regression covers Debug/Release, paths with spaces and unfiltered full fixture selection.
- ZIP extraction readmission has a private native source-admission flag. After acquiring original source/parent leases, a ZIP signature may use max(current output,current metadata) for encoded input. Ordinary read/TEST/non-opt-in calls, target output/decoder/Job budgets, source/family identity and publication/cleanup authority remain unchanged. Boundary controls reject ordinary low-cap reads, metadata overflow and signature impostors; the original five-format Job chain reaches shortfall with no target effects and completes only a fresh source-bound finite admission. See [verified source chain](research/zip-output-readmission-sources.md).
- Directory-entry synthetic tests call existing `EnterAndActivate`, not directory Reveal; existing empty/wrong identity controls remain, and the helper/Host assertion compares their actual common Desktop identity. No production Explorer behavior or real user window was modified.
- Failed reservations33–37 remain consumed and all prior negative logs/checkpoint history remain intact. Sequence37's original release-chain rejection is retained in `.diagnostics/packaging/20261030-093559-e5a236d4/`; its gate was not weakened. No key replacement, ledger rollback, unsigned fallback, assertion/budget weakening, process-name termination, staging-key package or HZ5 reclaim occurred.

## Limitations

100 C# optional-environment/fixture cases report typed SKIP, not PASS; exact names/reasons are retained in the original formal log. This package completion does not replace historical per-slice acceptance, physical Explorer/focus/DPI/IME/UIA/user acceptance, installed service validation or true power-loss/durable job recovery. No actual product installation, launch, Registry/UserChoice/ACL/service migration or real-user source reclamation was performed. Existing HZ4 process-interruption/fresh-safe-refusal and bounded secondary-reader unsupported distinctions remain unchanged.
