# Remaining feature-first UI layout migration — 2026-10-09

## Outcome and scope
The user requested continuation of the remaining whole-application layout work after the first feature-first slice. This continuation implements the remaining Downloads auxiliary presentation, advanced Quick Find components, compact Clipboard editors, notifications, and browser companion presentation. Existing product functionality is authoritative, not the simplified browser prototype. Canonical integration order remains Foundation → Downloads → Quick Find → Clipboard + Tools → Global settings + supporting surfaces. No new product feature, persistence/native owner, dependency or palette is introduced.

All changes remain local and uncommitted after remote checkpoint `42f60a8ee9e78402461d5c42f5b55ca5cc254c18` / `checkpoint/pre-ui-layout-20261009`. The 19:30 Validation installer is unchanged and does **not** contain these layout changes. No packaging, installation, ordinary runtime, service operation, real Clipboard history/config enumeration, user-URL request or DOpus navigation was performed in this continuation.

## Implemented surfaces
| Family | Actual change | Preserved boundary |
| --- | --- | --- |
| New download | URL paste/directory chooser match 32px fields; compact 12/18 guidance and probe status/capability; 14/20 local heading | Ordinary 560×176 → fresh 560×278; media353/418/456; pending freeze, draft/request/idempotency, fixed handoff/presentation retry unchanged |
| Task window | 12/18 metrics/path/details; 36px bounded metrics; explicit nonshrinking regions; source/destination and cancellation selectors | Progress/completed440×184, expanded280, failed196; same HWND/generation, content-only ordinary completion, one centered native resize owner and pressed-pointer OLE unchanged |
| Schedule/delete/shutdown | Aligned28px switch/time rails; readable supporting copy; bounded36px validation with complete-error tooltip; readable deletion consequence/path; deliberate36px shutdown explanation slot | Schedule430×326, shutdown450×200; independent start/stop/weekdays, draft validation, Apply/Cancel and correlated one-shot shutdown/delete semantics unchanged |
| Advanced Quick Find | Workbench13/20 +12/18; 32px command inputs/actions/picker, separate end-aligned Save row; fixed40px master rows; bounded196px volume status; stable paths/diagnostic values/actions | All five command fields/eight actions, enabled/admin/silent, volume automatic/manual, exclusions/picker/key policy, existing metrics/log actions/draft generations preserved |
| Clipboard editors | Compact12/18; nonshrinking input/footer; flexible bounded source viewport; long names retain full tooltip | Actual300×440 client and244px editor; source selection, occlusion, focus/IME/Escape/save/cancel, paging/worker/passive-preview owners retained |
| Notifications | Title13/20, message12/18; complete text/details in bounded scroll area, separate28px action | Max3, width336/max-height112; original generation dismissal, action/deferred center update and reduced-motion policy |
| Browser companion | Existing456px popup,420/445 workspace;13/20 body,12/18 support,32px forms/actions,28px compact actions; continuous rows, wrapped messages; two-level shadow resource copy and reduced motion | All Popup IDs/four views/controllers/handoff/CSP/manifest unchanged; palette token files unchanged |

## Verification actually executed
- Final `cargo test -p nxtools-desktop -- --test-threads=1`: **494 passed,0 failed,0 ignored**,23.20s test runtime after complete-error tooltip closeout. Six new tests relative to488: four auxiliary actions/budgets, advanced minimum/long/empty/disabled components, and long notification/action. Existing Clipboard category regression additionally covers actual editor/footer bounds and original keyboard actions.
- Production and tests strict Desktop Clippy `-D warnings`, `cargo check --workspace --tests`, `cargo fmt --all -- --check`, and Debug build passed. Every Cargo command inherited jobs1/incremental0/codegen-units256 and ran serially after writers settled.
- Extension existing Node suite: **21 passed,0 failed,0 skipped**. Layout harness `browser_extension/nxtools-interceptor/tests/layout-browser.cjs`: **38 checks,0 failures**, real Chrome DOM of product HTML/CSS with scripts removed, fresh scratch profile and HTTP/WS blocked. Normal/long/empty four views, overflow, bounded workspace, media field/action baseline, reachable Save, toggles, shadow hierarchy and reduced motion; not installed extension/controller execution.
- Static preservation comparison against42f60a8: **24 surfaces**, **11 event-callback surfaces**, **10 byte-unchanged owners**,0 omissions. Original Popup IDs retained. Downloads parent implementation tail unchanged; import cleanup only. No C#/Core/Protocol/browser controller change. This is omission evidence, not semantic/native certification.
- Eleven affected architecture gates passed across the verification runs: token boundaries, consistency, UI state, motion, preview isolation, desktop shell, download lifecycle, Quick Find lifecycle, Clipboard UI lifecycle, attached search visual and window-follow. Quick Find's old28px component assertions were synchronized to the measured32px workbench actions; all lifecycle/picker/generation/diagnostic checks retained.
- Final same-batch eleven architecture gates passed; spec-contracts checks87 documents/12 workspace owners, and its checker suite passes10 tests. Diff check passes. Document/source checks do not certify installed visual/native behavior.

