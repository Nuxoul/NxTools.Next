# Official UI composition follow-up — source outcome (2026-10-18)

## Delivered

User authorized the confirmed UI issues and low-risk official component adoption. Product remains pinned to GPUI Kit0.7.1, Default Light/Dark and existing fonts. All business models, key bindings, save/trust/cancel policies and native owners remain.

- **Trust recovery buttons:** original Dialog and deferred retry/browse callbacks retained. Outlined Medium32 Buttons use `Toolbar.content`, bypassing `Toolbar.child -> ghost().compact()`. Actual Light/Dark normal-state positive-border paint is asserted at Button bounds; existing PATH/explicit-path retry, Cancel0writes, trust-without-enable and config identity checks remain.
- **Empty session activity:** official Empty/Header/Title/Description inside original Dialog scroll, retaining the explanation that enabling does not execute manual actions. Test clicks the actual original activity Button, checks no invocation, and closes through Escape.
- **Shortcut hints:** official Kbd+Label render-only helper in Quick Find General, file-dialog default-directory Ctrl+G and Clipboard text editor. No new key/focus owner. Clipboard footer naturally has two rows (52px at300x440), full hints above original Save/Cancel; textarea returns40px of budget without window growth. Actual240/300px containment, original IME/temp-worker save/Cancel/Escape/focus tests remain. Only the default-directory row borrows4px from its elastic path lane for the52px Ctrl+G Kbd; ordinary result48px rails stay unchanged.
- **Extension technical metadata:** DescriptionList/GroupBox/ListItem project original permission and handler fields. Source/hash/runtime/trust/warnings remain; last handler is reachable through the original Settings scroll at744x430. No trust, enable or execution effect.
- **Download details:** DescriptionList public slots replace remaining manual key-value rows, retaining long Header/URL wrapping, semantic colors, values and masking. Marker(Start/Border)+two MarkerContent slots project original timestamps/messages/order, using the original scroll with no new timer/logging.
- **Probe waiting:** official XSmall Spinner only for actual probing AND motion_allowed; reduced/unknown/idle/handoff-only states keep static feedback. Compact176, footer/error/retry boundaries retained.
- **FFmpeg:** official XSmall4px Progress uses the supplied real percentage unchanged, only during Downloading with positive known total and percent<=100. No fabricated bar for unknown totals or later phase milestones; original status/bytes/Cancel/footer retained.

Not implemented in this bounded slice: blanket Tag restyling, InputGroup conversions, Clipboard Dialog controller migration, DataTable/Table/Calendar/NumberInput/Select/Command/complex result replacement. They remain separate compatibility candidates, not silently marked fixed.

## Regressions and corrections

- Genuine pre-fix red: `ui-followup-trust-outline-red.log` failed on the real trust recheck Button missing a visible normal-state outline in Light. Same flow passes after public Toolbar.content composition, including Light/Dark paint and original side effects.
- Actual Ctrl+G measurement caught52px Kbd overflowing48px rail by4px (`ui-followup-focused-final.log`). Parent rail adapted to52px, glyph unchanged; actual288px viewport and original row-height/nonoverlap tests pass.
- Download implementer added4 red-capable tests before product changes but ran no Cargo; no measured worker red claimed. Parent corrected test-controlled causes: direct DescriptionList reference must use identical value typography; real wheel+draw is required for ScrollHandle geometry; prior one-shot frame callbacks must drain before asserting static idle frames. Original containment/state assertions remain.
- First full run805pass/1fail was the former28px Clipboard footer assertion after intentional two-line Kbd composition. Updated to measured52px and real key/button containment, preserving focus/Escape/native client checks. Final full runs pass806,0ignored.
- Local compile/lint mistakes fixed without warning suppression: WindowExt import, unused metric import, redundant let-return and test-only typography imports. Existing `ClipboardApplication::extension_adapter` dead-code warning remains in non-test build.

## Current dependency snapshot verification

Every direct Cargo command used Rust1.96/jobs1/incremental0/codegen-units256. Final commands were `--locked --offline`; this task never wrote dependency manifests or Cargo.lock.

A final hash check detected concurrent lock mutation from `1050DA2D311C78A1372B3D10DE75AEFD56AE453B116E3B94880B32892EDD1440` to `26D81DB2936DEABA8362B4B028D9BC9F902E93BD8262F0E919C2FB6B736346C9` (mtime2026-10-18T04:06:10.7242215Z). Instead of restoring/reconstructing the lock or certifying stale inputs, a bounded revalidation used the new stable snapshot. Root manifest, Desktop manifest and lock hashes were checked before and after each command and stayed unchanged. Evidence: `ui-followup-current-inputs-before.json`, `ui-followup-current-desktop-inputs-verified.json`.

