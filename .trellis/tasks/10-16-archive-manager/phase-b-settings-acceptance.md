# Phase B — Compression module and settings: independent acceptance

2026-10-21. **Scoped source/configuration/navigation checks PASS; full Phase B acceptance NOT granted.**
The sole current status owner remains [checkpoint](phase-a-checkpoint.md). This review does not
accept Phase A or authorize Phase C/D/F, deployment or a dependency waiver.

## Scope and source findings

Reviewed the supplied implementation handoff, [contract](phase-b-settings-contract.md), approved
phase dependency amendment, Core archive types/tests and locked read/merge/save boundary, production
MainWindow initialization/navigation, ArchiveSettingsView callbacks/rendering and production-page
tests. Official component selection/version documentation and released dropdown source were checked.

- Formal production MainWindow constructs and retains ArchiveSettingsView from the real config/path
  owner; Archive is between Extensions and Tools, with six separate navigation focus handles.
- The page uses locked Kit0.7.1 Settings/SettingPage/SettingGroup/SettingItem/SettingField, official
  outlined dropdowns, Switch, stable InputState, outlined Save and persistent Alert. Six pages and
  built-in search/scroll are present; no substitute Card/Grid or parallel page/scroll controller.
- Saveable presets match the documented defaults. Unsupported browser/jobs/Shell actions are explicitly
  unavailable, not fake Apply/Saved controls. No fixed-directory setting exists here, so no new picker
  or Protocol/C# change is required for this slice.
- Archive defaults apply only when the whole field is absent. Present malformed/null/incomplete data
  fails. Validation is shared by load/save/update. The existing inter-process lock/reread/section merge/
  sync/rename path preserves unrelated owners and refuses missing/corrupt update targets.
- Cross-entity setters defer to the next application turn via the weak page owner. Failed saves retain
  the previous live value and exclusion draft; render/search do not save. Exclusion JSON preserves
  meaningful spaces/semicolons/escaped newlines. Gzip/XZ exclude Store; TAR disables level selection.
- Save diagnostics use one correlation UUID and fixed outcome/reason/elapsed fields, not config payload.

No new concrete in-scope implementation defect was found in this bounded review. This is not a claim
that later browser/forms consume these presets, or that source inspection establishes visual usability.

## Independently executed checks

All Cargo commands were offline, serial, with CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0 and
RUSTFLAGS=-C codegen-units=256. No competing Cargo/rustc process was observed before execution.
The first background wrapper was reclaimed before producing a test result; it is NOT counted.
The full verification script then completed in one foreground invocation.

| Command/check | Actual independent result |
| --- | --- |
| `cargo test -p nxtools-core --lib --offline` | 69 passed, 0 failed/ignored/filtered; includes 9 archive configuration tests |
| `cargo test -p nxtools-desktop archive_settings_ --offline -- --test-threads=1` | 4 passed, 0 failed/ignored, 867 filtered |
| `cargo test -p nxtools-desktop kit_foundation_navigation --offline -- --test-threads=1` | 2 passed, 0 failed/ignored, 869 filtered |
| Exact `main_window::tests::foundation_sidebar_keyboard_activates_the_same_module_as_pointer` | 1 passed, 0 failed/ignored, 870 filtered |
| `cargo clippy -p nxtools-core -p nxtools-desktop --tests --offline -- -D warnings` | exit0 |
| Same strict Clippy with `--bins` instead of `--tests` | **exit101**, three errors below |
| `cargo fmt --all -- --check` | exit0 |
| spec-contracts | exit0; 106 documents / 14 owners; document structure only |
| gpui-component-catalog | exit0; 79 pages / 79 rows / 78 locked entries; documentation only |
| task validate | exit0; 82 implement / 79 check entries; two existing oversized-document warnings |
| desktop-shell-lifecycle / ui-state-lifecycle / ui-token-boundaries / ui-consistency / ui-preview-isolation | all exit0; source gates only |
| `git diff --check` | exit0; existing line-ending warnings, not whitespace errors |

The four actual production-page tests were:
1. `archive_settings_exclusion_editor_is_lossless_and_stream_levels_are_valid`
2. `archive_settings_navigation_save_failure_and_return_use_production_owners`
3. `archive_settings_official_search_never_saves`
4. `archive_settings_real_keyboard_scroll_and_themes_keep_controls_reachable`

They mount production MainWindow/page in GPUI TestWindow and exercise synthetic input, on-disk
persistence/failure, retained owners, search/no-write and default/minimum × Light/Dark control bounds.
They are **not physical input, native window screenshots or user visual acceptance**.

Implementation's full Desktop871, Protocol54, C#2, workspace check and Debug build are historical
handoff results, NOT independently rerun or added to this review's pass count. No codec worker,
100k/large/archive-performance matrix, native dialog or system mutation was run.

## Remaining acceptance gaps

### 1. Real production-page window usability/visual evidence — NOT RUN

Read-only Computer Use listed 15 windows and no archive-settings preview. The handoff's isolated
launcher was already rejected by the self-app security rule. This review did not bypass it with
another launcher, shell route or tool. No screenshot was captured, no actual native mouse/keyboard/
scroll action delivered, and no physical DPI/provider acceptance established.

Required live default1060×650/minimum920×480 × Light/Dark checks remain pending. A permitted isolated
window supplied through an approved workflow, or user-performed checks, is needed to close this gap.
Imports, compilation and TestWindow tests do not close it. Formal installed-app startup is not
implicitly authorized by this review.

### 2. Production-target strict Clippy — FAIL, outside this settings implementation

The independent `--bins -D warnings` command reproduced the handoff's exact three diagnostics:
- `clipboard/extension_adapter.rs:275`: unused `extension_adapter`;
- `clipboard_worker.rs:431,453`: unused `query_category_page_async` and
  `query_category_page_async_with_cursor` (one diagnostic);
- `ui/components/project_menu.rs:153`: `clippy::needless_return`.

These files were not modified to force a green result. Passing `--tests` is not a production-target
lint pass. No new broad allow, unrelated refactor or repair is requested by this report.

Existing context warnings remain: native-contracts35497 and archive-reference33819 bytes exceed the
32768-byte injection limit. They do not invalidate the executed document gate or establish UI PASS.

## Evidence, preservation and stop

Raw script and independent logs/results are under:
`C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\`
- `phase-b-independent-verify.ps1`
- `phase-b-independent\results.json`, per-command `.log`, `baseline.json`, `preservation.json`

The verification-end hash comparison found **157/157 baseline task/source/manifest/asset files
unchanged**. Subsequent workspace edits are only this additive report and the current checkpoint's
status paragraph. Historical reports/results and implementation remain untouched.

**Disposition:** accept the bounded configuration/navigation/source-test slice; leave full Phase B
open for live UI checks and the disclosed production lint gate. Phase A remains incomplete under the
approved amendment. No packaging/install/signing/formal startup/registry/service changes, source
recycle/delete, commit/push or Phase C/D/F implementation occurred.
