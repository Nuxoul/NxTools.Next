# Steward launcher reference review

Verified2026-10-11 from user-supplied `D:\Code\steward-master`. No Git metadata in
snapshot. MIT license (`LICENSE`, copyright2026 Steward contributors). Stack is
Component/Base0.7.0 with GPUI/Platform0.3.7, Rust1.97.1, not Next's Kit0.7.1/
GPUI0.3.8. No dependency/toolchain change or code copying authorized by review.

## Authority and evidence
User requests Quick Find before Downloads and Steward-aligned UI, plus evaluation
of useful features/performance standards. Steward is visual/engineering precedent;
existing Listary/SwiftList indexed interaction and Next native contracts remain
unless user explicitly approves a different behavior. Legacy NxTools has no Quick
Find surface (see quick-launch-research.md); Listary evidence is managed boundary
only, not native engine source.

Viewed supplied static `assets/1.png` through review gallery and attached screenshot:
one borderless search field, continuous single-line icon/name/right-type rows,
subtle selection wash and translucent dark surface. The supplied image is not a
current running build, DPI measurement or proof of latest file-result rendering.
Gallery: artifacts/steward-reference-review/index.html. No Steward/Next executable
built/launched, benchmarks/tests run, index/store/user data read, or services changed.

## Verified source entry chain
Paths are relative to Steward root.

| Source | Verified role |
| --- | --- |
| README_CN.md, Cargo.toml, LICENSE, rust-toolchain.toml | Product goals, stack and MIT boundary |
| crates/app/src/config.rs:11-30,41-49 |760px width/60px search/4px drag margin,42px rows,8 visible rows,4 plugin rows,hide rather than destroy on Windows |
| crates/app/src/window.rs::init_ui_common/open_launcher_window:29-124,390-423 | Explicit quit, component initialization, one shared focus handle, hidden show:false/focus:false PopUp, owned activation subscription and initial frequent-app query |
| crates/app/src/window.rs::toggle_launcher/show_launcher/hide_window/show_window:426-539 | Reuse existing handle or recreate after close; release shared-state borrow before UI update; explicit native resize/show/foreground/input focus; hide resets presentation |
| crates/app/src/platform.rs::show:206-270 | Monitor-under-cursor bounds, backdrop sample while hidden, cross-DPI visible-first move then exact client correction/foreground |
| crates/app/src/launcher.rs::handle_key/search_unless_composing/search/search_files:1698-1769,1902,1922,2128-2204 | Handwritten input/IME gate; application search and background file channel; file search80ms dispatch coalescing; separate home/file page |
| crates/app/src/launcher.rs::render_merged:2493-2600 | Merge ranked heterogeneous results, freshness/IME confirm gate, skip unchanged target height, async GPUI resize |
| crates/ui-components/src/results_list.rs:119-187,201-219,294-303,313-495,641-681 |42px single-line rows/24px icons; visible selected-anchored slice; name flexible,detail280/size64 when applicable; selection/hover independent; pointer selection+confirmation; keyboard confirm gate |
| crates/core-engine/src/lib.rs::Engine/search_haystacks | Reusable nucleo Matcher, precomputed name/pinyin haystacks, logarithmic usage weighting |
| crates/core-engine/src/file_index/query.rs; tests/file_search_matching.rs:42-101 | case/nocase/path/regex, ext/size/date/type and boolean/filter DSL; exact name>prefix>substring>weak ranking fixtures |
| crates/app/src/file_index.rs:31,99-130,364-381 | File top12 result bound, worker/generation/cancellation request seam |
| crates/app/src/app_index.rs:24-85 | Start-menu1s trailing debounce,30s UWP reconcile, unchanged-set suppress rebuild/write |
| crates/app/src/search_input.rs:268-314 | IME marked-text replacement/cancel tests inspected, not run |
| scripts/bench-resident.ps1:106-165 | PID checked synthetic WM_HOTKEY->visible polling5ms; working set/private bytes; name-only Stop-Process at146 prevents safe reuse here |
| crates/core-engine/tests/file_index_bench.rs:20-95 | ignored manual synthetic300k/default, top12,20samples/needle p50/p95 and index memory/build report; no measured-latency gate |
| crates/plugin-host/tests/scaling.rs:198-243 |1vs100 plugin routing/load cost;5/10s loose ceilings and20x+2s ratio, correctness guard not100ms user-experience certification |

## What can transfer
- Search-first hierarchy, continuous list, stable icon/name/metadata columns, subtle
  distinct selection/hover; no redundant cards or permanent feature descriptions.
- Keep selection stable during identical-result/icon updates; bound render work to
  viewport, skip unchanged geometry and duplicate state updates.
- IME composition must not execute results; current query/generation, not displayed
  old rows alone, owns eligibility. Pointer and keyboard need the SAME gate.
- Separate process cold start, first/warm summon, query completion, visible first
  frame and input-focus readiness. Measure working set and private commit separately.
- Precomputed match data/cancellation/cache-first scanning are useful engineering
  patterns; assess existing Next equivalent before adding anything.

## Do not transplant blindly
-760x60/42px/8row geometry, default frequent-app home, file-search subpage, implicit
  calculator/link routing change Next behavior and need product approval.
- Handwritten input/per-frame IME reassociation and in-process Win32 are not a
  substitute for Next Kit input and C# native adapter ownership.
