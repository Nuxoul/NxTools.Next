# Audit #4 — Current decision object delivery and R1/R2 correction

## Status, authorization and phase

This remains the sixth individually authorized item, original audit #4. Conflict now presents this prompt's incoming member and complete overwrite target; CreateOverwrite presents this operation's output archive target. The snapshot belongs to the original ticket in `ArchiveJobPrompt.display`, not progress, selection/index or another job. Missing legacy fields explicitly show `本次提示未提供对象标识`, without an invented identifier or automatic approval.

This is stabilization within `Phase A — Engine contracts and safety foundation`, not Phase A–F or overall UI completion. The initial parent review is **CHANGES REQUESTED / NOT ACCEPTED**, with no confirmed production display-chain blocker from its one directed pass. The current correction is limited to R1/R2. Stop after re-delivery for the parent's concrete correction review; item seven is not authorized.

## R1/R2 changes in this revision

### R1 — complete unresolved diagnostic outside default regressions

Only the newly introduced `audit04_create_keyboard_release_and_missing_object_cancel_are_once_only` function was removed from default `archive/job_tests.rs`. Its complete original 76 lines, including `#[gpui_kit::test]`, real Tab/focus/Space down-up, expected-one reply assertion, duplicate-release assertions and the entire later fallback/cancel section, are preserved byte-for-byte in:

`evidence/revision-r1-r2/unresolved-diagnostics/create-keyboard-unresolved-probe.rs.txt`

The complete source context, original file hash and reproduction dependencies are preserved beside it. The original `job_tests.rs` revision-start SHA256 is `1723f2428a980d14a06871069c41ae5a92c5746f308e7e37c52f89347a5c639e`; the complete diagnostic function SHA256 is `d961cc97bed5db46c476dc1d10ee5036eb2d0c26b52bc867308854178d1d277c`, identical to the earlier saved diagnostic. It is explicitly **UNRESOLVED / NOT ACCEPTED**, not included by a Rust module/Cargo target, not ignored or conditionally bypassed, and not counted as a passing test. No pre-existing regression or assertion was removed. The six independently passing audit04 regressions remain standard tests, including CreateOverwrite pointer/missing-object cancellation and Conflict keyboard activation.

No product keyboard behavior was fixed: the original Button/DialogAction/on_ok ownership, vendor sources and #5/default Enter behavior are unchanged. Historical failures remain intact: focused **6/7** and the sole complete Desktop run **1207/1208**, with this diagnostic as their failure. No new complete Desktop run was performed to replace that record.

### R2 — English engineering documents

This task's `prd.md` and `outcomes.md` were intentionally rewritten in English, as required by `.trellis/spec/index.md:39`. Exact phase/API/test names, red/green and intermediate-failure history, compatibility, concurrency and acceptance limits are retained. Product Chinese labels/copy, raw logs, evidence source and other task documents were not translated or rewritten. The original Chinese document bytes remain available through their initial baselines/deltas and this revision's saved baselines. Parent-review documents and the original manifests remain immutable.

## Production chain and original implementation changes

