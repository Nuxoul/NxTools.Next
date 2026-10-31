# Tree / archive-root native drag — implementation evidence

Scope: existing **Phase F — Explorer integration and genuine native drag**, no phase renumbering.
This file owns this slice's evidence; shared reference/checkpoint files are not edited.

## Reference and component selection (recorded before implementation)

- ABDM `D:\Code\ab-download-manager-master\desktop\app\src\main\kotlin\com\abdownloadmanager\desktop\pages\singleDownloadPage\CompletedDownloadPage.kt:106-139`: visible gesture source, empty click; `pages\home\DownloadItemListDataFlavor.kt:14-39`: completed real-file transferable. Not virtual extraction or permission to drag on ordinary down.
- Legacy `D:\Code\NxTools\Views\DownloadCompletedWindow.axaml.cs:78-97,119-143` pressed-left guard -> `Services\WindowsFileDragService.cs:29-61` -> initialized OLE/one native loop. Historical Avalonia fallback is not adopted.
- Current `csharp/NxTools.PlatformHost/Windows/Archive/ArchiveVirtualDragSession.cs:38-110`: admission before dedicated STA, actual HWND/peer validation, original NativeFileDragService attach/cancelcapture then high-bit button check, final cancel guard and one DoDragDrop. `Protocol/PlatformPipeClient.cs:339` synchronously admits original session then detaches receipt waiting, not serial-reader OLE.
- Official released Component0.8.0 `vendor/gpui-component-080/src/tree.rs:42-44,91-122`: `Tree::new(&Entity<TreeState>, Fn(usize,&TreeEntry,bool,&mut Window,&mut App)->ListItem)`, original styled ListItem slot. Base0.8.0 registry `src/tree.rs:441-483`: selection/expansion only on full click; dragged-away press leaves Tree alone. Public depth/is_root/is_folder/is_expanded and ListItem child/MouseDown/Click suffice; no vendor modification.
- Requested shape remains existing left hierarchical Tree + right default details/selection + resizable preview. Official selected/secondary ListItem32DIP, existing24DIP icons,16DIP disclosure/depth,160..280DIP Tree pane and current splitter/scroll ownership remain. Only icon/label child down arms the same gesture; arrow retains official full-click expansion and never arms drag. No custom ordinary control, Toolbar variant/size change, GPUI on_drag or new gesture framework.

## Invariants / chosen semantics

Tree target is a metadata-only identity in the original candidate, not list selection or a second controller. Down alone does nothing; >=4DIP still-left-held move freezes original snapshot/source/generation/ordinal and immediately submits original bridge with actual HWND, on original read connection under existing locked admission. Navigation/selection/focus stay with existing owners; crossing threshold does not navigate Tree or rewrite right-list selection. Cross-entity Tree updates remain deferred. Sort/filter/refresh/source/close/disconnect/authorization invalidate original candidate and exact pending native run.

Subdirectory preserves its own basename and complete descendants including implicit/explicit/empty directories, regardless of current list folder/filter. Archive root wraps the complete archive in exactly one validated display-name stem directory; original entry ordinals and actual worker/cache paths do not gain this presentation prefix. Compound suffixes are one case-insensitive unit; ordinary suffix strips the last extension. Invalid original display name or resulting stem rejects (no sanitization/collision rename). Empty archive root refuses with explicit feedback, no empty placeholder drag. Root over budget refuses whole item, never truncates. Bounds stay32roots/128descriptors/32files/512MiB/256KiB plus native256 content requests.

No new wire field/version is needed: presentation is already `item.name`, content identity already `item.entry.ordinal/browser/source`; legacy v1/defaultfalse-directory/missing-or-null-size remain unchanged. New name checks must agree in Rust and C# without weakening original bounds.

## Verification status

Implemented source; compiled/native acceptance is deferred to the main session. Cargo and dotnet
builds/runs were explicitly prohibited in this subagent and were not started.

### Delivered owners

- `rust/apps/nxtools-desktop/src/archive/browser.rs:2548-2658`: original official Tree icon and
  label children arm the original candidate. Original ListItem click bubbles to Base Tree unless
  threshold was crossed. Root `capture_any_mouse_down` resets stale suppression before a new
  gesture, including disclosure/split/scroll/right-click. It does not select/navigate/cancel an
  already admitted run. Released pre0.3.9 `elements/div.rs:902-911` provides this public capture API.
