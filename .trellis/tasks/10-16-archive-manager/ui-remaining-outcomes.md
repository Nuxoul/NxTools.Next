# Remaining reference UI continuation — actual changes and acceptance limits

2026-10-25. Current state belongs only to [the checkpoint](phase-a-checkpoint.md).
This is implementation of remaining real controls/consumers, **not reference-image or whole UI
acceptance**. UI-AC01/02 and the association/classic-menu source/isolated acceptance stay CLOSED.
Their five historical reports are not rewritten. No canonical A–F phase names/order are changed.

## Reference differences and actual correction

| Requirement | Starting concrete difference | Actual change / retained owner | Disposition |
| --- | --- | --- | --- |
| Recent archives | No list/menu/config consumer despite design default10/range0–30 | Original Core adds remember_recent/recent_limit; original UiStateStore stores bounded paths separately from config; successful top-level read records MRU; actual Settings list/Open/Clear and original File→Recent submenu; clear waits durable writer receipt | Implemented; final automated counts below. Not visual/user acceptance |
| Temporary folder | Production cache root fixed; no setting | Original Advanced Settings Input/Browse/Save/Default/Reload draft controls; native materialize policy selects only dedicated owned namespace under chosen base; original cache retains bounded old-root identities for recovery | Implemented; real custom-root behavior proven with owned temporary paths only |
| Temporary retention | Fixed24h orphan expiry | Real dropdown1/6/24/72/168h, snapshotted in each new native lease expiry | Implemented; eligibility, not a guaranteed deletion deadline |
| External program exit cleanup | Generic Shell launch discarded Process; no lifetime consumer | Real Switch “可追踪子进程退出后尝试清理”; original entry session's lease-only Open captures actual returned process, keeps external hold and asynchronously observes exit; original safe release only | Implemented with explicit tracked-process limitation; actual external program NOT RUN |
| File/folder/parent icons and long names | Existing three icons/32DIP rows/columns already present; ellipsized file name lacked full-value hover hint | Retained icons/row/selection/drag owners; added official full cell text Tooltip; no Shell icon IO or engine change | Source/automation substantiated, reference appearance unaccepted |
| Tree/split | Existing official Tree/ListItem/public split constraints, no physical gesture proof | Retained depth/disclosure/icon/selection/generation; added real handle press→move→release regression, verifies width changes and160–280 range/details containment | Automated event drag PASS in earlier current run; physical native drag NOT RUN |
| Toolbar/menu | Existing icon-over-label outlined64DIP official toolbar and menu owners | Retained original toolbar/actions; File menu inserts real Recent after Open, before New, using official submenu; failure to Clear shows original-window error without marking valid archive stale | Existing hierarchy representative retained; original PNG visual match unaccepted |
| Scroll | Original X/Y handle restrictions already repaired | No replacement gesture owner; real horizontal scrollbar press→move→release test checks X moves/Y unchanged; original vertical-wheel representative included | Automated event regression; physical mouse/trackpad NOT RUN |

Reference resource recheck: public Bandizip SimpChinese.lang314/407–408/791–799 supplies recent/clear/
temp/browse/child-exit vocabulary only, not its closed-source lifecycle. Tracked legacy NxTools still
has package/FFmpeg ZIP rather than a general archive UI. Archiver constructor115→ui.run158→recursive
PID-temp cleanup159 and selected extraction697–715→open_path967–986 were read; unsafe global recursive
cleanup/launch inference were not copied. SupaZip lifecycle was inspected in the bounded explorer.
Exact source chain and limitations are appended to [archive reference](../../../docs/reference-code/archive.md).

### Actual component composition

Current Kit/Base/Component/Assets0.8.0/defaultpre0.3.9 unchanged. Original Settings/Page/Normal Group/
Item/Field remains the settings hierarchy/search/scroll owner. General adds a Recent group with
real Switch, `SettingField::number_input` default32DIP/min0/max30/step1, `ListItem` and actual Small
outlined Open/Clear Buttons. Advanced adds Temp group with original stable `InputState`/official
Input32DIP, public render layout and Small outlined Browse/Save/Default/Reload actions, official
scrollable dropdown32DIP and Switch. Parent flex-col/w_full/gap2 and wrapped action row arrange
controls, never repaint substitutes. Native failure remains visible, optional full path uses Tooltip.
Official `PopupMenu::submenu` uses immutable recent/state snapshot before mutating menu Context.
Existing `Toolbar.content` continues to bypass child ghost/compact transformation; icon-over-label
Medium outlined Buttons64DIP and official DropdownButton remain. No vendor/dependency/palette/Fast change.

## Persistence, lifecycle and safety