1. Existing ZIP, TAR/four compound formats, Gzip/BZip2/XZ/Zstd and isolated 7z/RAR output calls enter `PrivateOutput::begin_extracted_file`. Ordinary formats already provide the plan-prefixed target; isolated codecs use the original `plan_paths` mapping.
2. `PrivateOutput.display_prefix` records only the presentation prefix after this `reserve_subfolder` succeeds. It never replaces native `plan_prefix`. The complete current relative name is matched exactly against the immutable entries and retains the original member string. There is no basename/global-progress guess and no codec interface or dispatch change.
3. `ExtractionConflict::begin` attaches the display snapshot to the notification for the original Ask pending ticket. Domain `Prompt::Conflict` gains no path fields that could leak through its derived Debug. Original native paths, observation tokens and every decision branch do not consume display.
4. `ArchiveJobSession.Prompt` strictly parses presentation-only display and combines the target with the same immutable `Command.Request.Destination` using string operations, not new filesystem APIs. The original CreateOverwrite runner preflight callback uses that command's Destination/Create.Name.
5. The original typed event -> bridge -> job record -> `refresh_job` -> `ProgressSnapshot` -> `JobProgressView::show_prompt` owner remains. Original `reply_job` prompt equality/ticket/kind/terminal/once-only guards, C# Reply and InteractionPort are unchanged.
6. Official Component0.8.0 Dialog body/footer, vertical one-column borderless DescriptionList public slots and Label display the metadata. Original outline/default Medium32 Buttons and the same Checkbox callbacks remain. Only the Checkbox's placement moves into the original fixed footer so long body content cannot scroll apply-all out of reach. There is no second focus/close/reply dispatcher.
7. Full identifiers receive a visual newline every 24 Unicode scalar values; characters are not removed, ellipsized or parsed as Markdown. The actual Label-role semantic node retains the original full string, and the body can scroll to its end. There is no new Clipboard/Shell/OS action. Default560x300 and minimum480x260 client sizes remain.
8. Necessary construction adaptations are only `display: None` for readmission, the existing Destination fixture and two CLI test literals; their safety assertions remain. `worker.rs` and `worker_wire.rs` were observed/baselined but not modified by this item.

R1/R2 freezes this entire production implementation. Its only Rust source edit is relocation of the one newly introduced diagnostic; the other two existing-file edits are the document translations.

## Compatibility, privacy and preserved safety contracts

- Keep job wire version1 and the original worker schema. `display` is optional/defaulted and omitted for None. New receivers read old omitted payloads; old strict receivers reject populated new fields. This is **one-way legacy reading, not bidirectional mixed-version compatibility**. No unknown-field relaxation, version increase, capability or budget grant is introduced.
- Typed display is valid only for Conflict with a member or CreateOverwrite without a member, with finite UTF-8 byte bounds and no control characters. Invalid types/unknown fields are rejected; missing context has an honest fallback.
- Rust `ArchiveJobPromptDisplay::Debug` and C# record ToString use `[PRIVATE]`; nested prompt/event printing does not disclose the new payload. Original correlated accepted/state/terminal/count/elapsed/native-code events remain low-noise, without ticks/per-frame logs. Synthetic sentinel checks through the actual original C# sink passed.
- Preserve native identity/source session/entry ordinal, overwrite validation, path safety, finite resource budgets, fully verified temporary output/publication, password clearing, ticket/generation, once-only reply, cancel/disconnect/timeout/apply-all, cleanup-held slots and deferred cross-entity work. Native output/session/runner, safety policy, codecs, dependencies and vendor code were not changed.

## Reference and exact API evidence

Read-only reference inspection revisited Bandizip `D:\Tool\Bandizip\langs\SimpChinese.lang:579`–588 for existing/incoming/Skip/Overwrite/Rename/apply-all. No Bandizip GUI, decompilation, disassembly or dependency extraction was performed; resources do not prove dynamic layout.

Licensed Archiver `D:\Code\Archiver-main\core\src\lib.rs:230` decoder selection and `:665` single-file File::create were re-read. Truncating publication/progress-name callbacks were not copied as authorization. No general old NxTools archive UI was identified; no migration source is invented.

Exact Component0.8.0 `dialog/dialog.rs:735` and769–786 provide viewport limits, body scrolling and a separate footer. DescriptionList vertical/columns/item, Label and footer public slots were read. Only the verified entries/call chain/invariants were appended to `docs/reference-code/archive.md`, preserving historical conclusions.

## Historical implementer verification — immutable results

These are source/compiled/synthetic/headless observations, not visual or user acceptance. Original logs remain in `evidence/` and retain their bytes even when a historical filename contains “green”.

