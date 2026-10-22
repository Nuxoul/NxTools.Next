# Phase C — Archive browser and entry lifecycle: implementation outcomes

Date: 2026-10-21. **Partial implementation, not Phase C completion or independent acceptance.**
The sole current checkpoint is [phase-a-checkpoint.md](phase-a-checkpoint.md). User approval,
internal dependency exception and reference/component invariants remain in
[implement.md](implement.md) and [phase-c-browser-contract.md](phase-c-browser-contract.md).
Phase A/B remain incomplete. No D/E/F, packaging, installation, real registry/service changes,
product GUI launch, Shell execution, commit or push occurred.

## Production behavior implemented

- Formal settings-page Open entry sends a typed single-file archive picker through existing
  PlatformHost dispatch, bounded STA slot and cancellation helper. Selected/Cancelled/Failed
  replies are typed. Cancel is silent; submit/native failure has a local retryable error.
  Source HWND and weak originating-window lifetime are retained. No path-input mock remains.
- `ArchiveApplication` composes the accepted `nxtools_archive::product::ArchiveReadClient`.
  `archive_client_bridge` shares the existing router/archive transport but not Desktop's ordinary
  notification receiver. Browser reads explicitly opt into finite v2 browser/source context;
  retained batches precede exact credit. No second pipe, process runner or codec owner.
- Canonical paths key independent activating 960x640/min720x480 windows; duplicate opens focus
  existing windows. Retirement drops original reads/cancels entry work; generations/weak views
  prevent stale installation. Source failure/change preserves last-good content as stale/read-only.
- Official Tree, resizable panels/columns, Toolbar.content outlined Medium Buttons, per-window
  PopupMenu/DropdownMenu, Input, Empty, Alert, Dialog, TitleBar and StatusBar. Only archive details
  use the approved official ListItem/public GPUI uniform-list fallback. Other DataTable consumers
  and locked Kit0.7.1/vendor sources are untouched by Phase C.
- One model owns immutable snapshot+ordinal selection/focus/range; derived folder IDs are
  snapshot+path. Ctrl/Shift, Shift+arrows, Ctrl+A, Home/End/Page, right-click retention,
  sort/filter/navigation, deferred Tree synchronization, horizontal scrolling and released
  resizable columns have production-view regressions. Unknown metadata remains unknown.
- Safe single ordinary-file materialization reuses original native source lease, Job runner,
  protected output/integrity/cleanup. Output is under a GUID-owned lease's `files` child, separate
  from its manifest. Rust checks outer success, typed operation/session/generation, current
  snapshot and actual cleanup before Shell command submission or nested-browser creation.
- One file per activation, 512MiB temporary-output limit, depth <=3, active-content confirmation,
  MOTW setting forwarding, nested leases independent of parent, cancellation/late-result release.
  Shell command receipt is not external-process lifetime. Edited/locked files are retained.
- Saved Phase B filename encoding seeds new browser reads and saved MOTW preference seeds entry
  commands. Settings-success propagation is deferred and does not overwrite other config sections.
- ExternalPaths drop-in provides an explicit open-as-archive choice; multi-path/create/add/edit
  are visibly unavailable in this slice, not fabricated success or fake native drag-out.
- Payload-free lifecycle diagnostics retain operation correlation, transitions, elapsed/native
  error/cleanup facts; no member names, file paths, passwords or contents are added as log fields.

## One bounded source review and fixes

One optional read-only source review, not an independent phase acceptance, found:
1. Failed/unsealed leases consumed all32 cache slots. Added cleanup-bound abort; repeated34
   missing-helper requests now retain original helper_missing instead of exhausting admission.
2. 7z/RAR charged all declared members against selected temporary output. Preflight now checks
   only selected ordinal; decode work retains original finite engine ceilings and admitted
   working-memory bound. Synthetic600MiB metadata +8byte selection test passes; no large file run.
3. Successful release left manifests/session roots and a permanent4096-cycle stop. Clean output
   retirement now removes owned manifest and identity-checked empty ancestry; tombstones are
   bounded. No recursive delete, arbitrary-path cleanup, process kill or forced lock removal.

The admission-failure generation bug also received an actual red->green GPUI regression:
view generation0 vs slot1 remained loading before the fix; now submit failure reaches Failed.
The cache exhaustion regression was actually red before cleanup-bound abort. Earlier compiler
and API failures are implementation feedback, not unsupported Kit capability evidence.

## Executed verification (all final commands exit0)

Every Rust invocation used CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0,
RUSTFLAGS=-C codegen-units=256; only one Cargo/rustc build was started at a time.

| Check | Actual final result |
| --- | --- |
| `cargo test -p nxtools-desktop archive:: --offline -- --test-threads=1` | 34 pass,0 fail/ignore; included in full count below |
| `cargo test -p nxtools-desktop --offline -- --test-threads=1` | **908 pass**,0 fail/ignore;120.77s test time |
| `cargo test -p nxtools-protocol -p nxtools-ipc --lib --offline` | Protocol57 + IPC31 pass |
| `cargo test -p nxtools-archive --features isolated-codecs --test product_read_consumer --test product_read_cancellation --test worker_wire --test worker_extraction_contract --offline -- --test-threads=1` | consumer6,cancellation10,wire17,extraction1 pass |
| `cargo test -p nxtools-archive --features isolated-codecs --lib browser_budget_tests --offline` | 1 pass,16 filtered; synthetic metadata only |
| Three exact `product_read_chain` filters | actual typed-pipe open1,finite-v2 chain1,unavailable-source/cleanup/native-error1 pass; no100k/large/performance matrix |
| Strict Clippy Protocol/IPC/Archive/Desktop `--bins` and separately `--tests` | PASS `-D warnings` |
| Strict Archive `--features isolated-codecs --all-targets` Clippy | PASS `-D warnings` |
| `cargo fmt --all -- --check` | PASS |
| C# test-project build `--no-restore -v:q` | 0 warnings,0 errors |
| C# `--fixtures --list "archive browser"`, then identical RUN | selected=run=pass5,skip=fail=not_run0 |
| Architecture source gates | desktop-shell-lifecycle,ui-preview-isolation,ui-consistency,ui-token-boundaries,platformhost-test-selection PASS |
| Documentation gates | spec-contracts106documents/14owners; component catalog79; task context validate PASS |
| Preservation/whitespace | all142 opening historical task hashes unchanged; `git -c core.safecrlf=false diff --check` PASS |