- Recent data uses the **same original UI-state file and bounded writer thread**, additive missing-field
  default empty; Core stores only choices. Maximum30 records/128KiB raw path bytes/1MiB file.
  Dedup and bounded MRU, no render IO. Only validated current successful top-level read with native
  browser identity/cleanup enters history; admission, failure and nested temporary source do not.
- Geometry saves merge only their presentation fields and preserve newer history, so late browser
  close cannot resurrect Clear. Original corrupt-state preservation/write lockout remains. Actual
  Clear uses bounded flush receipts and reports native/storage-owner failure, not queue admission
  as durable success. One bounded state-change signal updates original app/settings/browser entities.
- Temp policy is additive `ArchiveSettings.temp` and materialize-only `ArchiveEntryCommand.temp_policy`:
  `{base_directory,retention_hours,cleanup_on_child_exit}`. Legacy empty/24h/off remains; invalid
  path/retention/unknown fields reject. Local drive-qualified only; no UNC/device/traversal/reserved
  components/reparse authority. Root changes affect new leases, never move active output/jobs.
- Temp directory Input keeps independent baseline/revision. Browsing fills only unchanged live draft;
  cancel/late selection does not save. Explicit save merges under original config lock; unedited
  save adopts newest disk, dirty conflict keeps newer disk/local draft. Retention/exit merge only
  changed nested fields, original successful save ordering/deferred publication remains unchanged.
- C# owns Windows APIs: `ArchiveEntrySession` receives policy, original Cache creates owned namespace,
  original runner/source/output/cleanup/Seal complete before Desktop may request lease-only `open`.
  No arbitrary executable/path argument on Open. Active-content confirmation and original
  snapshot/session/generation/budget/password/source/output/selection/cancel owners remain.
- Same cache owner stores at most16 durable custom-root identity slots in stable default location.
  Old roots remain discoverable after restart, no eviction/overwriting old authority; capacity,
  corrupt/changed roots fail closed. Fair recovery keeps original shared8-entry filesystem budget.
  Manifestv2/fileID/hash/mtime/attributes/MOTW/held ancestry and exact-file+empty-directory retirement
  remain deletion authority. User-selected base/namespace ancestors are never disposable.
- Native lease Open establishes external hold before detached bounded STA launch, captures only
  actual Process returned from original ordinary Shell start. Prompt return does not await viewer.
  On tracked exit: optionon attempts safe retirement even while browser remains; optionoff retires
  only after browser request. Null/unusable handle/short-lived launcher cannot prove document lifetime;
  fallback remains original browser retirement/expiry and exact edited/locked-file checks.
  Disconnect cancels observation, not the external program, and never blocks drain on it.
- Production diagnostics use correlation/start/terminal/elapsed/native code and bounded fixed states,
  no path/payload/secret logs, no pointer/frame/byte ticks. ProductApplyAuthorized=false remains.

## Actual checks and evidence