- `browser_drag.rs:11-18,78-138,183-270`: Selection/Tree identity in the same candidate, same held
  threshold/HWND/receipt/pending/connection admission and same password authorization. Tree freeze
  leaves the right model selection/focus/folder untouched. No application or second entity writes
  from the child down callback; only local candidate state is changed, no notification/writeback.
- `model_drag.rs:17-37,75-225`: one common `freeze_drag_roots` expands original entries for right
  selection and Tree. No snapshot clone, IO or preextract. Root inserts one ordinal-less directory
  after complete source-root/name/descriptor/file/known-byte validation. Wrapper counts against128;
  source top-level roots remain<=32 even though the published wrapper is one root.
- Root naming: remove `.tar.gz`, `.tar.bz2`, `.tar.xz`, `.tar.zst`, `.tar.zstd` as one case-insensitive
  unit; all other extensions remove only the last suffix (`a.v1.zip` -> `a.v1`, `a.tgz` -> `a`,
  `a.zip.001` -> `a.zip`); extensionless names remain unchanged. Validate both original display
  component and resulting stem. No replacing/trimming invalid characters, truncated names or
  hidden fallback directory. Empty archive refuses explicitly, explicit empty subdirectory works.
- Protocol `archive_virtual_drag.rs:17-39`: narrow descriptor component policy adds console names,
  superscript device aliases and spaces before device extensions, matching native output policy.
  C# `Protocol/ArchiveVirtualDragContract.cs:33` additionally matches existing Rust UTF8<=255 bytes
  and all control-character rejection. Version1/nullable size/defaultfalse directory/bounds unchanged.
- IPC `archive_virtual_drag.rs`: test only; original same-lock origin comparison/admission unchanged.
- C# `ArchiveVirtualTreeTests.cs:81-93,175-195`: existing registered tests now cover root DTO, native
  HGLOBAL wrapper paths and lazy original worker content ordinal. Actual cache output remains
  `files/root/known.txt`, never `Bundle/root/known.txt`; original owner/lease cleanup is asserted.
  Session/Content/Cache production code required no change and was deliberately left untouched.

### Exact regression names (authored, NOT compiled/executed here)

Desktop `archive::browser::drag_tests::` (seven new actual production Tree/event cases):
1. `production_tree_drag_icon_and_label_preserve_directory_without_list_navigation`
2. `production_tree_root_drag_adds_safe_archive_stem_and_preserves_original_ordinals`
3. `production_tree_drag_disclosure_click_release_and_unheld_move_never_start`
4. `production_tree_root_drag_empty_unsafe_and_over_budget_reject_whole`
5. `production_tree_drag_reconnect_and_projection_retire_original_candidate`
6. `production_tree_drag_empty_directory_and_authorized_root_use_original_session`
7. `production_tree_drag_split_scroll_and_empty_body_never_become_tree_sources`

Model `archive::model::drag::tree_tests::` (three new cases):
- `tree_freeze_is_independent_of_current_projection_and_preserves_basename`
- `tree_root_stem_compounds_and_unsafe_names_fail_closed`
- `tree_root_wrapper_counts_toward_existing_descriptor_budget_and_unknown_encryption_stays_denied`

Protocol: `archive_virtual_drag::tests::tree_root_wrapper_v1_preserves_authority_and_rejects_windows_device_aliases`.
IPC: `archive_virtual_drag::tests::tree_root_descriptor_uses_same_locked_origin_and_never_leaks_rejected_admission`.
Existing overlap/multi/unknown/unsafe/encrypted and row-drag tests remain unchanged.

### RED evidence and executed validation

- The first five actual Tree tests plus two model tests and native root-path assertions were
  authored before production changes. **Real GPUI/worker RED was NOT executed** under the no-build
  delegation; do not turn expected failure into observed RED. This prerequisite remains an evidence
  gap for main-session acceptance, not a claim physical/native interaction passed.
