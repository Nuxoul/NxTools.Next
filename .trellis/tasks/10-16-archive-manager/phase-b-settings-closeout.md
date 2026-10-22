# Phase B — Compression module and settings: remaining closeout

2026-10-21. **Production lint gap CLOSED; full Phase B remains OPEN for real-window evidence
and user visual acceptance. Phase A remains NOT passed.** Only [checkpoint](phase-a-checkpoint.md)
owns current state. This is an additive result record; prior [handoff](phase-b-settings-outcomes.md)
and [independent acceptance](phase-b-settings-acceptance.md) retain their historical facts unchanged.
No Phase C/D/E/F work is authorized or implemented by this closeout.

## Minimal source change / verified intent

Only three production source files changed from this round's opening baseline:

| File | Call/cfg finding | Minimal correction |
| --- | --- | --- |
| `clipboard/extension_adapter.rs` | `ClipboardApplication::extension_adapter()` has one caller, inside the existing replacement-copy test. Production `extensions/service.rs` directly constructs `ClipboardExtensionAdapter::new` from the worker. | Add `#[cfg(test)]` to the convenience getter, retaining its test and all production adapter/copy capability. |
| `clipboard_worker.rs` | `query_category_page_async` is called by two existing async/supersession tests; its private cursor helper has no other caller. Production `ui/mod.rs` uses `query_summary_page_async`, whose worker/cancellation behavior is unchanged. | Add `#[cfg(test)]` to the two full-item test query helpers and their exclusive `ClipboardHistoryPage` import; rewrap that import list only. |
| `ui/components/project_menu.rs` | Native open is called once after the same deferred trigger-lifetime guard. Only tests contain the legacy in-window fallback; therefore the old success `return` is redundant in production but significant in tests. | Keep the exact native call/arguments; bind its bool, discard it in production, and guard the unchanged test fallback with `!native_opened`. No production fallback or second dispatcher added. |

No method body/capability was deleted, no broad allow was added, no business behavior/layout/theme/
configuration/native lifecycle or dependency changed. Existing test helpers still compile and run.
The earlier native-open-success path still skips test fallback; native-open-false still permits only
the existing test fallback. Root/entity defer, source lifetime and focus owners remain unchanged.

## 1. Code checks — executed

All Cargo commands were foreground/serial/offline, with machine cargo/rustc absence checked before
each command and CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0, RUSTFLAGS=-C codegen-units=256.

| Command | Actual result |
| --- | --- |
| Before fix: `cargo clippy -p nxtools-core -p nxtools-desktop --bins --offline -- -D warnings` | exit101, exact three previously accepted diagnostics reproduced |
| After fix: same production `--bins` strict Clippy | **exit0**, no warnings/errors |
| `cargo clippy -p nxtools-core -p nxtools-desktop --tests --offline -- -D warnings` | **exit0** |
| `cargo fmt --all -- --check` | **exit0** |

One intermediate production run identified the test-only return-type import after cfg narrowing;
it was given the same test cfg, not suppressed. One intermediate fmt run requested import rewrapping;
that exact formatting was corrected. Final commands above ran together successfully. Original three
lint diagnostics are closed, not excused by test-target success.

## 2. Automated tests — executed (not real-window evidence)

| Command | Result |
| --- | --- |
| `cargo test -p nxtools-desktop clipboard_worker:: --offline -- --test-threads=1` |17pass,0fail/ignored,854filtered |
| `cargo test -p nxtools-desktop clipboard::extension_adapter:: --offline -- --test-threads=1` |2pass,0fail/ignored,869filtered |
| `cargo test -p nxtools-desktop ui::components::project_menu:: --offline -- --test-threads=1` |8pass,0fail/ignored,863filtered |
| `cargo test -p nxtools-desktop --offline -- --test-threads=1` |**871pass,0fail/ignored/filtered**, exit0; one full run in this round |

The focused groups overlap the full suite; they are not added to871. Existing worker cancellation/
async replies, deferred adapter copy and menu pointer/keyboard/focus/fallback tests are retained.
No new test matrix was added. Core69 independent acceptance remains prior evidence, not a new run.

Source-only gates actually rerun: desktop-shell-lifecycle, clipboard-lifecycle,
clipboard-ui-lifecycle, ui-state-lifecycle and ui-preview-isolation: **5/5 exit0**.
These do not certify real paint/input. No C#/codec worker/native dialog/service test was required
or run; no100k/large-file/all-codec/performance matrix, package/install/sign/commit/push.

## 3. Real production window evidence — NOT RUN

