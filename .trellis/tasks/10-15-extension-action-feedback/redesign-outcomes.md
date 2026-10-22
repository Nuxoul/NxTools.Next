# Extensions redesign — source handoff, 2026-10-15

## Current result
The authorized correction in [redesign.md](redesign.md) is implemented in source. The earlier prose-list gallery remains rejected; neither this handoff nor passing tests constitute user visual acceptance. Task is ready for source/interaction review with direct screenshot inspection blocked by the viewer tools, not a certified visual closeout.

### Visible layout
- Replaced long package rows with responsive Other Tools-style catalog cells: two columns at the supported main-window budgets, one at narrow standalone width. Each uses released Kit0.7.1 ListItem/Icon/Label/Tag/outlined Button; GPUI grid arranges them, without an invented Card/Grid component or DataTable substitution.
- Cells show name/version, compact purpose, actual trigger types, current readiness and one next step. Existing full Settings detail and all six page indices/drafts remain.
- Header import/runtime/chain/activity and detail Back actions retain actual official outlined Button boundaries. Public Toolbar.content arranges them without Toolbar.child's ghost/compact conversion. Text actions retain Medium/default32; result paging deliberately uses compact24.

### Interaction changes
- Current pending run -> **查看进度**, independently of a newer completed record.
- Script needing current trust -> **检查并信任**, opening the original warning/runtime/trust flow without enabling or saving on open/cancel.
- Disabled package -> **启用插件**, through the existing configuration owner.
- Enabled package -> **使用方法**, with complete purpose and real per-handler entry instructions, effective keywords and explicit links to existing settings/actions/automatic-trigger configuration.
- Clipboard guidance requires the user to select a history item. No guessed input, automatic execution, automatic trust, clipboard copy/paste or silently opened native panel.
- **导入扩展** opens the original ZIP chooser directly; manual ZIP-path inspection remains a collapsible fallback. Chooser failure copy now consistently refers to ZIP.
- Manual feedback puts state/outcome first and separates plugin-provided explanation from host-confirmed saved history results. Long text scrolls independently of fixed Cancel/View Result/Close actions. Up to64 results use bounded paging rather than64 buttons.
- Recent Activity presents package/action/state/elapsed/confirmed-result count and per-run controls, not a technical ID wall. It remains the original bounded session-only manual ledger, not complete event/search/download telemetry.

## Behavior and regression evidence
- Actual old-render red: `extension_catalog_renders_cells_side_by_side_not_prose_rows` failed because the old first and second actions were on distant rows (y158.5/y664.5). New catalog passed the same boundary.
- `extension_header_actions_paint_real_outlines_in_both_kit_themes`: current Button bounds,32px height and nontransparent actual border paint in Light/Dark, not merely a type/import/size assertion.
- `extension_outlined_header_retains_official_toolbar_keyboard_roving`: original search -> ShiftTab -> four header commands -> Right wrapping.
- `extension_cell_next_step_opens_trust_without_enabling_or_saving`: real click/dialog/cancel, unchanged owned configuration bytes.
- `enabled_cell_use_guide_is_explicit_and_never_dispatches_a_run`: real usage dialog/keyword instruction/close, no manual dispatch.
- Manual feedback tests cover actual per-run cancel, close without cancelling another run, terminal repaint, long explanation,64 results, six states, Light/Dark,300x440 and280x280. Ineligible result and repeat-cancel behavior are tested by actual non-dispatch, not unreliable disabled snapshots.
- Existing host-confirmed result observation, original worker exact-ID navigation, current-origin/session/generation checks, deferred cross-owner callbacks and one-Escape/one-layer regression remain intact.
- Compile/import/fixture authoring failures are not claimed as behavioral red. One redundant-closure lint was corrected; no warning suppression or dependency change.

## Final checks actually run
All Cargo commands were parent-serialized with Rust1.96, `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`.

| Check | Result | Session scratch log |
| --- | --- | --- |
| `cargo test -p nxtools-desktop -- --test-threads=1` | **754/754**,0failed/ignored,87.69s final run | `extension-redesign-desktop-final.log` |
| `cargo clippy -p nxtools-desktop --tests -- -D warnings` | PASS after final correction | `extension-redesign-strict-final.log` |
| `cargo check --workspace --tests` | PASS | `extension-redesign-workspace-final.log` |
| `cargo fmt --all -- --check` | PASS after final correction | `extension-redesign-fmt-final.log` |
| Clipboard/UI-state/preview-isolation/Desktop-shell source lifecycle gates | All four PASS | `extension-redesign-gates-final.log` |
| Component catalog documentation gate |79 families/78 locked entries PASS | same gates log |
| Spec contracts |94 documents/13 owners PASS; rerun after handoff edits | same gates log / `extension-redesign-handoff-final.log` |

The workspace check preceded the final equivalent closure/format correction; final strict/full tests compile that exact final source. Earlier Clipboard44/44 is the first-slice checkpoint, not a newly rerun Clipboard suite. Debug preview build passed before that equivalent correction; its production-only preexisting `ClipboardApplication::extension_adapter` dead-code warning remains unsuppressed.

## Capture evidence and genuine remaining blocker
- New gallery: [artifacts/extension-redesign-ui/index.html](../../../artifacts/extension-redesign-ui/index.html). Old rejected gallery is preserved separately.
- **10 actual isolated Debug GPUI PNGs generated, NOT directly viewed in this turn**: Light/Dark1060x650 catalog,920x480 catalog/detail,300x440 running/saved feedback. Synthetic configuration/packages, disconnected bridges, no script execution or real clipboard data. Exact owned PID/HWND,96DPI and native client dimensions verified. `evidence.json` records final hashes.
- Initial Light default image had only2 sampled colors and was discarded/replaced. Recapture raises/repaints only owned fixture windows and rejects fewer than20 sampled colors; final ten have47–144. This establishes nonblank capture, **not visual correctness**. Recapture log: `extension-redesign-recapture.log`.
- BrowserPreview requested the local gallery; Browser screenshot failed with **“Browser view did not become visible for the calling session. Switch to that session and retry.”** Independent REA browser scenario failed with `execution_failure`; loopback screenshot target inspection returned `provider_unavailable`. No successful image was returned for direct visual inspection. Do not claim10 images inspected, adequate aesthetics, or user acceptance.
- Usage/activity behavior has actual UI regressions but no new screenshot capture in this gallery. Installed/physical clicks/mixed-DPI/screen-reader/full product acceptance remain unperformed.

## Non-actions
No release packaging, signing, install, deployment, service restart, user-plugin execution, real user clipboard/history/credential mutation, commit/push or task archive. Existing sequence8 installer remains unchanged and contains neither this redesign nor the earlier source-only manual feedback slice. Preserve all unrelated dirty-tree work and W10 deferral.

## Later user-authorized packaging — sequence9
The preceding no-packaging/unchanged-sequence8 paragraph is the source-handoff checkpoint. User subsequently authorized packaging verification. Release sequence9 now contains this source slice and passed build/publish/sign/installer fixtures/independent artifact checks; see [release-package.md](release-package.md). This supersedes that earlier packaging status only: no installation, production launch, trusted deployment or live service mutation, and no visual/native acceptance is claimed.
