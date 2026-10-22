# W6–W9 source implementation outcomes

## Outcome and scope
The user-approved cross-feature extension source is implemented: W6 declarative packages and joint W7 one-shot execution / W8 stable results and feature integration / W9 settings and whole-user trust. Downloads, Quick Launch, Clipboard and Tools share one host-neutral runtime and management registry; this is not launcher-only. W10 resident plugins is deferred.

Joint source gates passed before removing the development-only RuntimeUnavailable rejection. New imports/replacements remain disabled and untrusted. Approval does not enable; explicit enabling verifies actual staged declaration/code and selected interpreter identity. Runtime rechecks identities/settings before effects/results. Revocation disables/cancels and invalidates pre-acceptance receipts/results. No real installed package or user configuration was enabled by the agent.

## Delivered boundaries
- Strict bounded local manifest/listed-code import, staged content identity, aliases and two-way manual keyword conflicts, original Core atomic section save.
- Python, Windows PowerShell, pwsh, Node and package-local EXE; actual installed execution fixtures and checked-in examples. Demand C# private stdio broker verifies actual parent/current SID/session/ordinary token, outer/inner Jobs, suspended admission, inherited handle allowlist and bounded cleanup. Rust owns state; C# owns native process/credentials; GPUI renders.
- Download common Probe/direct/browser/IPC/batch acceptance, host-issued receipt reuse, durable original/effective provenance and idempotency-before-parser. Single selection or explicit ordered chain <=8, conflict/cycle/failure stop, conservative cross-origin credential drop, explicit retry/bypass; Blob/media/schedule/resume owners preserved.
- Feature host calls route to original DownloadEngine / Clipboard worker / original typed native command dispatcher. GUI-only operations fail explicitly in headless mode. Image reading is not advertised; no raw DB/HWND/GPUI API exposed.
- Actual committed Download/Clipboard subscription, host-managed causal roots/depth/dedup/rate/circuit. Automatic handlers and ordinary providers individually default off. Three failures within a minute pause the automatic handler; manager exposes reason and explicit retry, no historical replay.
- Stable namespaced Quick Launch results, same-ID update and selected identity preservation, current generation/content/action checks and no plugin elevation. Downloads/Clipboard selected-item manual entries and named Tools actions use shared invocation.
- Official GPUI Kit0.7.1 Settings/Form/Alert/DescriptionList/Select/Switch/public slots; original geometry, entities and deferred cross-owner effects. Save/Cancel typed settings, masked password drafts and attempted-write credential compensation. Native generic credential limit2560bytes; fake stores only in tests.

## Bounded final review and repairs
One read-only lifecycle review produced five concrete defects, all repaired with regressions:
1. Dropped invocation consumer no longer stops unrelated broker runs. Pending native identity and execution permits survive until cleanup or confirmed exact broker reap; restart only after reap, never after runtime shutdown.
2. HostContext cancellation/lease closure and dropped host-call future cancel the actual DownloadResolutionControl; blocking permit remains until actual finish. Paused resolver cancellation proves zero probe/task effects.
3. Registry publication serializes and installs a fail-closed transitional resolver, not a no-op. Every prepared request, including no-op, binds engine generation through final acceptance. Barrier-controlled concurrent submit creates no task during publication.
4. downloads.probe receipt can be consumed by submit/enqueue using its original operation ID; actual localhost counted adapter resolves once. Invalid/expired/revoked receipts return resolution_required without rerun.
5. Separate queue32/global4/per-package1 admits one blocked package plus32pending and three unrelated active packages, not35pending behind one package. Queue wait counts toward deadline; native cleanup owns running permits.

Additional verified integration corrections: per-entry omitted deadlines search3s/resolver10s/manual-event30s with10s/30s/120s caps; GUI confirmation rejects invalid receipt without rerun/task and retains draft; shared event circuit has reachable retry; protocol error completion is not a manual/event success.

