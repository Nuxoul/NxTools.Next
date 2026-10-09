# Consolidated desktop UI refinement — 2026-10-09

## Outcome and authority

The user requested implementation of all listed module surfaces/common capabilities before one closeout, not a pause after a representative Tools/Foundation page. The existing staged owner remains `.trellis/tasks/10-02-staged-desktop-ui`, in_progress; canonical order remains **Foundation → Downloads → Quick Find → Clipboard + Tools → Global settings + supporting surfaces**. Historical per-stage pause statements are superseded, not new phase names. Branch/worktree and unrelated dirty changes were preserved; no commit/archive/package/install.

Product presentation, keyboard/focus and system-motion source work has been integrated across the named surfaces, with final compiled/source checks below. This is **not a claim that every requested capability or native/visual acceptance is complete**. Real screen-reader names/roles remain blocked, the screenshot matrix is partial, and retained measured/native roles remain explicit migration/acceptance debt.

## Surface/owner inventory

| Surface | Actual source delivery | Regression / preserved boundary |
| --- | --- | --- |
| Foundation/shell | Approved palette and component mapping,24/28/32 controls,3/6/8 radii,48 sidebar,36 caption; focused sidebar and real caption Button controls | Render-order Tab/ShiftTab; one GPUI KeyUp click; no domain/native placement owner change |
| Downloads list/filter/details | Continuous46px stable-ID rows, selected rail, status strip/docked inspector; detail tabs keyed Buttons; detail/segment/timeline gaps migrated4/8 | Snapshot/search/sort owner unchanged; actual list/tab/detail bounds; OLE remains still-pressed mouse-down native gesture |
| Download settings/new/task windows | Settings navigation and categories focusable, shared40px rows; media radio selection, calendar disabled/eligible keyed days, task disclosure; completed/error/actions readable and bounded | Settings984x650; ordinary560x176/278 retained; measured DASH353/HLS418/error456 footer gates; keyed progress/completion/error and close/retry generations retained |
| Schedule/delete/shutdown | Shared dialog shell,40x28 weekday Buttons, readable path/failure and fixed footer actions | Space changes actual weekday; draft/apply/result and cancellation preserved; no real shutdown exercised |
| Quick Find search/results/actions | Search focus border,40/26/48 result rails with4px title/detail gap; keyed eligible result/action/submenu/file-dialog rows, disabled skip; continuous settings navigation and deferred switches/refresh | Actual Tab/KeyUp command counting, modifier/held gates, IME/search generation and arrow/Right/Escape owner retained |
| Quick Find preview | Bounded identity/path in existing400x410 shell | One passive preview, generation controller, hidden→noactivate→final bounds→show; primary native/search focus restoration unchanged |
| Clipboard history/categories/settings | Keyed history/category/editor actions; self-drawn steppers/record entry become Buttons; focus-visible action rail, source selection/editor cancel, continuous settings and4/8 spacing | Actual300x440 viewport/input/focus; metadata24/textrow60/action rail140; local updates/deferred cross-owner work; no real clipboard captured |
| Clipboard passive preview | Shared surface/readable bounded content, reduced-motion consumption | One passive reused window; callback defer, generation and noactivate/first-show native contract unchanged |
| Tools/Link Detection | All currently registered Tools are included (Link Detection), not only a representative row: settings/row/expander/copy-first close and selected target-kind visuals | Link420 width, collapsed/expanded/footer and KeyUp copy-only synthetic test; default copy-only, opt-in browser, generation-scoped six-second hide unchanged |
| Global/supporting surfaces | Global/network settings rows/fields/footer, caption controls, notifications/dialogs/popovers; shared list/radio/command/volume/hotkey entry focus, theme/type/radius/grid | Notification KeyUp action exactly once preserves replacement generation; existing network/media/footer tests retained |

Detailed reference traces and measured freezes are in `downloads-polish.md`, `quick-find-polish.md`, `clipboard-tools-polish.md`, `motion-implementation.md` and `docs/reference-code/{download,quick-launch-research,clipboard,link-detector,gpui,ui-motion}.md`. Downloads reviewed ABDM before legacy NxTools. Quick Find evidence includes managed Listary/SwiftList boundaries only; no Listary native engine or missing Avalonia base was invented.

## Common capabilities

### System reduced motion

C# reads `SPI_GETCLIENTAREAANIMATION` through the existing system-event subscription and original EventWindow STA. Typed correlated IPC snapshots are sent after each handshake/discard. Rust global defaults unknown/failure/disconnect to reduced motion. All current desktop animation sites branch to immediate final elements; weak local observers defer repaint instead of synchronously re-entering an owner. No fake persisted flag, zero-duration frame workaround, native bounds animation or per-frame logging.

