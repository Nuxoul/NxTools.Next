# PlatformHost safe verification / native acceptance selection (2026-10-13)

## Outcome and scope
Completed the requested separation in test-selection/accounting and wrapper arguments only. No production C# Windows implementation changed. Preserve broad dirty work and all original211test registrations/names/delegates/order. Six new selection/accounting regressions added, total217. No ignored/deleted tests, name-prefix safety heuristic or blanket implicit safe default.

Read whole-project review, PlatformHost native/diagnostic and shared verification/architecture/engineering contracts before edits. scripts/build.ps1 called no-argument harness; package.ps1 verification invokes that build path. Original no-argument runner selected every test and returned0 on zero matches; printed SKIP followed by normal return was unconditionally labeled PASS. The two red selection tests each failed exit1 before selection behavior changed, without executing live delegates: default included native; no-match was not an error. Evidence:session scratch native-selection-red.log.

## Responsibility split
- Program.Catalog owns unchanged original registration names/function delegates/order, with6new tests. It creates reviewed TestCase via TestEffects and performs no test execution during listing/classification.
- TestEffects explicitly identifies every exact function symbol from body/callee review; unknown/new symbols throw before execution. It distinguishes Fixture,NativeAcceptance,RestrictedFixture. No generic native operation abstraction added.
- TestRunOptions/TestSelection own switch parsing, scope/authorization/name matching. TestRunner owns existing fail-fast execution, ambient SynchronizationContext reset and truthful terminal counters.
- scripts/build.ps1 explicitly invokes C# --fixtures and runs platformhost-test-selection.ps1 source gate. package.ps1 remains unchanged and inherits that call; no native-authorization option forwarded from package/build.
- tests/native/gpui-kit-acceptance.ps1 requires -AuthorizeNative before checking/starting fixture, then passes --native --authorize-native and original exact filters. It remains a separately invoked acceptance wrapper, never ordinary build.

## Entries and compatibility
No args or --fixtures:189explicitly reviewed fixtures. Bare legacy case-insensitive substring filters keep union/catalog order/no duplicate execution for fixtures. A filter crossing into native/restricted rejects the whole selection, not silent filtering. --native --list lists28nonfixture entries without invoking delegates. Actual native execution needs --native --authorize-native AND nonempty explicit filters; no unrestricted native run-all. Unknown/empty/zero-match/unclassified/cross-effect/unauthorized selection prints SELECTION ERROR,exit2,RUN0. Invalid combinations reject.

Catalog review:original211=183Fixture+23NativeAcceptance+5RestrictedFixture; six pure selection tests produce current189/23/5. Self-owned synthetic HWNDs/STA and fake native actions are genuine fixtures despite native/real wording; global environment owners are not. BoundedSTA/COM/OLE/schema/permission regressions remain and are not removed.

NativeAcceptance23:
-6external GPUI acceptance methods:actual env-bound HWND/PID, move/resize/show/hide/noactivate/UIA; retained original ownership preconditions.
-4existing-window tests:2DOpus,Files,Explorer; capture/navigation/activation needs user approval.
-3known-folder tests:actual Pictures/Desktop/Users and localized Shell metadata.
-2taskbar tests:live Shell_TrayWnd/taskbar COM despite owned overlay Form.
-3desktop-shell tests:actual GUIDHKCU writes/deletes/tray, not actual Run key but still live registry.
-1completion adapter test:fake sound/shutdown does not isolate constructor's global clipboard/window/keyboard/hotkey hooks.
-3pure-looking third-party tests:production static OneCommander UIA focus tracker can initialize via Files identity, timed-out UIA or elevation fallback; not safely isolated.
-1Shell context-menu enumeration:actual installed Shell providers plus production icon staging/trim.

RestrictedFixture5 (retained outside default, clear reason in LIST, not executed):CRX package test reaches production LocalAppData signing PEM; action bitmap test reaches production action-icons staging/trim; missing-type-icon and shortcut-icon tests use shared production icon cache; packaged-sibling fixture overwrites/deletes fixed TEMP/NxTools.Indexer.exe rather than an owned GUID path. These require separate isolation changes; current task does not refactor production signer/icon/cache/OneCommander initialization. Authorization switch is not permission to run them without explicit user approval of those effects.

Complete source/callee classification:session scratch platformhost-test-effects-audit.csv/.md (213entries at read-only audit checkpoint), subsequent4selection tests pure reviewed in TestEffects. All217classified; runtime LIST records189Fixture,23NativeAcceptance,5RestrictedFixture. Original Program snapshot native-selection-before.cs is dirty pre-task baseline, not HEAD. Deterministic preservation comparison found original211registrations/order/delegates unchanged,218private static methods unchanged; only4live test methods changed six print-and-return skip sites to typed TestSkippedException. No test assertions/native action sequences changed.

## Accounting
Typed skip bypasses PASS and retains reason. Runner summary:selected,run,pass,skip,fail,not_run;run=pass+skip+fail and selected=run+not_run. Fail-fast retained. Synthetic accounting regression simulates pass/skip/fail/not-run without live callbacks. Success exit0 may include skips, so SKIP never counts as native acceptance; unknown native fixture conditions that originally fail remain failures.

## Actual checks
- C# test project dotnet build --no-restore:exit0,0warnings/0errors.
- Precisely selected six test-selection regressions + seven installed native synthetic fixtures + four installed integration synthetic fixtures + parser-failure/dispatch regressions:19RUN/19PASS,0SKIP/FAIL/not_run,exit0. No claim all189fixtures executed.
- --fixtures --list:189selected,RUN0; --native --list:28selected,RUN0 (23native/5restricted); default --list byte-identical to fixture list.
- Five actual command-line guards:zero match,legacy DOpus without native mode,native without authorization,native authorization without filter,unknown option. Each exit2,RUN0,explicit error.
- GPUI native wrapper without -AuthorizeNative:rejected before process launch; no authorized wrapper run.
- platformhost-test-selection.ps1 source gate passed:build fixture argument/package inheritance,fail-closed catalog,summary/skip,PowerShell parsing,native wrapper authorization.
- git diff --check exit0. Relevant documentation/source gates recorded at final closeout.

Logs:session scratch native-selection-safe-final.log,native-selection-command-guards.log,native-selection-fixture-list.log,native-selection-native-list.log,native-selection-default-list.log. Initial red log retained. New entries instructions:csharp/NxTools.PlatformHost.Tests/README.md;specs/platformhost/index/sharedverification updated.

## Limits
No full scripts/build.ps1 run:its Rust workspace tests can scan real volumes and regenerate release manifests; user authorized C# build/exact fixtures and script selection contracts, not that broader wrapper. No Release/package/install/commit/push,real DOpus/Files/Explorer/Shell/registry/service/clipboard/global-hook/production signer/cache execution. No external GPUI acceptance run. Five restricted fixtures remain isolation follow-ups, not blockers for selection separation. Source/catalog review is not actual native acceptance or proof of all fixture outcomes. No product behavior change claimed.

Final closeout:five source scripts passed(platformhost-test-selection,clipboard-lifecycle,desktop-shell-lifecycle,indexer-lifecycle,browser-native-messaging);spec-contracts87documents/12owners passed;git diff --check exit0. Log native-selection-closeout.log. These checks do not execute live native acceptance. No remaining compiled/selection/source-gate blocker.