| Check | Actual result |
| --- | --- |
| Current-lock full Desktop | **806passed,0failed/ignored**,88.42s; `ui-followup-current-lock-desktop.log` |
| Current-lock strict Desktop tests Clippy `-D warnings` | PASS; `ui-followup-current-lock-strict.log` |
| Current-lock Debug Desktop build | PASS; `ui-followup-current-lock-debug.log`; existing extension_adapter warning only |
| Scoped changed UI rustfmt, skip_children=true | PASS; no archive formatting writes |
| Nine source gates | Clipboard UI, QuickLaunch, Desktop shell, Download UI, motion, UI state, token, consistency, preview isolation PASS; `ui-followup-handoff-gate-*.log` |
| Component catalog |79pages/78locked mappings PASS, local documentation only |
| Full workspace `--tests` | **FAIL outside this UI slice:** `nxtools-archive/src/worker.rs:44` contained intermediate edit directive `CUT 33.=33`, causing parse errors and downstream Serialize/Deserialize errors; `ui-followup-current-lock-workspace.log`. Not modified by this task. |
| Full read-only `cargo fmt --all -- --check` | **FAIL outside UI slice:** in-progress archive files including `tests/zip_creation_worker_contract.rs`; `ui-followup-current-lock-fmt.log`. Not formatted by this task. |

The previous dependency snapshot also passed full Desktop806, strict, workspace, full read-only fmt and Debug build (`ui-followup-handoff-*.log`). Those earlier workspace/fmt greens are historical only, not current whole-workspace acceptance. The external in-progress archive failures are not treated as UI defects and were not repaired here.

## References and ownership

Mandatory component rules/table/index and exact released Toolbar/Button/Kbd/Empty/DescriptionList/Marker/Spinner/Progress public sources inspected. Original Clipboard constructor/Open/Activated/Hide/KeyDown and single passive-preview lifecycle preserved; Avalonia native base source unavailable. Downloads inspected ABDM before legacy NxTools, following metadata/probe/progress and keyed native Window/close boundaries. No native owners or engine operations changed. Exact entry paths and tiers/parent adaptations are recorded in `component-record.md`.

## Generated actual screenshots — not directly inspected

Eight Debug GPUI captures at `artifacts/ui-component-followup-ui/index.html`: Light/Dark trust-minimum and missing-runtime920x480, Clipboard text300x440, Quick Find General1060x650. Runner verified preview executable/PID/HWND/96DPI/client bounds; PNG dimensions, SHA256 and nonblank sampled colors checked. Only `--ui-preview foundation`, synthetic scratch config/history, disconnected bridges; startup runtime=false/capture=false/indexer=false/platform=false. Exact owned preview processes cleaned up.

BrowserPreview opened, but screenshot tool returned “Browser view did not become visible for the calling session. Switch to that session and retry.” Captures therefore remain generated/integrity-checked, **not directly inspected or user-approved**. Download details/progress have production compiled render coverage, not separate captures in this8image subset. Physical pointer/keyboard, mixed-DPI, reader/provider and installed visual acceptance remain unperformed.

## Concurrent archive session

Session370bbe8f shares this worktree, not an isolated worktree. No direct Desktop-session messaging capability or peer acknowledgement was available. Trellis channel `nxtools-ui-archive-coordination` only publishes durable messages; it is not claimed to notify or automatically coordinate that Desktop session.

File scopes were separate: selected Desktop UI/test files, one QuickLaunch source gate, this task documentation and capture gallery only. Download implementer owned5exclusive files and ran no Cargo. This task did not write archive crate/fixtures/CSharp Archive/task, manifests, lock, shared specs or packages; never terminated peer processes. Scoped rustfmt only, no `cargo fmt --all` writes. Earlier focused batches observed target/package-cache lock waits when the peer started mid-batch; final commands waited for existing Cargo/rustc before each command. No zero-impact or fully coordinated all-session claim: shared builds, lockfile and workspace checks demonstrably interact.

## Release and remaining boundaries

No packaging/install/service/deployment/registry/normal product launch/real user config/history/clipboard/plugin effects, commit/push/archive. Sequence15 installer and packaged Indexer hashes remain unchanged. Prior official-notification and Indexer source fixes preserved and not newly packaged. No UI compiled blocker remains under the verified current dependency snapshot. Remaining: external archive workspace/format completion, direct screenshot inspection, physical/native/DPI/reader/user acceptance. New package requires separate user request.