One directed integration review identified three concrete defects, all fixed: full output queue retired a deduplicated change (single latest-state pending slot now retained/coalesced and timer7/250ms retried on STA); reducing during notification exit repainted an exiting card (now omitted); visual-policy dedup hid native failure/recovery diagnostics (logging independent of visual change). Stop clears/rejects pending state. SetTimer zero return now retains correlation/state and emits one native-code diagnostic per failure episode, never replacement threads or retry-tick spam. Synthetic slot/reader/wire coverage is not proof of live preference toggles or timer allocation failure injection.

### Keyboard and focus

Reuse GPUI0.2.2 focused KeyUp ClickEvent; never add a second Enter activation framework. Shared wrappers own keyed focus and overlaid2px ring without changing content budget. Disabled categories stay selectable for settings editing; disabled actions reject input. Volume rows have one action and no inert nested checkbox tab stop. Commands select the editor, not execute. Hotkey entry on release transfers into the existing recording KeyDown owner; applying/waiting/recording states remain ineligible. Pointer drag/resize/backdrop and passive previews are deliberately not Tab actions.

### Layout migration and retained roles

Retired production SPACING_2/SPACING_6; actual radio-dot6 and border2 roles are separate. Shared rows40/group28, command rows40, common gaps4/8, compact result padding8. Downloads detail/task layouts now use4/8 and completion inset16; horizontal action8 and vertical4 preserve fixed-client containment. Static gates reject restoration of off-grid Downloads spacing and retired layout tokens. All actual containment tests remain.

Retained roles are **not blanket compliance waivers**: calendar19px days in existing six-row304x240 Popover; task/new titlebar34 and footer44; Quick Find header42/preview footer30; detail-tab29 inside30 strip; meta block21; Clipboard passive-preview10px inset paired with22px native/content chrome; switch knob2px internal clearance and genuine glyph/border/progress sizes. These require paired budget migration/native acceptance when changed, not blind global inflation. Complete raw numeric/token-role audit and supported-DPI focus/visual acceptance are not certified by this delivery.

### Real screen-reader support — genuine blocker

Pinned GPUI0.2.2 and local component API have no verified rendered role/name/accesskit or Windows WM_GETOBJECT/UIAutomation node-provider seam. Newer Zed reference has roles but that is not the pinned API. C# runs in another process and cannot simply attach a provider to GPUI controls. No tooltip/name-string claim, dependency/vendor patch, runtime replacement or cross-process provider injection was performed. Actual reader names/roles need separately scoped dependency/native-provider work and real reader acceptance; this requested capability remains unimplemented.

## Verification and causal failures

All Cargo used one build at a time with CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0, RUSTFLAGS=-C codegen-units=256 and full-operation timeouts. No compiled command timeout or duplicate build in this continuation.

Final source after the last grid changes:

| Check | Actual result / scratch log |
| --- | --- |
| `cargo test -p nxtools-desktop -- --test-threads=1` | **474 passed,0 failed,0 ignored**; `all-ui-consolidated-desktop-grid.log` |
| Production `cargo clippy -p nxtools-desktop -p nxtools-protocol -- -D warnings` | PASS; `all-ui-consolidated-clippy-production-grid.log` |
| Test configuration same packages `--tests -- -D warnings` | PASS; `all-ui-consolidated-clippy-tests-grid.log` |
| `cargo check --workspace --tests` | PASS; `all-ui-consolidated-workspace-grid.log` |
| `cargo fmt --all -- --check` | PASS; `all-ui-consolidated-fmt-grid.log` |
| Debug desktop build | PASS with no warnings; `all-ui-consolidated-preview-build-grid.log` |
| Protocol tests | Earlier integrated **40/40**, schema unchanged since; not rerun after presentation-only continuation |
| PlatformHost test project build | **0 warnings,0 errors**; `all-ui-continuation-csharp-build.log` |
| Exact C# safe synthetic test | **1 RUN/1 PASS**, preselected/actual names equal `system motion synthetic reader and wire round trip`; `all-ui-continuation-csharp-test.log` |
|11 source gates | motion,preview isolation,token,consistency,state,shell,download,QuickLaunch,clipboard,clipboard UI,file-dialog visual all PASS; `all-ui-consolidated-final-static-gates.log` |
| Specs |87 documents/12 workspace owners PASS before final report append; rerun for final docs recorded in task closeout |
| `git diff --check` | Exit0; native Git CRLF notices are not content errors; `all-ui-consolidated-diff-check.log` |

