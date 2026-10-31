# Extensions API/UX repair — deferred interactive acceptance plan

## 1. Status and authority

**Plan only — NOT RUN.** On2026-10-25 the user selected “暂不运行原生界面” after being offered isolated native acceptance. This permits preparing this checklist, not creating/building/launching a new fixture, opening Windows dialogs, capturing desktop screenshots, operating the installed product, or repeating full tests. Existing source approval and no-deployment restrictions remain unchanged. Do not repeatedly ask for the same denied native action without a new user instruction.

Canonical source stages remain **Results and recovery → API contracts → Configuration convenience**. This is acceptance of those stages, not a fourth implementation phase or permission to add image/blob, HTTP/storage/progress services, resident W10, arbitrary UI or other capability gaps.

Latest measured compiled result: [sort closeout](../10-25-download-sort-closeout/outcomes.md) records1196PASS/0FAIL/0ignored, one full Desktop run; subsequent concurrent2line test-only selection strengthening has separate current-source3/3 and strictlint evidence. This does not freeze the worktree or establish native/visual/installed acceptance. Earlier failures remain chronological in [outcomes](outcomes.md).

## 2. Evidence already available, not rerun in this documentation step

| Evidence | What it establishes | What it does not establish |
| --- | --- | --- |
| Extension remediation10/10, settings23/23 | Script completion/status, destructive confirmation, bounded events/pages, actual GPUI settings editing, original draft/request guards | Windows dialog ownership, real foreground/focus, physical gestures, native provider or user visual approval |
| C# ExtensionSettingPathTests.WireOwnerCancellationAndSelection,1selected/1run/1pass/0skip | Strict wire, injected owner/existence/presentation, selection/folder/cancel/dismiss result logic | Real HWND validity, actual file/folder dialogs, OS cancel/focus lifecycle |
| Same-turn A→B→A regression | Real Input Change events plus original fakeIPC result cannot overwrite a changed-back draft | A physical modal-dialog race or proof an executing Win32 call stopped |
| SDK/schema/examples and offline validator | Bounded authoring/transport/structural contract coverage | Generic JSON Schema semantic validation, sandbox/security certification or arbitrary extension safety |
| Latest strict Clippy/workspace and full Desktop | Executed compiled fixture/integration checks | Release/native service/installed acceptance, all capability completeness |

