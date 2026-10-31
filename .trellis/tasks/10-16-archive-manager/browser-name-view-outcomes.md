# Reference name-list continuation — implementation and checks

2026-10-27. User requested continuation of the unfinished browser UI task. Default name-list /
details switching is now implemented, not merely planned. [Current checkpoint](phase-a-checkpoint.md)
owns current status; [prior partial repair](browser-render-repair-outcomes.md) remains historical,
unchanged. **No overall visual/user/native acceptance or package delivery is claimed here.**

## Implemented

- Original ArchiveBrowser starts in name-list mode. Old UiState files with no details_view field
  also select name-list; additive serde defaultfalse, same version and original writer/file.
  Explicit true/false persists. Null is rejected rather than silently converted.
- Name header and file-name rows use the same mode-aware width as content/hit region. The actual
  right viewport is measured through public ElementExt::on_prepaint and updated next turn only
  when changed. No guessed window-minus-sidebar width, render IO, polling or new selection owner.
- Name header is left aligned with the file text lane and remains a real official sorting Button.
  Default name mode hides metadata columns without destroying their saved widths/visibility.
- Original View menu has checked Name list / Details entries (名称列表 / 详细信息). Actual menu
  pointer tests exercise the original deferred callback. Details restores original resize tracks;
  obsolete resize callbacks cannot mutate name mode. Selecting a detail column reveals Details.
- Switching retains immutable row selection, original sorting/focus/controller and detail column
  preferences; invalidates the current drag candidate through the original owner and resets only
  obsolete horizontal offset next turn. No read/materialize/open/Job/native authority is added.
- Original official ListItem/uniform_list32px, Tree/resizable, menus, UI font and shared Shell type
  icons retained. No vendor/dependency/palette change or private API. Mode changes emit low-rate
  correlated start/terminal events without paths/payloads; not a persistence-success claim.

## Tests and retained failures

Evidence root: current session scratch `archive-name-view-20261027-002544`.
Every Cargo used locked/offline, jobs1/incremental0/codegen256 and an idle-lane check per command.

-01-red.log: real production default test FAIL because metadata headers still appeared.
-02-default.log: same default test PASS after implementation.
-03-related.log:67PASS/0FAIL. Original browser/application/drag collections, not full Desktop.
-04-final-related.log:66PASS/1FAIL after adding a splitter gesture check. Single-move synthetic
 gesture without settling draw did not resize; same press/segmented move/draw/release sequence as
 original splitter regression then passed in06-split.log. No production splitter change was needed.
-07-final-related.log:67PASS/0FAIL, then08-final-strict.log found a new test field_reassign_with_default.
 Fixed the initializer without allow/suppression. Intermediate syntax/initializer issues were not
 counted as behavioral reds; logs remain.
-09-verified-related.log: **67PASS/0FAIL/0ignored**,1264filtered,17.78s,exit0 on final source.
-10-verified-strict.log: **Desktop --tests Clippy -Dwarnings PASS**,exit0. No separate production
 strict run or whole-workspace strict claim.

Three new tests cover: actual default name pane at720/960/1200 plus splitter press-move-release;
actual menu switching/long filenames/selection/columns/width retention/horizontal reset and
Light/Dark minimum; legacy missing/null and both explicit modes saved/reopened in original store.
Existing details-focused tests explicitly select Details rather than assuming product default;
new default test uses the actual production default. All original business/security assertions stay.
Scoped rustfmt/diff and spec114docs/14owners/catalog79/78/release/editor gates passed.
Test artifact at measured completion:93DD447ECD4F8A70E2B3DFD12D038D44BA293419EC3A673928DD0DC51D97BFF8.
Shared targets may subsequently change; raw names/log interval and source hashes are retained.

## Remaining limits / stop

This completes the previously missing source-level name-list/details slice. It does NOT certify
all reference-image geometry, current real HWND rendering, physical mouse/keyboard/trackpad,
DPI/IME/UIA or user visual acceptance. Existing self-app GUI refusal remains; no bypass attempted.
No product launch, install/package, service/Registry/COM/Explorer/UNC/source recycle or commit/push.
Current installer remains unchanged and does not include this source continuation.
Original checkpoint remains exact byte suffix; prior outcomes/acceptance reports are retained.