Failed attempts are preserved, not declared green:
- Earlier integration442/15 and456/3 exposed duplicate component hover, notification112px overflow, media old budgets, cached selectors/viewport fixtures and simulate_keystrokes input text injection. Causal fixes and measured assertions yielded460, then review/grid fixes461.
- Continuation first compile did not execute tests: integer ElementId inferred i32, String ElementId, Result/Option mismatch and a restored tooltip variable. These are compilation errors, not business red evidence.
- First compiled continuation **440 passed/34 failed**: primarily new Button `.hover` duplicated its component's own variant listener, plus drawing-only keyed-state API used from test callbacks and one invalid-owner result fixture. Fix in product callsites (not vendor), use real Tab paths, retain containment. Directed polish run15/4 then narrowed and corrected fixtures.
- Pinned Buttons intentionally do not take mouse focus; test-only clicks were not a valid proof of keyboard focus. Real Tab established the date/disclosure/category/notification actions. Calendar retains original production Popover bounds/disabled-pointer checks plus an isolated production calendar renderer for keyboard traversal.
- Quick Find invalid application cleared the result fixture during parent-query restore; replacement fixture uses actual synthetic Finder catalog and nonempty `row` query (empty query intentionally returns zero). No production search policy was changed to satisfy the test.
- Final production lint found needless_borrow; removed mechanically. Production and tests lint plus full final474 after grid migration pass. Previous Debug warnings from retired tokens are closed without new suppression.
- New shared selection tests were authored before implementation but the attempted red run was blocked by another module's compile error; do not claim a valid behavior-red run for them. The integrated tests are red-capable and executed green.

## Native evidence subset

`artifacts/ui-foundation/{index.html,README.md,evidence.json,*.png}` was refreshed at2026-10-09 13:22+08:00 from final Debug binary SHA256 `FF96EEF3148406761D31BF5C89B0F67832750AF94B997D968C6D973B314E3A82`. Owned-HWND PrintWindow client only: main/Tools1060x650, minimum920x480, message-level Downloads→Tab/Enter QuickFind920x480, synthetic direct-confirmation560x278. Logs show runtime/capture/indexer/platform=false and `ui.preview.stop`; ownership verified, no preview process retained. Previous PNGs preserved in scratch.

Browser screenshot succeeded this continuation; the presented gallery shows the synthetic confirmation footer/actions and no obvious crop in these subset frames. This supersedes the previous guest-unavailable observation, not historical failed capture attempts. It is not full aesthetic/state coverage or physical-pointer evidence. Filenames retain Foundation history; current hashes identify this candidate.

## Remaining acceptance / safety ledger

- Actual screen-reader provider/names/roles: blocked/unimplemented as above.
- Native100/125/150% complete surface/state visual matrix, CJK contrast, physical keyboard/pointer, passive first-show/noactivate/alpha/hit-test/topmost/timer interaction remain unaccepted. GPUI scaled/synthetic tests are not native DPI.
- Live Windows animation preference toggle/reconnect/failure/backpressure-timer acceptance remains pending; synthetic tests do not mutate preferences.
- Real download176→278 probe/move/release/C# recovery and later shrink recurrence are not closed by a directly created560x278 screenshot. Installed09:19 Validation package does not include these later source changes.
- Protected Indexer LocalSystem idle/restart/base+delta/search/hash gates remain pending and no service operation is authorized/performed.
- No ordinary runtime, real clipboard, real DOpus navigation, registry/installer/service mutation in this continuation. Preserve the historical unfiltered-harness DOpus incident disclosed in the SOL/download-window audit; do not rewrite it as no DOpus operation in all history.
- No automatic packaging/install/commit/archive. The staged task remains in_progress because native/provider and full migration acceptance are not complete. This closeout records the delivered source/check subset, not “all done except installation.”

Final documentation/evidence checks after report/task/spec updates: spec checker87 documents/12 owners PASS; git diff --check exit0; PNG hashes/byte sizes4/4 match evidence.json; isolated owned preview process count0. Branch refactor/ui-gpui-kit and HEAD70707a6 unchanged.

## Separately authorized package follow-up — 13:40
User requested a Validation acceptance package. Fixed installer AppId/directory unchanged; artifacts/installer-validation-20261004/NxTools.Next-validation-20261004-setup.exe now contains this source and later window corrections. Size15221001 bytes; SHA256147F6B76887BBBA1176F1FD840E8A5A3360013CBE5D5CFDA3B35D85A8F5B9EAF. Release/PlatformHost publish/Inno compile and PE/icon/memory gates passed; actual preflight9.81GiB≥6; five package/build hashes and build-time trusted manifest match. Generated manifest restored to prior dirty bytes; installed service state/path unchanged. Default production output untouched, no installation/launch/service/real Clipboard/DOpus operation. Earlier package-omission/no-packaging statements are historical source-delivery status, now superseded for this new candidate only. Native/provider acceptance is still incomplete; see artifacts/installer-validation-20261004/INSTALL.md.
