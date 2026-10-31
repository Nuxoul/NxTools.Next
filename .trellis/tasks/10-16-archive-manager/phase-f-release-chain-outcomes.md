# Phase F — release-chain source integration and isolated staging outcomes

Date: 2026-10-23. **Release-chain integration and staging verification pending independent acceptance.**
This is the SAME **Phase F — Explorer integration and genuine native drag**. Authority was recorded in
[existing plan](implement.md#approved-phase-f-release-chain-source-and-isolated-staging-amendment--2026-10-23)
and [sole checkpoint](phase-a-checkpoint.md) before implementation. No new phase or parallel plan.
F-AC01/02/03, Phase E and earlier independent integration records remain closed/historical.

## Delivered boundary and identity graph

```text
Rust Desktop / Indexer / BrowserHost / CLI outputs
+ nxtools-archive-worker (--features isolated-codecs)
    -> actual Desktop SHA256 -> generated ShellSelectionHelper Desktop identity
    -> framework-dependent helper apphost + DLL + deps.json + runtimeconfig.json
    -> existing binary-manifest generator: core hashes + five archive-member hashes
    -> PlatformHost publish with explicit generated-source inputs
    -> original offline RSA-PSS signer: complete final directory
```

Desktop does NOT embed helper/PlatformHost output hashes. Its original protected-redirect function
compares sibling files; there is no reverse build dependency or cyclic embedded-hash graph.
No codec/backend/dependency/version/vendor change. Rust state/layout, C# Windows boundary and GPUI
rendering ownership remain unchanged. No UI redesign or production archive-row drag connection.

| Original owner | Current change / invariant |
| --- | --- |
| `scripts/build.ps1`, `package.ps1` | Build original Rust products plus isolated-codecs worker, then original helper/host publishing chain. Shared low-memory initialization and per-command Cargo-lane checks; Release commit headroom before Cargo. |
| `scripts/publish-release-hosts.ps1` | Shared build orchestration, not a second signer/deployer. Actual Desktop -> helper -> generated approval -> PlatformHost. Explicit scratch generated and .NET build outputs. |
| `write-shell-selection-identity.ps1`, helper project/Program | Generate this Desktop's actual SHA; explicit MSBuild replacement of empty source default. Empty/stale/wrong/missing Desktop still refuses through original `DesktopIdentity.Verify`; same-directory check unchanged. COM identity/refcount/pump/once-only ordered selection unchanged. |
| `write-release-binary-manifest.ps1`, PlatformHost project | Actual worker and helper four runtime files enter generated `ArchiveReleaseManifest`; explicit external generated core/archive/trust compile inputs. Existing production Generated trust/core-hash files are NOT overwritten by staging. |
| `ArchiveReadHelper` / original runner | Fixed sibling worker, generated approval under retained no-reparse/write-denying source lease before original suspended Job admission. Original package runner also retains the approved helper in a production host; historical exact test-apphost fixtures retain their fixed sibling-only branch. No production fixture environment/PATH lookup. Source binding, Job/resource limits/cleanup unchanged. |
| `TrustedReleaseManifest.RequiredPaths` | All five new members required; seven executable files plus helper DLL/deps/runtimeconfig, licenses and extension content authenticated by original signed full manifest. Missing/altered files reject. |
| `IndexerServiceInstaller` | Stable-handle archive-source approval before native service effects; original deployment/readback includes all five files. Registered archive hashes and BrowserHost approval are written with original service tuple. Protected health/current approval reject incomplete current sets; old approved host distinction does not authorize partial current release. Null/empty/malformed hash rejects before optional-hash open semantics. |
| `TrustedUpdateBroker` / `TrustedUpdateTransaction` | Existing all-signed-file post-deploy validation, complete pending/active/backup snapshots, durable journal and last-good recovery remain the owners. No competing update service/channel. Fake deployment tests now actually mutate files and throw at each new member. |
| Desktop `protected_package_matches_release` | Existing Desktop/Indexer/PlatformHost equality plus worker/helper five-member equality. Missing, older or mixed member rejects redirect. This is not a new trust anchor. |
| Inno `[Files]` / original suspend/finalize | Five exact new file entries, two exact executable process names, full old protected hash approval. Twelve owned root executable/runtime/sidecar files journaled before mutation; new hashes predeclared from extracted new package, no post-failure hash grants deletion authority. Original BrowserHost library API retained. |

Installer precommit failure compensates every attempted member in reverse, including mutation-then-throw;
foreign/wrong-hash files remain, incomplete compensation aggregates failure and retains journal.
Durable `committing` is the point of no return after verifying the ENTIRE new set and all old backups.
Thereafter cleanup errors retain the complete new set and journal, never Undo into a mixed set. Explicit
commit retry validates new set plus durable per-backup cleanup evidence and resumes exact-hash cleanup;
non-commit rollback at/after that point refuses. Licenses/extension assets retain original Inno ownership.

## Explicit isolated-staging wrapper

`package.ps1 -Configuration Release -OutputPath <fresh scratch path> -SkipVerification -SkipInstaller -IsolatedStaging`
requires scratch-only fresh output, no supplied signing key/sequence/LegacyUnsigned. It creates a temporary
TEST key through the original key tool, generates TEST public trust externally, uses original publishing/
signing/verifying owners, never calls production sequence reservation, never stops processes/services,
and never creates an installer. Outside-package labels say NOT installable / NOT production-trusted.
Temporary private keys were deleted (success and failed-refresh finally verified); test public trust remains
only as evidence. Test sequence1 belongs to distinct ephemeral TEST keys, not a formal new release.

Default signing/sequence/installer behavior remains explicit; default packaging was NOT executed.
Staging is not offered as a user installer or as trusted by any existing enrolled production bootstrap.

## Actual final output

Evidence root:
`C:\Users\Administrator\.pi-desktop\scratch\7c2ebec3-f1a7-4623-9ed1-760f9adb94f5\phase-f-release-stage-20261023-090605`.

Final stage:
`stage-final-31820c5806f34fd58716ed86b1360352` below that evidence root.
Full actual path/size/SHA256 inventory: `final-staged-files.json` (79 files including signed sidecars).
Original signer authenticates **77 files / 112,860,404 bytes**; sidecars are outside their own file table.
Test release ID `31646015-6cb6-4132-ab07-f065a318570f`, test sequence1,
TEST public-key SHA256 `d44dad102642abc70ef2889f02275f6db101a75365bedb1ef7daa337d2776b38`.

| Actual output | Bytes | SHA256 |
| --- | ---: | --- |
| `NxTools.exe` | 85,298,688 | `628ACA287C4424F496E3CFF8BC6B530B8C3D5F564E0612B51C33167F256EFA77` |
| `NxTools.ArchiveWorker.exe` | 4,828,672 | `83BF56832A59FBE2A04F34E6EE9F70880F2609DB9751677B0A139A5E2D165DCD` |
| `NxTools.PlatformHost.exe` | 2,023,826 | `1B58563A62BCA17221F3D04BB25B66102488AC7324A438E4469062487F269C31` |
| `NxTools.ShellSelectionHelper.exe` | 152,064 | `C63648F4733C14BC0F4838F40189C5169374ED586774C9ACA3F75755A0540A56` |
| `NxTools.ShellSelectionHelper.dll` | 40,448 | `31D985C473D00F99C55C0F69CBA0394A3EED2853E4E861393289FC5E868B20B9` |
| `NxTools.ShellSelectionHelper.deps.json` | 529 | `070B1F144987FAD95AC9641BC5D6B88446A82AEFB42D699FBBBA3528B61009CF` |
| `NxTools.ShellSelectionHelper.runtimeconfig.json` | 417 | `4F8E15234471D8F0F8B3CC25EFBE7AD49A11387B72E393CACF5EC92359ABF22C` |

Actual helper deps lists only its DLL; runtimeconfig declares external `Microsoft.NETCore.App` .NET8.
Framework-dependent runtime is a prerequisite, not a secretly bundled runtime or dependency installation.
Actual compiled `ExpectedDesktopSha256` equals the Desktop hash above. Staged PlatformHost archive
approval equals actual worker/helper hashes and original signature verifier independently accepts it.
No helper Main, COM server, Desktop, Explorer or archive operation was launched to prove this.

Build evidence: first full isolated wrapper built real Release Rust outputs (3m41s products, 2m59s worker),
then helper/PlatformHost. Headroom preflight17.02GiB and wrapper16.82GiB, required6GiB. Serial Cargo
jobs1/incremental0/codegen-units256. Later C# source fixes refreshed only helper/PlatformHost/test trust
into fresh stages; all five Rust output hashes remained identical to this build. Earlier outputs retained.

## Regressions and checks actually run

| Boundary / command | Actual result |
| --- | --- |
| Original signer completeness | RED `03-red-run.log`: selected1/run1/pass0/fail1 (old signer accepted missing members). GREEN now requires every added member before signing. |
| Original Desktop redirect | RED `04-redirect-red.log`: exit101, 0PASS/1FAIL, missing archive set accepted. GREEN `14-redirect-green.log`: **1/1**, exit0, 1025filtered; original function, no product launch. |
| Original package runner approval | RED `27-worker-red-run.log`: exact1FAIL, no retained approved helper. GREEN `30-worker-green-run.log`: exact1PASS. Historical test-apphost path retained explicitly; production entry uses original helper. |
| Null archive approval | RED `44-null-hash-red-run.log`: exact1FAIL. GREEN `47-null-green-run.log`: exact1PASS; null/empty/malformed identity cannot mean unchecked open. |
| Final C# exact collection | `48-closure-list.log` -> same `49-closure-run.log`: **LIST30=RUN30=PASS30**, selected30/run30/pass30/skip0/fail0/not_run0, exact names/order equal; `final-csharp-results.json`. Includes original16 Shell fixtures, six signed-manifest cases, four original transaction cases, three new release fixtures and actual-stage identity. |
| Actual staged identity | Existing helper identity + staged managed constants/approval + original signature verifier: actual Desktop binding, worker approval, four runtime files and full signature PASS; empty/wrong hash rejects. Additional GUID file-only missing/old Desktop refuses. |
| Fake native deployment | Mutation-then-throw at each of five new members; original trusted transaction restores all signed old bytes, sequence unchanged, foreign file preserved; retry commits. Included above, no native deployment. |
| Installer set regression | Initial source RED exit1 (no complete-set mode); first GREEN414 assertions. One bounded review found post-commit cleanup defect; RED `installer-set-cleanup-red.log` proved first old backup gone and other11 members reverted. Final GREEN **632 assertions**, exit0; parent independently reran same script in `50-parent-cleanup-final.log`, also632. No installer/process/service/Registry/COM execution. |
| C# builds/publish | Fresh isolated Tests/host/helper builds and final publishes exit0, **0warnings/0errors**. |
| Rust strict/scoped format | `cargo clippy -p nxtools-desktop --bin nxtools-desktop --jobs 1 -- -D warnings` PASS (`24-strict-clippy.log`); scoped `rustfmt --edition 2024 --config skip_children=true --check .../main.rs` PASS. No full test/fmt matrix. |
| Source/doc gates | archive-release-chain (including unsafe staging reject with unchanged production inputs), shell/spike ownership, package-memory-policy, platformhost-test-selection, installer-update-lifecycle, installer-exact-file-rollback, browser-signing-isolation, GPUI release, Markdown provenance, spec-contracts and scoped diff PASS. Specs109documents/14owners. |
| Task context validation | PASS82implement/79check entries; preexisting over32KiB context warnings remain (explicit relevant ranges read), no unrelated doc restructuring. |

One bounded optional review only. It found cleanup-after-commit and missing BrowserHost durable approval;
both concretely fixed and retested. No new review cycle. No full Desktop suite, codec/split/ZIP-edit/100k/
performance rerun; historical independent1022/1022 is not a new result.

Honest execution notes: C# run08 failed after6PASS because the new test catcher omitted
`InvalidDataException`; corrected test and exact collection reran successfully. Several Cargo checks
refused to start while other owners occupied the lane; none were killed or duplicated. An attempted
background runner left no process/log/build and was replaced by the same foreground serial runner.
One C# refresh in a deeply nested scratch path failed MSB3030 apphost-copy; failed output/key cleanup
was checked, failure logs retained under `failed-long-path-*`, and a fresh short scratch build passed.
Earlier stage/list/run logs were retained under `before-null-fix-*` before final refresh. No failure is
relabelled PASS or silently erased.

## Preservation

- Opening baseline:2966 tracked/preexisting-untracked files; `final-preservation.json` separates exact
  owned paths/hashes from concurrent Clipboard/UI task changes. No baseline file deleted. Do not use
  cumulative HEAD diff to attribute this round. No unrelated formatting, resets, commit or push.
- **Sequence21 artifacts:78/78 unchanged, no extra files** under `artifacts/win-x64` and
  `artifacts/installer`. `protected-artifacts-before.json`, `protection-audit.json` and final audit carry
  actual before/after SHA256. Sequence remains21.
- Existing production `Generated/ReleaseBinaryManifest.g.cs` SHA
  `E66F89C4BFB8DF0FB8701F4CF7EE3122D592E256C7747B71AAFC637AAD202E4C` unchanged;
  `Generated/UpdateTrust.g.cs` SHA
  `A333089F742E553A8C1C60DD26772F8C036767C90A725DD5415E6AB1051ECE97` unchanged.
- Sequence21 setup SHA remains
  `8F371044ADA651DD01F86DBD5AA8A6D77BF15CDD49AAD3E5A4CCE1EF5C22A713`.
- **54 existing archive outcomes/acceptance reports unchanged**. New report does not rewrite prior
  failed runs, acceptance or closed-defect evidence.
- No production signing-key environment path was configured; known candidate roots contained no
  discoverable key/ledger. Accordingly no claim of hashing an undiscovered private ledger. No production
  sequence-reserve/key/trust-writer was invoked; only new scratch TEST keys, subsequently deleted.

## Remaining / stop

**Stop at release-chain integration and staging verification pending independent acceptance.**
No current ProgramData protected copy update, no real service/Registry/CLSID/DelegateExecute/
LocalServer32, COM server activation, Explorer/physical drag, formal product startup, production signing/
sequence issuance, installer compilation/delivery, commit or push. ProductApplyAuthorized=false;
P208production rows disconnected. No P206/P207live/P208/PhaseF/first-release PASS claim.

A previously enrolled stable bootstrap is deliberately not replaced by routine package updates. Its
older signed-path allowlist may reject added files; actual compatibility/explicit bootstrap maintenance
is a separate authorized deployment prerequisite, NOT solved or certified by this staging. No trust
rotation, UAC bypass or maintenance channel was invented to hide that boundary.

Real signed distribution/install/update/rollback/crash/SCM and native Shell acceptance remain separate.
Earlier A–E/JOB07/SAFE08/native/DPI/UIA/user obligations, P204/P205/P209 and source-recycle boundaries
are unchanged. This round neither reopens closed backend/container/cache/font decisions nor starts the
next live acceptance step.