- Cross-DPI show-on-old-monitor-first and search-driven resize are not a passive
  preview hidden->noactivate->final device bounds->show implementation.
- Acrylic/background sampling, violet accent and all-window scale entrance are
  visual choices, not approved Next palette/motion/native budgets.
- Source asymmetry: keyboard confirm_selected checks confirmable; render_row pointer
  callback lacks that check. Learn the intention, retain Next shared eligibility.
- The original resident benchmark kills by name/reads real settings; never run it
  here as-is. Use exact owned fixture PID/path and temporary synthetic data.
- Plugin runtime/calendar/SDK/network capabilities are feature research, not this UI
  stage or permission to add a second plugin architecture.

## Performance evidence levels
`docs/benchmarks.md` contains historical self-reported Windows release first summon
8-29ms(M1)/14ms(M2), second21ms(M2), app query87us on200entries/500average runs.
These are NOT measurements made in this review; tray-ready/visible are not
input-ready. The50ms cold-start target explicitly refers to macOS release and is
not an applicable Windows target. M3 end-to-end numbers remain unfilled.

File benchmarks report300k selective query p50~77us with3gram vs8.9ms scan,
broad candidate fallback~21ms in that section; earlier wide scan~100ms uses a
separate historical measurement.1M synthetic structures105.3MB and<=200MB private
application target are reference-specific. Do not compare with Next desktop+
PlatformHost+LocalSystem service as one equivalent process. Manual/ignored report
and loose plugin scaling tests are not strict interactive release gates.

Proposed Next evaluation: fixed dataset/build/DPI/hardware, first/warm summon to
actual first paint AND input readiness; query p50/p95/p99 including debounce,
CJK/IME/backspace/cold icons/zero/broad/selective cases; stale-result activation0;
visible-row work bounded;60s idle CPU/private/handles plateau and repeated100
show/hide sessions no monotonic retained growth. Measure initial Next baseline
before adopting absolute targets. No indexer algorithm/service change in UI slice.

## Next invariants / clarified adaptation boundary
Current Next620x44(collapsed)/410(expanded),42px search/40px rows,26px icon/48px
trailing rail,200bounded results,96icon cache,65ms debounce; passive400x410 preview,
C# native owner, Root/input focus restoration, actions/host/file-dialog integration
and deferred entity boundaries remain unchanged. User explicitly clarifies layout
and component reference ONLY: no dimension, visible-behavior or feature changes.
Assistant's proposed wide geometry option was unsolicited and is withdrawn.
Steward custom SearchInput and ResultList/div slice are not direct Kit controls;
inspect composition, independently adapt compatible layout without replacing Next
input/search/window/preview owners. Feature/performance findings are informational.

## Snapshot anchors (SHA256)
- Cargo.toml: DB2DCE4CCCCEE702F7F2CCE84EAD3F4AB74A09B470C3507CE8AAC10072654149
- crates/app/src/launcher.rs:4F45E350872D1398A1EEAF55D2E83DDE2D9CF10CFC74B590F8E8F25011D38F2B
- crates/ui-components/src/results_list.rs:C53FA0ABE9E01D41A940914C8F5B95E53464A3775DF123D68D64A14AA51A00E3

## Approved implementation boundary — 2026-10-11
User approves corrected layout/component-only scope with "理解就好，开始吧".
Rechecked current Next QuickLaunchPanel::render_result/render and row shell against
Steward results_list::render_row and launcher search composition. Next retains
uniform_list, two-line title/subtitle,40/26/48 rails,620x44/410 native budgets,
all actions/IME/query/preview ownership. Released Kit0.7.1 input/group.rs inspected:
InputGroup retains InputState and one frame; addon mouse focus respects buttons'
prevent_default. Adopt a real InputGroup with Search addon, not Steward handwritten
SearchInput. All cross-owner and native operations remain untouched. Managed
Listary ShowWindow base was rechecked; recorded SwiftList input entry path is no
longer present in supplied snapshot, not claimed reread. Existing production
interaction lifecycle regressions remain required. No algorithm/benchmark change.

## Collapsed double-outline correction — 2026-10-11
User screenshot rejects the nested rounded InputGroup plus square exterior outline.
Confirmed actual PNG edge27272a versus canvas18181b and current panel root1px border/
header bottom divider. Gallery also added a1px image outline. Keep existing client,
border layout reservation, InputState and expanded-window boundary; collapsed root
border/divider paint canvas so only Kit InputGroup's rounded input frame is visible.
Remove gallery-added outline. This is not native HWND round-corner/transparency
acceptance; no native policy change. Real PNG pixel regression before/after required.

Expanded search-outline follow-up: user rejects the same nested outline in results
and action modes. The collapsed-only correction was incomplete. All three modes
must share canvas-painted shell border/header-divider reservations; Kit InputGroup
owns the only visible search outline and focus treatment. Retain full client sizes,
same content insets, result/action rails and original keyboard/native behavior.
Check actual collapsed/results/actions PNGs, preserving the Kit input outline;
no native corner/transparent-HWND change or new borderless interaction owner.

User approves latest running result/action effect and asks input frame to extend
outward because inset to outer corners is too wide. Shared header horizontal inset
changes8px->4px in all presentations. Kit radius/default32px input and42px header,
620px clients/1px border reservation, result/action/preview/native behavior stay.
This is container composition, not control-radius or native-corner modification.