Main evidence: session scratch `archive-ui-continuation-20261025-160802`.
Native isolated evidence: same session `archive-temp-csharp` (fresh scratch C# source/build).
Every Cargo checks shared channel/process state itself, jobs1/incremental0/codegen256, locked/offline;
no reservation/pause/termination asked of others. Other sessions' builds/test runs are not this task's
verification. Counts below describe distinct selected collections, not a full product matrix.

| Check | Observed result / evidence |
| --- | --- |
| Actual red: recent defaults | 01-core-red.log FAIL Null vs true, exit101 |
| Actual red: state consumer | 02-ui-state-red.log FAIL recent field discarded, exit101 |
| Actual red: lease Open wire | 07-temp-protocol-red.log FAIL unsupported open, exit101 |
| Core recent/temp | 10-layer-tests.log 2PASS; final latest path checks in17-final-layers.log |
| Protocol temp/Open | 10-layer-tests.log 1PASS; final latest path checks in17-final-layers.log |
| Desktop remaining controls/owners | 11-desktop-tests.log9PASS; initial final12 collection9PASS/1FAIL due test scheduler registration order; fix allows real-thread activity BEFORE spawning original store (not product suppression). Final18 result recorded below |
| Original materialize/lease Open representative | 13-original-entry-tests.log1PASS, strategy policy/correlation/lease-only target and original release assertions |
| Strict Core/Protocol/IPC/Desktop tests Clippy | 14-strict-tests.log PASS, -Dwarnings at measured source interval |
| Production strict | Existing unrelated floating_assistant/media.rs::animated remains out of scope; final production check is separate, never infer global strict green from --tests |
| C# fresh build | green-build-final.log0warnings/0errors; nine task files SHA256-match tested copies, parent independent hash comparison mismatch0 |
| C# exact LIST/RUN | final-list.log/green-tests-final.log:18selected=18run=18PASS,skip/fail/not_run0;11new temp fixtures +7 original cache/browser representatives |
| C# red/mutation | strict policy/open actual red each1FAIL; four scratch-only boundary mutations eachFAIL, workspace never mutated |
| Static gates | spec114docs/14owners,catalog79,GPUIrelease,Markdownprovenance,PhaseF-shell PASS (15-static-gates.log); task82/79 context PASS with existing size warnings |
| Scoped format/diff and preservation | final audit below; no whole-workspace frozen/HEAD-diff ownership claim |

The new split/scroll tests use actual mounted handle/thumb press-move-release events, not direct
set_offset as drag proof. Existing wheel/icon hierarchy representatives only protect relevant changed
surfaces; UIAC/Shell CLOSED collections/full Desktop/codec/password/CLI/100k/resource matrices not run.
The scheduler failure is retained in12 raw log: original writer first notification arrived before
`mount` called allow_parking; moving that call before registration keeps real original store/controls,
not a mock consumer. It does not establish arbitrary native scheduler timing.

## Visual / physical / user acceptance

**Original seven PNGs were NOT actually inspected this round.** No direct image-view capability was
available; previous static-browser screenshot failure was not retried. Dimensions/OCR/accessible tree
were not treated as viewing images. No request to resend existing images. No self-app GUI restriction
bypass, alternate product launch or substitute screenshot acceptance.

Thus page structure/order/alignment/spacing relative to each original reference, exact icon appearance,
row/long-name readability and toolbar visual restoration remain **UNACCEPTED / VISUAL BLOCKED**.
Real GPUI/native physical mouse/keyboard, splitter/thumb/trackpad,100/125/150/mixed-DPI/IME/UIA and user
visual approval are **NOT RUN**. Synthetic production view/action tests do not close those gates.
No actual Windows default-app launch/Registry/COM/Explorer integration, package/install/formal product/
service/updater/protected-directory/release-ledger/UNC/source recycle/P209/commit/push operations.

## Preservation and stop

Opening baseline3323source/workspace files, exact original1332-line checkpoint, status and five prior
reports were saved. All old reports remain byte-identical; sole checkpoint receives only additive
current status with full original suffix. Concurrent archive forms/forms_tests/job_tests/progress and
downloads polish_tests/schedule_view changes are separately retained, not attributed to this slice.
Small extra field `temp_policy:None` in existing drag/authorization constructors is schema compatibility
only, not a new drag feature or controller. No reset/overwrite/dependency/vendor/engine rewrite.

**Real behavior gaps implemented; original reference UI as a whole is NOT accepted.** This delivery
must not be relabeled “已按参考图完成” or A–F/first-release PASS. Exact remaining blocker is original-image
visual comparison and applicable real physical/user acceptance, not a vague future optimization.

## Final measured closeout

- `17-final-layers.log`: Core2PASS/0FAIL (86filtered), Protocol1PASS/0FAIL (80filtered), exit0;
  current reserved-name/repeated-separator/traversal/legacy/save-reopen cases included.
- `18-final-desktop.log`: **10PASS/0FAIL/0ignored**,1222filtered,5.01s,exit0. Actual recent list
  Open→original read and durable Clear now pass after pre-registration real-thread test setup.
- `18-final-representatives.log`: original materialize/policy/leaseOpen1PASS, vertical-wheel1PASS,
  mounted icons/toolbar1PASS. These3 are separate from the10, no closed UIAC/Shell collection rerun.
- Same current test artifact SHA256 before/after those representatives:
  `E5345CECC8710BB4BC64972FFD4D73D20AAB4EBEB7FDE32797D9682D92DD77E5`.
  Shared target may later be replaced; the saved hash/log interval is evidence, not its future state.
- `14-strict-tests.log`: affected Core/Protocol/IPC/Desktop tests Clippy exit0/-Dwarnings; not global
  production strict green. Existing production animated-field limitation remains explicitly retained.
- C# parent checked all9 `final-tested-hashes.csv` entries against workspace, mismatch0; exact18
  list/run and0warning/0error build confirmed from raw logs. No real external viewer/native launch.

No further feature/review/build/package cycle is implied. Reference visuals and physical/user gates
remain specifically unaccepted despite the completed real functionality and automated checks.

Final source/document closeout `19-final-static-results.json`: scopedrustfmt/diff/spec/taskcontext/
preservation all exit0. Opening3323files/current32changedexistingpaths, five historicalreporthashes
unchanged, original1332checkpointlines retained; six explicit concurrent paths preserved separately.
Production strict was NOT rerun this round: its known unrelated animated-field blocker is retained,
not newly measured here and never called PASS. No remaining task test/build process is owned by this slice.