| Check | Actual original result |
| --- | --- |
| Typed-wire executable red | Two new tests compiled; runtime strict decoding rejected unknown display, **0/2**. `desktop-object-red.log`; not a compilation failure. |
| Initial actual Dialog green | After the typed/producer/UI changes, both real Dialog object tests passed, **2/2**. `desktop-object-first-green.log`. |
| Actual UI executable red | With the typed model working, a controlled removal of only the two Dialog prompt_objects slots produced **0/2** at actual incoming/output-target-missing assertions. Both slots were immediately restored to progress.rs SHA256 `6fc8f43dabefde15197d8d7f55b5b5050236c3aa299bbeda6310ce2d9422efe3`. `desktop-real-ui-red.log`. This is not an old-keyboard runtime comparison. |
| Expanded/final focused UI | Initially **4/5**, then **6/7** after independent pointer/fallback and Conflict keyboard coverage. The CreateOverwrite Space diagnostic consistently failed with actual0/expected1; an explicit KeyDown/Up attempt did not resolve it. All failures remain in `desktop-object-expanded.log`, `desktop-object-input-green.log`, `desktop-object-final.log`. |
| Worker producer matrix | `prompt_display_contract` **2/2**, covering18 format/strategy runs:10 engine formats plus ZIP/TAR Name, 7z non-solid/solid and RAR4/5 plus7z/RAR Name. Real worker/InteractionPort, controlled fake native observation tokens; Skip never allocates/writes/publishes. |
| Direct worker/safety regressions | interaction4 + job2 + output_readmission3 + prompt2 + cancel1 + extraction9 + wire18 = **39/39**. Original cancel/stale/source/budget/integrity/cleanup assertions remain. `worker-direct-regressions.log`; not the entire Archive suite or native product acceptance. |
| Protocol | Archive job **6/6**, including omission/roundtrip/strict type/unknown-field/shape/bounds/Debug sentinel. Full Protocol **79/80** with the Default Apps negative-test failure preserved. Do not call that full suite green or assert causality merely because lib.rs was not edited by this item. |
| IPC | **42/42**, `ipc-regressions.log`; isolated temporary pipes/synthetic identities, not real product/service connections. |
| C# build | Final Tests build without restore: **0warnings/0errors**. The first build found CS8602 in the newly added test; the local nullable annotation was corrected and its failed build log retained, not counted as behavioral red. |
| Exact C# fixtures | Exact LIST5 then RUN5/PASS5/SKIP0: create confirmation cancel/changed-target, detached preflight cancellation slot, strict wire/pre-cancel picker without opening a picker, session conflict/global admission/cancel, and ZIP decision matrix. GUID-owned synthetic source/output/helper copies with original Job/worker; no product/service/actual HWND/Shell/registry execution. `csharp-five-fixtures.log`. |
| Strict Clippy | Initial new-test unwrap violations were corrected to contextual expect without suppression. Archive isolated-codecs --tests and Protocol --tests passed. Desktop --tests then failed at the shared `archive/shell_integration.rs:256` needless_update. The temporary shared launch_entry unused-import warning later disappeared externally; it is not attributed to this item. |
| Workspace check/format | `cargo check --workspace --tests` passed. Workspace format initially failed for two new protocol test lines plus shared `main_window/archive_page.rs:798`; only the new lines were locally formatted. Eight task Rust files then passed scoped rustfmt --check. No unrelated owner was edited to obtain green results. |
| Sole complete Desktop run | **1207 passed / 1 failed / 0 ignored,1208 total**,236.99s test execution. The sole failure was the newly introduced CreateOverwrite Space diagnostic. `desktop-full-once.log` remains a failed historical run. All other tests, including the original Unlock test, passed; that is not attributed to this display repair. No complete rerun occurred in R1/R2. |
| Source gates | Ten related gates passed: catalog/release/Markdown/spec, archiveCLI/PhaseF shell/spike/release chain, ClipboardUI/DownloadUI. The extra Quick Launch gate still failed at1765; no build/security/gate repair was attempted. |