Temporary command logs and full preservation/DOM reports are in this session's scratch directory (`remaining-closeout-*`, `remaining-desktop-*`, `remaining-preservation.json`, `remaining-extension-layout.json`). They are not installed product diagnostics.

## Failure evidence and concrete fixes
1. Initial integration did not execute tests: missing compact line-height/type imports and non-static test debug selectors caused17 compiler errors; unused imports also removed. This was **not** a behavior-red result. A later Debug preview import error was similarly fixed before successful final build.
2. First compiled suite:489 passed/5 failed. Schedule and expanded task fixtures relied on `window.resize()` although the GPUI test viewport remained1920×1080; explicit `simulate_resize` now exercises430×326 and440×184/280, and existing geometry/action assertions remain. Shutdown reserves its deliberate two18px lines instead of depending on synthetic font wrapping.
3. Real notification action displacement and long Clipboard source-list overflow were caught by production bounds regressions. The dependency scrollbar wrapper did not bound the intended inner viewport. Explicit GPUI `overflow_y_scroll()` on the identified constrained container fixes both; notification action wrapper cannot shrink. No card/window inflation, omitted copy/detail/control, or relaxed containment assertion.
4. Browser DOM found a real2px range-input overflow; local `min-width:0;margin:0` fixes it. Historical browser metadata/font expectations were updated to inherited12/18 without dropping security/behavior checks.
5. Static callback extraction initially misclassified expression closures by searching for the next brace. Parsing the complete event-call parentheses resolves the false positives; all11 call bodies match. Quick Find source gates initially failed on old action-tier references; only intentional visual contract assertions changed.

New delegate regressions were added before presentation changes but not executed before those changes; there is **no** claim of an executed pre-change red for every surface. The actual compiled489/5 failing run and subsequent concrete fixes are retained.

## Native captures and honest acceptance limits
Five current Debug production-render captures use only owned HWND/PID and synthetic/in-memory owners:
- Confirmation560×278;
- Schedule430×326;
- Shutdown450×200;
- Progress440×184;
- Main with long notification1060×650.

Files/hashes/binary identity are in `artifacts/ui-layout-quality/remaining-native/evidence.json`; gallery `remaining-native/index.html`. Browser companion captures/gallery are under `remaining-browser/`. Native client dimensions were read back by the capture helper; all preview processes were closed. These are direct synthetic render captures, **not** probe handoff, physical-pointer, installed extension, service or mixed-display lifecycle proof.

The work-panel screenshot tool twice captured the PI client/another session instead of the requested gallery. Those screenshots are excluded. This continuation therefore does **not** claim direct visual inspection of the new PNGs; evidence marks `inspected:false`. Advanced components/Clipboard overlays have executed production GPUI geometry/focus/action evidence, not new owned native screenshots. Prior first-slice inspected images remain separate historical evidence.

Source/compiled scope is complete for the requested remaining presentation slice. Whole-application installed/user visual acceptance is still open:100/125/150% and mixed displays, complete native state/physical gesture matrix, actual passive preview/OLE/focus behavior, live motion changes, and user approval. Pinned GPUI0.2.2 has no verified rendered screen-reader/provider seam; HTML ARIA/tooltips are not native accessibility. No provider/dependency/service work is authorized by this styling task.