The prior Computer Use self-app safety denial is still binding. This round made **zero GUI tool
calls, zero preview launches, zero screenshots and zero native pointer/keyboard/scroll actions**.
No PowerShell launcher, alternate executable, other tool or renamed route was used to bypass it.
No newly approved automation route is available. Automated GUI verification stops here.

| Required native page | Screenshot / operations / visual result |
| --- | --- |
|1060x650 Light |**NOT RUN** |
|1060x650 Dark |**NOT RUN** |
|920x480 Light |**NOT RUN** |
|920x480 Dark |**NOT RUN** |

TestWindow theme/size/input regressions remain compiled synthetic evidence only. Missing native
inspection is not an unsupported Kit capability and cannot be converted to PASS by compilation.

### Short user-performed acceptance checklist

Only if your approved local debugging workflow can open the existing isolated Debug entry, use the
same production MainWindow/ArchiveSettingsView; otherwise keep this entire section NOT RUN. Do not
use the previously denied launcher or start the formal installed product/PlatformHost/indexer.
Existing entry is `--ui-preview foundation`; choose `NXTOOLS_PREVIEW_ARCHIVE_SETTINGS=1`,
`NXTOOLS_PREVIEW_THEME=light` or `dark`, and absent/present `NXTOOLS_PREVIEW_ARCHIVE_MINIMUM` for
default/minimum. `PI_SCRATCH_DIR` must be the session scratch root. These are existing debug options,
not an automation bypass or a newly executed command.

Repeat the following on each of the four rows above:
1. Confirm client1060x650 or920x480, selected Light/Dark theme and console isolation marker
   `runtime=false capture=false indexer=false platform=false`. Retain the printed scratch config path.
2. Check navigation order 下载/速启/剪贴板/扩展/压缩/其他工具 and all six settings pages
   常规/解压/压缩/查看/集成/高级. Capture a screenshot; note any clipped/overlapping/unreachable content.
3. Search 压缩后测试 and a nonexistent term, then clear. On 压缩, scroll to the final Save button;
   open a dropdown and cancel with Escape, then change a valid value. Toggle one Switch. Inspect the
   scratch config to confirm only the selected archive field changed; search/scroll do not save.
4. Edit exclusion JSON to `["*.tmp"," keep-spaces.txt ","semi;colon"]`, save and inspect exact values.
   Use Tab/Shift+Tab and Space/Enter to activate eligible controls once; verify visible keyboard focus.
   Leave for 其他工具 and return: saved values and exclusion draft remain; Settings may return to 常规.
5. For failure evidence, back up **only the printed scratch config**. Leave a different exclusion
   draft unsubmitted; make that scratch config invalid JSON in your editor, then toggle a setting.
   Capture the visible failure Alert: no fake Saved, prior value and exclusion draft retained, invalid
   file not replaced. Restore the scratch backup; do not touch a real user config or system ACL.
6. Save screenshots/short action notes with theme, client size, DPI, expected/observed result and config
   readback; report any defect rather than accepting it. Close the isolated window normally.

## 4. User visual acceptance — NOT RUN

No user observations/screenshots were supplied in this round. Neither this record nor the automated
test counts grant aesthetic/usability/DPI/provider approval. Close individual rows only after the
above evidence is supplied/reviewed; full Phase B stays open while any required row is missing.

## Preservation / evidence / stop

Before these records,140/140 existing task files matched opening SHA256 hashes. Only the current
checkpoint status paragraph is updated; historical reports/results remain byte-identical.
Opening copies of the three source files and exact incremental diff are retained in session scratch:
`phase-b-closeout-baseline/`; commands/exits/timing in `phase-b-closeout-results.json` and
`phase-b-closeout-check.ps1`; diagnostics and per-step logs use `phase-b-closeout-*.log`.

Final documentation/diff/preservation checks are recorded in
`phase-b-closeout-document-gates.json` and `phase-b-closeout-preservation.json` in the same scratch.
Actual final results: spec-contracts/component-catalog/task-validate all exit0; task validation
retains only the two existing context-injection size warnings. Exact source-delta assertions and
scoped diff/added-line whitespace checks pass;139/139 historical task files (excluding the approved
current checkpoint edit) retain opening SHA256. No cargo/rustc process remained at the final check.
No automated GUI restart will follow. Stop with lint closed and real-window/user evidence open.
Outstanding Phase A risk/budget/readmission, trusted UNC and strict source recycling remain unchanged;
no safety downgrade or stage acceptance is inferred.