Every original Rust shell used `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`, serial natural-idle waiting and sufficient timeout. No process/session was killed, paused or asked to yield/reserve/reorder. No duplicate build or residual worker was reported at delivery. No Release/package/install/commit/push/recursive delegation occurred.

## Outstanding diagnostic and Protocol attribution limits

### CreateOverwrite Space remains unresolved

`audit04_create_keyboard_release_and_missing_object_cancel_are_once_only` reached the actual focused `确认覆盖` Button and sent explicit Space KeyDown/KeyUp, then received0 replies instead of1. Its later fallback section did not execute; only the separate passing pointer/fallback test establishes that behavior. R1 relocates this newly introduced out-of-scope diagnostic, not any old regression, and does not repair keyboard activation.

The original baseline's DialogAction.child(Button), on_ok, on_cancel and Conflict decision callback lines match the delivered implementation after stripping indentation only; `evidence/late-instruction/footer-compare.json` preserves that comparison. Exact locked Component/Button/Base source installs keyboard activation only when on_click exists, while the original outer DialogAction owns pointer click dispatch. This is source evidence supporting the existing owner limitation, **not a separately run complete old-UI runtime proof**. No shared production baseline was restored to reproduce the old keyboard path, no no-op on_click/new dispatcher/vendor patch was added, and no default-Enter or overall accessibility success is claimed.

### Historical Protocol failure is not rewritten

The original full Protocol panic at `lib.rs:2332:13` asserts that `open_default_apps_settings` decoding rejects the loop's data. The log does not print the current loop value, so its exact failing input cannot be recovered or invented. At the later read-only check, the source listed four inputs: `{"uri":"ms-settings:privacy"}`, `{"path":"C:\\x"}`, null and an empty array. The then-current lib.rs SHA256 was `b19d21a17d720365839374ed51394be717b395d549ea04fc5263f128c24a6384`, with a dedicated empty-map Deserialize Visitor at491–513 and the test at2342. The original failed-run lib.rs hash/full snapshot was not saved; those different source times cannot be conflated.

A bounded read-only call-chain/own-delta check found separate tagged PlatformCommand branches for ArchiveJob and OpenDefaultAppsSettings; the new display belongs to ArchiveJobEvent.prompt, and this item changed neither that enum/default-apps DTO/parser/test nor serde dependencies. This supports branch separation but does not prove full causality. Earlier “unrelated” wording was too absolute and was explicitly corrected by `late-instruction-addendum.md`. No protocol-wide green or Default Apps fix is claimed from the initial delivery.

## Parent review results — separate attribution

`parent-review.md` is immutable and records the parent's own directed pass: focused6/7 with the same keyboard failure, worker display2/2 plus safety37/37, Archive Protocol6/6, IPC42/42, C#build0warnings/0errors and exactLIST5/RUN5/PASS5/SKIP0, ten related source gates,13 exact dirty-deltas/16 source hashes/81 evidence hashes/13 protected hashes and concurrency-segment reconstruction.

On the later shared tree, the parent also ran the Default Apps test alone **1/1PASS**, strict Desktop/Archive/Protocol Clippy **PASS**, and workspace format **PASS**. These shared-tree changes are not attributed to audit #4 and do not replace the implementer's historical Protocol79/80, Desktop Clippy or format failures. A targeted Default Apps pass is not a full Protocol rerun. The parent's first PowerShell aggregate-result script had a scope/array error after native command results were captured; this is recorded as a harness aggregation error, not a product failure or repeated-runtime-test justification.

## Fresh R1/R2 verification

| Check | Actual revision result |
| --- | --- |
| Standard `cargo test -p nxtools-desktop audit04_ -- --test-threads=1` | **6/6PASS,0failed,0ignored**. Only the six object-display-scope regressions are registered; unresolved keyboard diagnosis is not counted as green. |
| Original archive job regressions | **11/11PASS**, using `archive::application::jobs::tests::` with `--skip audit04_`; no original test/assertion was removed. |
| Strict Desktop lint | `cargo clippy -p nxtools-desktop --tests -- -D warnings` **PASS** on the current shared tree. This is not a repair of the historical shared-tree failure. |
| Scoped format | `rustfmt --check --edition 2024 --config skip_children=true .../archive/job_tests.rs` **PASS**; no formatter write or whole-tree formatting. |
| Document/source gates and diff | spec114documents/14owners, component catalog, GPUI release, pinned Markdown editor, and scoped git diff --check all **PASS**. These are static checks, not product/native acceptance. |