- Actual observed static RED: `python tests/architecture/archive-tree-root-drag.py`, exit1/3FAIL:
  Tree child arm count0!=2; missing original freezer and native-agreeing name check. Original output:
  `$env:PI_SCRATCH_DIR\tree-root-static-red.log`. These are source assertions, not behavioral RED.
- Final static gate:3PASS. Existing archive-phase-f-spike PASS; GPUI release, pinned Markdown editor,
  component catalog and spec-contracts(114docs/14owners) PASS. Scoped rustfmt/check six Rust files and
  git diff --check PASS. No Cargo/dotnet/test harness/worker/native OLE was run here.
- Browser edits are confined to Tree composition and original root drag capture reset. Current
  detailed columns/icons/marquee/preview were not replaced; formatting touched only scoped owned
  Rust files (`skip_children=true`), not neighboring modules.

### C# effects review and exact LIST -> RUN handoff (main session only)

Existing Program registrations at570-589 already classify these as Fixture. Added assertions use
only synthetic DTO/native HGLOBAL plus one tiny GUID-owned ZIP, original approved worker Job/cache
and controlled injected IDataObject consumer. No live HWND, physical input, actual DoDragDrop target,
Explorer, Registry, services or product initialization. The ZIP case requires an explicit matching
`NXTOOLS_ARCHIVE_WORKER_FIXTURE`; absent worker is typed SKIP, not acceptance. No new Program or
TestEffects registration/change is needed. After the main session's serial C# build:

```powershell
$treeDragFilters = @(
  'archive Phase F virtual tree wire shapes conflicts bounds',
  'archive Phase F virtual tree SDK descriptors directory refusal',
  'archive Phase F virtual tree actual ZIP unknown streams identity',
  'archive Phase F virtual STA gesture cancellation responsiveness',
  'archive Phase F virtual actual worker delayed indexed contents'
)
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures --list $treeDragFilters
# Review exact LIST5/effects/scopes before invoking the same union:
dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-build --no-restore -- --fixtures $treeDragFilters
```

Do not run the full fixture suite. Preserve LIST=RUN=PASS5,SKIP0/FAIL0/not_run0 for this precise union;
if an environment is missing report its SKIP honestly. Suggested serial Rust checks for main:
`cargo test -p nxtools-desktop --bin nxtools-desktop --locked --offline archive:: -- --test-threads=1`,
`cargo test -p nxtools-protocol -p nxtools-ipc --lib --locked --offline`, then scoped strict lint/check
as the main lane permits (jobs1/incremental0/codegen256). Do not duplicate a running build.

### Remaining acceptance / restrictions

Main-session compiled tests and C# targeted LIST/RUN are still required; no source integration in
application.rs is needed. Real Tree mouse gesture/native HWND/capture/OLE/Explorer, DPI/focus/IME/UIA,
product visual/user acceptance remain NOT RUN. No build/package/install/productlaunch/Explorer/
Registry/service/sign/commit/push or recursive delegation occurred. Shared reference/checkpoint and
all prohibited application/forms/progress/draft/jobs/ui/titlebar/Core/window files remain untouched.
New reference evidence above is supplied for the main session to merge, not silently duplicated there.

## Main-session compiled integration evidence

Original archive:: production test selection passed205/205/0ignored (archive-targeted-green9.log), including all7newTreegestures and3newmodel tests plus retainedrightrow/marquee/preview/password/cancel boundaries. Core93/IPC43/Protocol83 pass; protocol/IPCnewroot tests included. Fresh `cargo build -p nxtools-archive --bin nxtools-archive-worker --features isolated-codecs --locked --offline` passed with jobs1/incremental0/codegen256. C#testhostbuild0warnings/0errors; exact five-filter LIST5 then same RUN5/PASS5/SKIP0/FAIL0/not_run0 with NXTOOLS_ARCHIVE_WORKER_FIXTURE=the freshtarget/debugworker. Logs archive-csharp-tree-five12.log/archive-worker-build12.log in main scratch. This closes deferred compiled/controlled fixture evidence, NOT actualTree physicalpointer/foreignOLE/Explorer/nativeDPI/useracceptance. Earlier static-only RED limitation remains historically unchanged.
