# A — Native OneCommander UIA acceptance

## Verdict

**Compiled/source-delivery and existing safe-retention checks pass. Native lifecycle and memory-root-repair acceptance remain unverified; overall status is not_accepted.**

This verdict does not identify a dynamically reproduced remaining leak or certify that one is fixed. The implementation deliberately supplied no build/native/memory evidence; this independent round adds the limited execution below. No new product-source changes, product/helper launch, global focus subscription or sustained memory experiment were made.

The source change removes ordinary/helper unconditional WarmUp, migrates affected OneCommander access away from System.Windows.Automation and introduces native client/handler/pane ownership under the original tracker. Scope is the [ten-path implementation manifest](implementation-changed-paths.txt), not the entire dirty HEAD diff. The implementation's [original not_run table](implementation-outcomes.md) remains its historical handoff.

## Independently executed checks

| Check | Actual result | Evidence |
| --- | --- | --- |
| Matching Debug PlatformHost.Tests build | Exit0,0warnings,0errors | [Build](evidence/uia-coordinator-build.log) |
| Existing safe OneCommander selection | LIST9/RUN9/PASS9/SKIP0/FAIL0/not_run0; Fixture only | [LIST/effects](evidence/uia-coordinator-safe-list.log), [RUN](evidence/uia-coordinator-safe.log) |
| Spec/document navigation gate | PASS129documents/14owners | [Spec log](evidence/uia-coordinator-spec-contracts.log) |
| Scoped changed-product/reference/spec diff whitespace | Exit0 | Command recorded below |

Build command: `dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj -c Debug --no-restore -m:1`. TEMP/TMP used coordinator scratch. No Cargo verification or release/package build ran.

The original Debug apphost first LISTed and then RUN with identical filters: `OneCommander focus tracker fixture`, `OneCommander focus filter fixture`, and `OneCommander adapter freezes its UI Automation pane`. All9observed effects were Fixture. Bodies were read: injected synthetic elements/native identities, cache/thread/barrier and fake adapter only; no real UIA subscription or user windows.

Scoped diff command: `git diff --check -- <implementation-changed-paths.txt entries> docs/reference-code/platformhost-uia-memory-repair.md docs/reference-code/system-shell.md .trellis/spec/platformhost/native-contracts.md`. It returned0. The spec gate checks links/structure, not COM method meaning or runtime ownership.

## What these results establish

The current C# source and existing callers/tests compile together. Existing sample/remember/replace, bounded64-window cache, unavailable-window/element retirement, stale validation, window reuse/stop, in-flight synthetic stop, process-before-tree filtering and frozen-pane adapter semantics remain green at their old seams.

By source inspection, ordinary constructor and elevated helper now begin dormant directory lifetimes rather than startup WarmUp. The original pipe disconnect, helper using/finally, context consume/expiry/capacity and native session stop paths have cleanup wiring. The native client has explicit HRESULT, BSTR/VARIANT/interface/handler ownership code. These are source observations, not observations of actual UIA cleanup or memory behavior.

## Remaining acceptance evidence

### A-E1 — Native ABI and actual COM lifetime

The safe tracker fixtures instantiate synthetic operations without the new native StartOwner path. They do not invoke NativeUiAutomation/OneCommanderUiaSession through a real native client, prove every callable ABI slot/argument/result, or observe native handler reference counts and final release. Successful compilation cannot establish those facts.

A controlled witness must cover actual native creation/query/pattern/comparison/cache/event registration and exact unregistration, entered-callback drain, pane/context release, COM teardown and actual thread exit. Cover registration/removal/query failure, HRESULT preservation, demand loss/reacquisition, timeout with physical operation still held, abandoned capture and host/helper scope retirement. In particular, verify failed removal plus subsequent demand against the declared no-reregister-over-unknown-cleanup contract rather than treating logged failure as proof of retirement.

### A-E2 — Ordinary/helper no-target and target behavior

No controlled role startup/module-load witness ran. Confirm that neither no-target role activates this managed global focus/proxy path or loads UIAutomationClientSideProviders through it. Target start/stop, window destruction/reuse, feature disable, pipe reconnect/helper EOF and disposal need actual ownership assertions.

Preserve active/split/WPF HWND-less pane capture before overlay, nearest getter/setter, ordinary/elevated read/navigation/inline-search, failure and focus restoration. Existing fake adapter tests do not prove physical OneCommander/provider or helper behavior. Real product/reference GUI/UAC/elevation remains separately controlled.

### A-E3 — Reported native-memory growth

No before/after native-memory workload, module observation, allocation trace or sustained activity measurement ran in this acceptance. There is no measured private-bytes/native-heap improvement to report. The original incident's CCW allocation share remains inference; it was not newly established here.

Use an owned equivalent activity workload and record private bytes, GC heap/committed, system commit and relevant native/handle/thread/subscription evidence, with warmup and post-teardown observations. Distinguish no-target from active-target conditions and implementation/client from physical/elevated product results. Define useful workload/time/thresholds before execution; an idle sample, forced GC or migration alone cannot answer whether the original activity-dependent growth is gone. This verdict does not impose a new fixed long-test schedule; it states which evidence is absent.

## Release and coordination

Do not label a formal package as memory-fixed from these results. Source delivery is compiled and its old safe seams pass, but native/memory acceptance remains open. B6 experiments and urgent formal Release packaging were not performed by the coordinator.

Final fixture-process scan found0matching test apphosts. No global UIA handler, product/reference window, actual elevated helper, power action, service/registry mutation, install, commit or push was initiated. No product defect is invented from a missing witness; any real remaining native error or growth requires its own concrete boundary result.