Revision checks use the same required low-memory Rust environment, serial idle waiting and sufficient timeout. Document/spec gates, exact-scope/diff and versioned-manifest/frozen-hash checks are recorded in `evidence/revision-r1-r2/`. This revision does not rerun complete Desktop, full Protocol, worker/native/C# suites, or the previously failed extra Quick Launch gate; their historical and parent-run results remain separately labeled.

One revision evidence-aggregation command used invalid PowerShell `ConvertTo-Json -InputObject @(...)+@(...)` argument composition and failed after the four additional source hashes had been checked. The corrected command first assigned the combined array, then serialized it; all28 frozen source rows were retained. This tool-script error did not modify product files, invalidate the captured native command results, or cause any runtime test rerun.

## Dirty baselines, concurrent edits and versioned evidence

Every original existing file was saved at its current dirty baseline before editing, not at HEAD. The first delivery had13 changed existing task files, one new worker test and two task documents, plus two observed-but-unmodified source files. All13 prior-item protected production hashes matched. Exact old baseline bytes, forward/reverse reconstructable own-deltas and original logs remain in `evidence/`; parent-review files are not edited.

Two original external formatter events were detected and writing stopped immediately: job_tests after its initial append changed1E0F -> E176, and Protocol after its append changed9325 -> EE7F. Original baselines and observed live snapshots were retained. The parent explicitly authorized only bounded resumed changes; external formatting was not claimed as this item's work. The job_tests pre-existing849-line prefix was verified byte-identical, and its original-own/external-format/resumed-own three segments are exactly reconstructable. Protocol's exact pre-format intermediate bytes were not saved, so no isolated exact formatter delta is invented; task-start -> final and observed-live -> resumed-final are exact. The sole necessary old fixture adaptation was display:None, not a changed behavior assertion.

R1/R2 saves new current dirty baselines for job_tests, PRD and outcomes. The original `evidence/delivery-manifest.json` SHA256 `bea18e4b467792ba69e46cb825d96b405a63e684634c1638e92ea64d27371924`, `own-deltas.json` SHA256 `ce07e982cc393e881976240d5cd20900db633bb21db074b278f60bb5b14d277b`, historical manifests/logs and parent-review documents remain frozen. Those original manifests describe the original version; the new revision manifest records the three changed current hashes, their baseline -> after deltas and the unchanged protected sources. It does not silently make the old manifest describe the revised documents/test registration.

The complete unresolved diagnostic source hash, context/dependencies, original failed commands/logs and status remain readable under `evidence/revision-r1-r2/unresolved-diagnostics/`. Fresh revision logs and exact change/frozen-artifact checks are separately indexed. No cross-session scratch access is required for parent review.

## Not executed and acceptance limits

No real Desktop/PlatformHost/service, native HWND/Explorer/Shell/picker/clipboard or real user config/database/archive/registry was operated. No physical input/IME, multiple-DPI, UIA/reader, screenshot/visual comparison or user acceptance was performed. Headless semantic/geometry/paint lifecycle evidence is not reader or native UI acceptance; C# scoped native-file/Job fixtures are not live application lifecycle acceptance. Cross-layer synthetic tests cover real boundaries separately, not one live product process/UI end-to-end session.

The production CreateOverwrite keyboard diagnostic remains outstanding. Standard6/6 verifies only this item’s display/preserved-decision scope. Historical failed suites and the extra Quick Launch source gate remain explicit. R1/R2 re-delivery is **pending parent inspection and acceptance**, with no phase or next-item advancement.