## Verification evidence
Session scratch contains complete logs. Counts below are distinct suite totals; child-process assertion output is not counted twice.
- Desktop full718/718 after actual script enablement path: `w6w9-enabled-desktop.log`; prior joint gate718/718: `w6w9-desktop-joint-gate.log`.
- Extensions32/32 (lib6 + events2 + invocation3 + package8 + routing1 + merge2 + scripts7 + settings3), Core47/47, Finder85/85: `w6w9-enabled-domains.log`.
- Download selected safe suites297/297: lib117, domain12, engine103, HTTP client22, HTTP fixture7, message15, resolution14, storage7: `w6w9-download-reviewed-full.log`.
- Download IPC6/6 and Protocol45/45: `w6w9-owners-reviewed-full.log`.
- C# focused extension process/credential/picker21/21, no skip: `w6w9-native-fixtures-corrected.log`; project build0warnings/errors: `w6w9-native-build.log`.
- Strict six-owner Clippy, workspace--tests compile, fmt, five QuickLaunch/DownloadUI/ClipboardUI/DesktopShell/UIConsistency source gates, spec93documents/13owners passed before enabling: `w6w9-*-joint-gate.*` and final source gate logs. Final post-enablement rerun is recorded in the closeout update below.
- Real staged Python -> shared executor -> original engine; publication barrier/revoke-retained task; committed created event -> real script -> host enqueue -> own-event no-loop; GPUI provider and manager pointer/keyboard/minimum viewport; Clipboard worker commit and derived/native completion; secret compensation all executed in full Desktop/domain suites.

Compile mistakes, formatting failures and the invalid C# --filter selection were authoring/command errors, not behavioral reds. The earlier full Desktop708/1 failure was causally repaired and integrated full runs subsequently passed. No temporary DEBUG probes remain. Earlier installer/UI/indexer/update/W1–W5 work was preserved, not recertified as this release.

## Limits / no implied acceptance
No Release packaging, installation, service/registry/ACL/tray mutation, real Credential Manager or user Clipboard/history/config/download operations, external URL/program action, commit or push. Existing sequence2 installer predates these source changes. Installed physical input, mixed/high DPI, UIA/reader, real plugin/network/runtime policy and protected release tuple remain separately authorized acceptance. Trusted scripts have full current-user OS authority; no malicious containment, external dependency guarantee or rollback of arbitrary external effects. Debug hashing measurements are not Release/end-to-end performance claims.

Protocol and actual author examples: [PROTOCOL.md](../../../docs/extensions/PROTOCOL.md); user workflow: [usage.md](usage.md); executable contracts: [code-spec.md](code-spec.md) and Extensions quality spec. Source task completion is distinct from unperformed installed/release checks.

## Final closeout verification
Latest post-enablement full Desktop718/718 (`w6w9-desktop-closeout.log`), strict six-owner Clippy (`w6w9-strict-closeout.log`), workspace--tests (`w6w9-workspace-closeout.log`) passed. C# entire audited safe fixture selection244/244,0skip/0fail (`w6w9-csharp-safe-full.log`); native/restricted acceptance was excluded by the test-effect selector. One remaining fmt-only manual_actions change was formatted before the final fmt/source/spec checks. No new behavioral failure is hidden by that formatting correction.

Final fmt, normal/build production dependency graph test-support absence, five architecture gates, spec93documents13owners and scoped diff checks all passed. No surviving Cargo/rustc was found. Parent and both children marked source-completed2026-10-15 without archive/commit/package. W10 remains deferred.

Diagnostic closeout revalidation: bounded metadata-only host-call accepted/terminal and truthful run error status added; Extensions32 and Desktop718 again passed (`w6w9-extension-diagnostic-final.log`, `w6w9-desktop-diagnostic-final.log`), strict/workspace/fmt passed (`w6w9-*-diagnostic-final.log`). No plugin-controlled call ID/method/params/output is logged. No additional review or new feature scope.

## Subsequent authorized formal package
User then explicitly requested formal Release packaging. Signed sequence4 package and installer completed2026-10-15, existing publisher identity preserved; sequence3 interrupted before Cargo and permanently reserved. Current installer18,062,425bytes/SHA256624B0877FD5542C584257A764F2DC0557404E37F159374A6B673331DE9124B0C includes W6–W9. Independent38file signature/hash and3embeddedapproval checks,10installerfixtures,productiontest-supportabsence and finalspec/diff checks passed. RustReleaseoneunusedmethodwarning recorded. No installed/service/user-data acceptance performed. Exact current evidence/exclusions: release-package.md and artifacts/installer/evidence.json; earlier no-package paragraphs describe source closeout before this later authorization.

## Installed UI rejection correction
User2026-10-15 rejected Tools placement and managerUI. Original Tools-entry implementation/tests certified wrong location, not user acceptance. Corrected directExtensionsworkspace belowClipboard; listfirstoverview/intentionalimport/structuredpackagepages/pinnedSaveCancel/draftretention, allfunctionsruntimeowners retained. FinalDesktop721/strict/workspace/fmt/5sourcegates/specPASS; fiveactualGPUIcapturesinspected, pendinguserapproval. Currentsequence5installer includes updaterpeerrepair but predatesUIcorrection; no automaticrepackage. Exactscope/evidence `.trellis/tasks/10-15-extension-workspace-ui/outcomes.md`.
