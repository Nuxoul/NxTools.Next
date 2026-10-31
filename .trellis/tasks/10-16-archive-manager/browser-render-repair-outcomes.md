# Rejected browser rendering — partial source repair, NOT visual completion

2026-10-26/27. User rejects installed TrafficMonitor ZIP browser screenshot and asks for official
GPUI components/design guidance plus shared file icons. **Overall reference UI NOT COMPLETE.**
The two original Bandizip images were read as pixels through Read, not OCR/dimensions; they show
name-list-dominant content and real folder/type icons. No current repaired native screenshot was
captured or inspected. No package/install/native GUI bypass or actual product startup here.
Current state belongs to [the checkpoint](phase-a-checkpoint.md).

## Actual work

Read online [Design Guides](https://gpui-kit.com/zh-CN/docs/design-guides/), component catalog and
Toolbar document (website0.8.1), and current locked0.8.0 public sources. No dependency/vendor change.
Verified shared native icon chain: Downloads/QuickLaunch ExtractShellIcon -> Windows adapter ->
ShellIconExtractor bounded STA -> IconCache.Shared, also used for Clipboard source icons.
windows_icon.rs is only the build ICO encoder, not file-type icon reuse.

- New private browser_icons.rs state belongs to original ArchiveBrowser. Visible regular safe
  files request existing typed ExtractShellIcon with extension-only synthetic target. No archive
  member filesystem path, materialization, execution or shortcut resolution. Existing native cache
  and PNG validation reused;8pending/128type resources/15s deadline; failure retains fallback.
- Real PNG image replaces generic File in actual row; unknown/unsafe/nonfile/noextension fallback
  retained. Existing Material folder/up icons retained, not proprietary assets. Executables get
  associated EXE type icon, NOT archive executable's embedded icon: no decode/extract to obtain it.
- Original ArchiveApplication creates view with platform bridge and routes matching result next
  turn; no synchronous entity reentry. Snapshot/generation/epoch/retirement/disconnect guards;
  validation occurs in background and pending slot retained through validation.
- Root explicitly uses project UI_FONT_FAMILY. Paint-time cell probe proves family, not source text.
- Narrow official Button.dropdown_caret/DropdownMenu companion at24x64 beside original64px action;
  original DropdownButton lacks public height-only menu slot. Two original styled Buttons,4px
  grouping gap; not a custom-painted split or private border API. Official one-action/menu/focus
  owners retained. No ButtonGroup because its toggle semantics would be wrong.
- Name column restored/dragged lower bound240, other widths retained. Human-readable binary units,
  right-aligned numeric text, directory size cells blank; raw model/sorting/unknown metadata unchanged.
  Measured header AND row lanes align at720/960/1200; initial competing-column theory not confirmed.

## Remaining / edit stop

**Reference name-list default is NOT implemented.** A persisted-view-mode draft and regression were
added but not wired: Edit repeatedly rejected browser.rs unseen anchors149/175/297/517/536 with
EDIT_LINES_UNSEEN. After3+ failures the project stop rule forbids further editing that path this
turn. No shell/Write/delegate bypass. Unwired mode draft removed; ui_state.rs restored byte-exact
SHA256 AB16D263001BD96CEE9F905EA8E05E0AF4696CCD65DF38363B93BC16C710838B; added incomplete
mode regression removed. Current layout remains detail columns. No claim of default name list,
view-mode persistence or whole screenshot fidelity. No user layout data was deleted/reset.

The supplied screenshots are references and user rejection, NOT acceptance of this repair. Actual
post-repair native pixels, physical mouse/keyboard/DPI/IME/UIA and user approval remain unperformed.
Existing self-app GUI denial unchanged. Installed/package output was NOT updated by these sources.

## Exact checks / retained failures

Evidence: current session scratch archive-render-repair-20261026-225242.
-02-red.log: header geometryPASS; split actual top74/main versus90/menu FAIL.
-05-icons-red.log: actual file rows produce no shared type command FAIL (empty channel).
 Earlier new-test compilation mistakes E0716/E0593/E0599 are retained, NOT behavioral reds.
-06-render.log:3new casesPASS after initial repair;07-browser.log:33relatedPASS.
-08-related.log:56PASS/8FAIL; new readonly icon commands broke old next-wire-is-business test
 assumptions. No actual unsafe open/drag asserted from those failures. Narrow test helper validates
 ONLY extension-only32px icon requests (max8) before business command; all other original assertions
 remain. Dedicated icon tests use raw queue and original application notification routing.
-09-related.log:64PASS/0FAIL/0ignored,1264filtered,18.31s. Browser/application/drag related collections,
 not whole Desktop. Six new tests included: typePNG/stale, pending/disconnect, font/sizes, lanes,
 action/caret geometry, original deferred result router. Same test collection after test adaptation.
-10-strict.log: Desktop --tests Clippy -Dwarnings PASS. Not a production strict run. Existing
 SPACING_12 import and animated field warnings observed in product targets during tests, unchanged.
- Scoped rustfmt passes. Spec114docs/14owners, catalog79/78, GPUI release/editor provenance gatesPASS.
 Rust commands jobs1/incremental0/codegen256, locked/offline, checked lane before each, no pauses/kills.

No full codec/password/resource/CLI matrix, no accepted UIAC/Shell behavior changes, package/install,
service/Registry/COM/UNC/source recycle/dependency/vendor/commit/push. Do not claim UI done from tests.