Three exact read-chain filters:
- `product_open_uses_typed_pipe_original_job_and_actual_worker`
- `explicit_finite_v2_uses_actual_product_chain_without_raising_default_limits`
- `unavailable_source_remains_typed_with_actual_native_error_after_cleanup`

C# five names:
- archive browser fixture session stale missing helper
- archive browser fixture empty context actual budget
- archive browser worker selected revision lifecycle
- archive browser fixture strict wire backward defaults
- archive browser fixture owned lease edited retention

The actual native worker fixture is a small synthetic ZIP: only selected output is materialized;
source revision change rejects; exact cancel/stale generation/duplicate/release are checked. It
never Shell-opens. 7z/RAR selection code is compiled/source-checked plus synthetic budget test,
not a full native per-format selected-entry interoperability claim.

Logs are in session scratch `163f435b-1ee5-4c07-8102-31a2c7441b9b`, principally
`phase-c-desktop-full.log`, `phase-c-desktop-final-scope.log`, `phase-c-wire-ipc-final.log`,
`phase-c-read-worker-final.log`, `phase-c-chain-*.log`, `phase-c-selected-budget.log`,
`phase-c-clippy-*-final.log`, `phase-c-fmt-final.log`, `phase-c-csharp-final-{build,list,run}.log`.
No historical evidence file was rewritten to hide an intermediate failure.

## Exit-condition mapping and remaining work

| IDs | Evidence / current status |
| --- | --- |
| UI02 | Tree/implicit folders/single-root/parent/comment/status source + TestWindow coverage. Live screenshot comparison NOT RUN. Solid indicator remains unknown when read context supplies no fact. |
| UI03 | Model and production ListItem selection/sort/filter/virtual scroll/semantics regressions pass. Native UIA/paint/pointer acceptance NOT RUN. |
| UI04 / LIFE01 | Deferred immediate Tree/list callbacks, snapshot replacement and no leased-entity reentry regressions pass. |
| UI05 | Implemented Open/Filter/Enter/Backspace/F4/Escape/list keyboard routes tested; D/E command shortcuts intentionally absent, not marked complete. Actual Tab/focus/native input NOT RUN. |
| UI06 | TestWindow minimum viewport/horizontal/footer/empty/error tests pass. Default/minimum x Light/Dark/CJK real captures NOT RUN. |
| UI07 / LIFE03 | Keyed duplicate/close/reopen/two-window source and tests pass. **Remembered window bounds/column presentation state is not implemented**; minimized restore/mixed-monitor/native maximize acceptance NOT RUN. |
| LIFE02 | Closed picker origin, old snapshot, cancelled/retired entry, disconnect and weak-view retirement regressions pass. |
| LIFE04 | Source-bound selected ZIP native fixture and Rust Shell-submission gate pass; active-content Escape sends no materialize/Shell and clears pending. Actual Shell launch NOT RUN. **Encrypted entry temp-open is explicitly UnsupportedOperation; no browser password interaction is delivered.** |
| LIFE05 | Parent-close/child lease, edited/locked retention and later owned-manifest cleanup fixture pass. **No age/expiry policy or startup-triggered sweep is implemented**; current sweep is bounded on new materialization admission. External viewer physical lifecycle NOT RUN. |
| LIFE06 | SourceChanged invalidates execution binding; failed read admission/refresh retains last-good stale/read-only and explicit retry. No proactive filesystem watcher is claimed. |

**Phase C is NOT complete and is NOT at independent acceptance.** At minimum, window/UI-state
persistence and the missing expiry/startup cleanup behavior remain source gaps; encrypted-entry
UI behavior is a declared limitation requiring reconciliation with the Phase C UI/password exit
criteria, not a silently passed item. Recent history and broader Phase B unfinished settings
remain their original ownership. No D/E/F work is pulled forward to conceal these gaps.

The session editing workflow also reached its per-path failure limit after repeated exact
`EDIT_LINES_UNSEEN: anchors reference lines never displayed` replies on the parent application
path. Further edits to that path stopped in this user turn; the unconnected UI-state draft was
removed, not counted as implementation. Existing verified code remains buildable/tested. Continue
from this checkpoint in a subsequent turn, without repeating fallback/spike authorization.

## Evidence that remains NOT RUN / permissions

Existing GUI self-app safety denial remains binding. No product/preview launcher, renamed helper,
PowerShell launch workaround, screenshot/input/DPI/UIA or user acceptance was attempted. TestWindow,
synthetic owned native cache files and test apphost/worker are not actual GUI or release acceptance.
No real registry/service/Shell mutation, package/install, commit or push. Trellis validation retains
its documented context-size warnings for platformhost/native-contracts.md and archive reference;
these are truncation warnings, not a document gate failure.