Exact source seams: [path request/admission](../../../rust/apps/nxtools-desktop/src/main_window/extensions_page/setting_paths.rs), [settings and regression tests](../../../rust/apps/nxtools-desktop/src/main_window/extensions_settings.rs), [C# native picker](../../../csharp/NxTools.PlatformHost/Windows/ExtensionSettingPathPicker.cs), [injected C# test](../../../csharp/NxTools.PlatformHost.Tests/ExtensionSettingPathTests.cs). Current test is explicitly injected; never relabel it as a live picker test.

## 3. Required isolation design before any future execution

No ready-to-run interactive acceptance entry is delivered by this plan. A later separately approved implementation must first prove these guards; do not invent a fixture flag or launch the ordinary Desktop/PlatformHost hoping environment variables isolate it.

1. Use a unique session-scratch root containing only synthetic package/config/history/files, plus the exact owned fixture executables. Create fixtures A.txt/B.txt and folder A/folder B only within that root. No real user data or production extension directories.
2. Instantiate the original ExtensionManagerView/ExtensionApplication/Core transaction and original native picker/STA/pipe handlers, rather than copying their logic. Source HWND belongs to the owned fixture process; authenticated peer PID must match. The normal PlatformHost startup must not install hooks/tray/clipboard capture/service integrations.
3. Use a dedicated authenticated fixture pipe, no production singleton/summon endpoint. Confirm all configuration/storage/log/runtime destinations resolve to scratch before creating a window. Fail closed if isolation cannot be demonstrated.
4. No real clipboard reads/writes, credentials, external network, user packages, shell open/program launch, Registry/COM registration, services, indexer or protected runtime. Script cases may use only existing audited synthetic fixtures and isolated child-process supervision; no third-party package execution.
5. Verify executable path/PID/start identity and owned HWND before every GUI action. Never target NxTools by process name alone. Existing abnormal main window remains untouched: no activation, move, resize, input, hide/show or restart.
6. Terminal cleanup retires owned dialogs/windows/pipe/child workers and deletes only the proven scratch tree. Logical cancellation does not free an STA reservation until actual modal exit; never spawn a replacement for a stuck Win32 operation. Hung/ambiguous cleanup stops the case and retains evidence.
7. Freeze only this case's inputs/source hashes for evidence; do not ask other sessions to pause or terminate their processes. If new Rust verification becomes necessary, natural idle/preflight and CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256 apply. Build/package/install is not authorized by this plan.

## 4. Proposed cases and pass/fail boundaries

Every row below is **NOT RUN — deferred by user**. Existing synthetic evidence remains separately named above. A missing fixture seam or unavailable environment is BLOCKED/NOT RUN, never PASS. Record assertion failures as FAIL even if a later narrower case passes.

### A. Owned native picker and focus (first execution slice)

| ID | Action using owned fixture only | Required observations |
| --- | --- | --- |
| N01 | From file setting A.txt, click the real 浏览 action and select B.txt | One real native file dialog owned by exact source HWND/PID; existing file only/single selection; draft becomes B.txt; scratch config bytes unchanged until Save; dialog exits and source window is interactive |
| N02 | Repeat for folder A→folder B | Actual folder dialog, existing folder only, no new-folder creation; no file result in folder field; draft-only update |
| N03 | Dismiss with Cancel, Escape and window close, separately | Quiet terminal dismissal; original draft/config unchanged; pending action clears; Browse usable again after actual exit; no extra dialog or second terminal result |
| N04 | Start from missing/empty current path | Picker remains usable without changing draft first; selecting a valid owned file succeeds; inaccessible initial directory yields bounded actionable error or usable fallback, not false success |
| N05 | After select/cancel, Tab/Shift+Tab and edit the original field | Same source fixture foreground policy, original GPUI input entity/IME owner retained; actual focus/readable focus indicator/no stolen focus. Do not require exact focus-return control unless original contract establishes it; observe reachable input and no lost keyboard input |
| N06 | Retire/disconnect owned origin while its dialog is pending through an approved fixture control | Native cancellation/cleanup belongs to the original dialog owner; no late draft resurrection/save/retired HWND access. No process kill, external input or timed-out-worker replacement used as proof |
| N07 | While dialog is open, send one approved inert fixture diagnostic/health request through original serial channel | Reader remains responsive while detached STA waits; report measured bounded round-trip for this case only, not a general latency claim. No clipboard/hotkey/native side effect used as a responsiveness probe |

### B. Deterministic stale-result and draft guards

These are fixture-controlled interleavings, not physical modal acceptance. A modal native dialog can disable its source window; do not pretend a user can type A→B→A into that disabled field. Retain the already green real InputEvent regression. Any future end-to-end interleaving must use a clearly labeled controlled completion/barrier on the original owner, not forced focus, a second UI controller or test-only admission bypass.

| ID | Controlled boundary | Required observations |
| --- | --- | --- |
| G01 | A→B→A through original input events before original request result is admitted | Older selection rejected although final text equals A; current value/config retained and exact pending cleaned |
| G02 | Retire draft, switch package, change content hash or deliver foreign request ID | Result cannot update another draft/package; no auto-save/trust/enable; current newer pending not cleared by foreign reply |
| G03 | Native failure/disconnect followed by fresh explicit retry | Typed actionable failure retains original value; original slot released only after real exit; a new user action gets a new request ID; no automatic effect retry |

### C. F1–F9 user workflows under temporary state

| ID / audit | Representative task | Required observations |
| --- | --- | --- |
| U01 / F1,F3 | Run audited synthetic completion fixtures for success/handled/not_handled/error and infrastructure failure | Success and handled agree; bounded plugin explanation labeled as plugin text; infrastructure/native failure distinct; cancellation truth retained; no raw stderr/payload diagnostics |
| U02 / F2 | Open removal review with synthetic configured chains, cancel, then confirm current review | Opening/cancel changes no config or run ownership; disclosure names affected configuration and whole chains; one explicit confirm commits current reviewed registry only; stale review refuses; no secure-delete or credential-cleanup guarantee |
| U03 / F6 | Synthetic new events fail to pause, inspect activity, recover, then deliver a new event | Bounded metadata/cause/time/correlation; actionable environment/settings entry; recovery waits for new event, no replay; no event payload shown in history |
| U04 / F5 | Synthetic text/files/image/download selections with legacy and declared accepts | Relevant menu actions only/conservative counts; incompatible selection cannot launch interpreter; final current full input still validates; legacy package behavior preserved |
| U05 / F8 | 使用方法→打开设置, saved resolver host selection, edit→Cancel and edit→Save | Direct actual editable page, no extra Edit detour; original parked draft/input identity; original saved route loaded, no implicit save; single explicit owned-section commit |
| U06 / F4,F7,F9 | Offline author validates package/requires, discovers methods, traverses>32 synthetic items and sees local search scope | Formal parser rejects invalid package/requirements; capability known versus currently available distinct; bounded cursors/current identity/stale semantics; local catalog does not claim indexed/provider completeness. Offline aids not sandbox certification |

### D. Visual, keyboard and accessibility observations

- At default and actual supported minimum fixture viewport, Light/Dark, short/long/empty/error/busy states: current catalog cells, recognizable outlined header actions, clear warnings/status, usable input widths, scroll-to-last control and reachable Save/Cancel. Compare actual screenshots to approved current component/layout brief, not historical measurements alone.
- Actual pointer plus Tab/Shift+Tab/Enter/Space/Escape: one action on release, no duplicate submission, original modal priority and focus return. Capture whether visual focus is readable without changing production palette solely to pass a fixture.
- Synthetic capture is not physical acceptance. Real Windows IME,100/125/150% or mixed-monitor DPI and screen reader/native UIA each require a supported environment and separately authorized operation. If unavailable/deferred, mark NOT RUN rather than failing unrelated logic or granting PASS.
- Screenshot only the exact fixture window/dialog. No full-desktop/user data capture. AI image inspection is technical evidence, not user aesthetic approval. Native provider observations are not screen-reader usability acceptance.

## 5. Evidence record and exit criteria

Per case capture: case ID, source revision plus source/executable hashes, fixture PID/start/HWND/pipe identity (internal evidence only), OS/theme/DPI/viewport, exact synthetic dataset revision, action sequence, expected/actual, PASS/FAIL/SKIP/NOT RUN/BLOCKED, safe correlation/event sequence/native code/elapsed, and exact cleanup outcome. Store screenshot/video artifacts only after capture authority, within scratch; diagnostics never contain file paths, input text, URLs, credentials, clipboard payload or raw exceptions. Use IDs/kinds/counts/fixed reasons; scratch config bytes may be hashed rather than copied into logs.

No acceptance row is passed by this document. To close the first native slice later: N01–N07 execute with no unclassified effect, exact owner/current request, real dialog exit, draft-only/no-write and focus/input evidence; retain actual failures and cleanup reservations. G/U/D rows have their own evidence levels and may remain open. Overall F1–F9 acceptance remains pending until required user tasks and supported state/interaction matrix are observed and reviewed; full test green alone is insufficient. No automatic package/install/commit/archive follows acceptance.

Installed workflow, real user clipboard/credentials/network and Release/protected tuple/reboot/service tests are excluded. They require new explicit authority and a separate data/recovery plan. Do not use an installed run as an isolation shortcut.

## 6. Resumption handoff

Current authorized action ends with this plan and documentation checks. When the user later chooses to resume native work, first confirm the exact slice and isolation/capture boundaries; inspect whether the isolated entry now exists, design only the missing guarded seam, then build/check under workstation policy and execute only approved case IDs. Follow [C# selection/effect rules](../../../csharp/NxTools.PlatformHost.Tests/README.md) with exact LIST→RUN accounting; a new native test must be registered/classified before execution. Do not provide an invented command or reclassify the existing injected fixture to bypass authorization.
