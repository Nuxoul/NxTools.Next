# Incoming Add and failure feedback

Status: implemented; directed automated and owned native verification passed. Phase E — Format expansion and ZIP editing / in_progress.

## Result

The incoming dialog now names the action and description “添加到当前压缩包”. Explicitly choosing it submits one original Modify/Add request through the existing queue without creating an OperationForm. Deliberate editing and Paste retain their forms; Open-first and New-from-all retain their existing owners.

Normal Add to ZIPs with existing backslash-ended directory records succeeds at root and in an existing subdirectory. The repair normalizes the Add comparison namespace before trimming directory markers; it does not rename external inputs or rewrite retained ZIP record spelling.

Transient refusals and failures use the existing NotificationCenter with an operation-specific title, friendly cause and native publication/cleanup facts. The original passive screen presenter reaches hidden prepare, first frame and reveal while the main window is hidden. Progress retains concise status, actions, optional diagnostics and actual partial/cleanup/unknown-result risks.

Release46 predates this batch and does not contain it. This work does not produce a new package or installation.

## unsafe_path root cause

The failing boundary is:

`worker_zip_edit::additions` → `worker_conflict::add::resolve` → `reserve` → `path_policy::validate_relative`.

The original Add resolver trimmed only `/` from an existing member name. A valid existing directory encoded as `folder\` retained its terminal backslash. The path validator recognizes both separators, so it encountered an empty final component and returned UnsafePath before incoming file payload reads or candidate output.

The resolver now uses:

```rust
let normalized = old.name.replace('\\', "/");
let name = normalized.trim_end_matches('/');
```

This comparison/occupancy normalization preserves the original member index and retained encoded ZIP records. The existing path policy, source identity/revision checks, resource ceilings, complete candidate verification, native publication and cleanup remain authoritative.

Boundary distinctions:

| Object | Finding |
| --- | --- |
| External sources | Ordinary owned Chinese/space file and folder, including an empty child directory; unchanged by Add |
| Internal target prefix | Root and an already existing Chinese/space directory both reproduced the defect |
| Incoming new names | Legal names admitted by the original protected source planner |
| Existing member namespace | Backslash-ended directory left an empty terminal component after slash-only trim |
| Candidate output | No publication on RED; unchanged original bytes and cleanup confirmed |
| Final target | Original native one-shot replacement after candidate verification; one publication on successful GREEN |

The read-only sample contains 197 raw backslash member names, including seven backslash-ended directories. On Windows, Python `ZipInfo.filename` is normalized; `orig_filename` preserves raw spelling. The sample has no Unicode-path extra field. Checked-in owned fixtures reproduce the defect with explicit backslash directory records and no unusual input names.

Historical correlation `c24a35aa-20ed-4329-8881-8d251ea235a2`, at `2026-11-04T08:25:16Z`, records Modify failure `unsafe_path`, zero committed files, zero source/output payload and cleanup complete. Its exact external source vector was not logged. The same earliest failing boundary is independently reproduced with owned normal inputs and isolated sample copies.

## Direct entry and lifecycle

`handle_drop` freezes the complete `ModifyDraft::request` and `EditFormOrigin`: archive path, original browser/window/session/generation/snapshot/source, loaded read connection, internal directory and every external source.

The original outlined Medium Button takes the captured intent once, closes the Dialog and defers cross-entity work. `add_drop` checks the active originating tab and shared `edit_request_current`. `submit_drop_add` installs origin and connection on the original Job record before presentation/pumping. Queued dispatch rechecks the same binding before native Start; stale work terminates locally as `stale_add_target`.

Closed origins, changed folder/generation/snapshot/source, switched tabs and replaced connections cannot redirect the request. Conflict/password prompts, cancellation, same-archive queue ownership, source preservation, actual cleanup and successful new-snapshot refresh/result selection retain their original owners.

The local refusal event `archive.drop_add.refused` records only the browser ID. Original Job accepted/start/terminal diagnostics retain the operation ID across IPC/native, elapsed time and native error codes; no paths, member contents or credentials are added to logs.

## Notification and publication truth

- `incoming_notice`, Add form `fail_submit` and Job terminal `notify_job` use the original NotificationCenter; no new Toast, owner or clock.
- Add failures use “添加文件失败”. Technical codes stay in optional diagnostics. Fixed friendly reasons distinguish path validation, stale source/target, conflict, password/data ambiguity, corruption, limits, connection, file sharing, permission and actual disk-full errors.
- Original terminal dedup is retained. A deliberate Add form keeps sources and can release its submitted state only after the original precommit conditions and affirmative native `output_published=false`; it does not add a duplicate generic error block.
- Native `output_published`, journal and commit state govern “原压缩包未被替换”, published and unknown-result wording. `files_committed=0` alone does not prove unpublished.
- The business test verifies the actual title/cause, one notice after repeated terminal delivery, originating archive target, one independent popup, hidden preparation, frame progression and reveal ACK with the main window hidden. The existing native HWND fixture separately verifies hidden/nonactivating preparation and client readback.

## Reference and components

Saved `bandizip-input-clipboard.md` incoming action3 retains the complete source list and calls the existing current-archive update coordinator; action2 opens creation. Cancel does no work. The indexed legacy NxTools investigation has no general archive-browser owner. The current explicit-choice requirement narrowly overrides the historical form-only drop rule.

The existing Root Dialog, DialogFooter, DialogClose and `Button::new(...).outline().with_size(Size::Medium)` are retained. Buttons remain direct footer children, not `Toolbar.child`, which forces ghost/compact. Notification/NotificationList and the original passive screen host own presentation, preparation, first frame and reveal.

## Changes

| Owner | Files and responsibility |
| --- | --- |
| Desktop incoming | `rust/apps/nxtools-desktop/src/archive/application.rs`: label/description, frozen one-shot intent, original queue admission, local refusal notice |
| Desktop Job | `rust/apps/nxtools-desktop/src/archive/jobs.rs`: shared origin guard, queue-time recheck, origin assignment, operation-specific feedback and publication truth |
| Desktop form/progress | `forms.rs`, `progress.rs`: retained draft, one transient failure owner, concise state and real risks |
| Archive planner | `rust/crates/nxtools-archive/src/worker_conflict/add.rs`: normalize existing separators before directory trim |
| Desktop regressions | `incoming_add_tests.rs`, `application_tests.rs`, `intake_tests.rs`, `job_tests.rs`, `progress_feedback_tests.rs` |
| Native fixtures | `csharp/NxTools.PlatformHost.Tests/EditInteractionTests.Incoming.cs`, `Program.cs`, `TestEffects.cs` |
| Contracts | `docs/reference-code/archive.md`, `.trellis/spec/nxtools-archive/backend/edit-interactions.md`, `.trellis/spec/nxtools-desktop/backend/archive-browser-workflows.md` |

Original R45/AC01/7z fixes and unrelated working-tree changes remain intact. No output optimization, backend or dependency change is included.

## RED and GREEN evidence

Evidence root: `.diagnostics/archive-incoming-add/20261105-repair/`.

| Boundary | RED | Final GREEN |
| --- | --- | --- |
| Original rendered Add button | `desktop-qualified-red.log`: two failed assertions, existing editor opened and old label rendered | `desktop-archive-green.log`: one complete typed Add, no form, requested label |
| Normal existing backslash directory | `normal-red-run.log`: slash root/subdir succeeded; backslash root/subdir failed unsafe_path, zero publication, unchanged original and cleanup true | `native-final-run.log`: all four normal cases succeed; all four Desktop→native cases succeed |
| Isolated sample | `sample-run.log`: root unsafe_path, zero publication and cleanup true | `native-final-run.log`: root and existing-subdirectory copies each publish once |
| Failure feedback | `desktop-feedback-behavior-red.log`: old generic title and progress Alert fail two assertions | `desktop-archive-green.log`: business notice, dedup, hidden/frame/reveal and no generic Alert pass |

The actual Desktop→native bridge consumes real native Browse context/entries, opens the original archive window, performs ZIP qualification, enters the frozen directory and clicks the original rendered Button. C# launches the exact Desktop test apphost with `--ignored --exact`, requires one passed test and the `request_emitted=1 forms=0` marker, then sends the unchanged exported typed command through ArchiveJobSession and the original worker/native boundary. No replacement success DTO is used.

### Final counts

| Verification | Selected | Run | Pass | Skip | Fail | Evidence |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| Desktop archive | 474 | 474 | 474 | 0 | 0 | `desktop-archive-green.log` |
| Original notifications | 46 | 46 | 46 | 0 | 0 | `desktop-notifications-green.log` |
| Worker output / AC01 | 10 | 10 | 10 | 0 | 0 | `worker-output-green.log` |
| Worker Add namespace | 5 | 5 | 5 | 0 | 0 | `worker-preservation-green.log`; 145 scenarios across ZIP/7z/TAR/TGZ/TXZ |
| Worker extraction / 7z ResourceLimit | 11 | 11 | 11 | 0 | 0 | `worker-preservation-green.log` |
| Native final group | 7 | 7 | 7 | 0 | 0 | `native-final-list.log`, `native-final-run.log` |
| ZIP Add password/preservation | 1 | 1 | 1 | 0 | 0 | `password-preservation-list.log`, `password-preservation-run.log`; 17 scenarios |
| Original C# progress and native Create wire | 3 | 3 | 3 | 0 | 0 | `wire-list.log`, `wire-run.log` |

Rust totals: 546 executed/passed. C# final groups: 11 distinct fixtures executed/passed, not_run=0. Each C# RUN uses the identical preceding LIST selection. The Desktop bridge is excluded from the 474-test selection and explicitly executed four times by the native group; its default ignored status is not counted as a pass.

The native final group contains four incoming fixtures plus original notification HWND, Windows descriptor units and actual multi-member drag progress/order fixtures. Incoming operations total14: ordinary4, Desktop4, sample copies2, conflict decisions4. Ten normal/sample operations and overwrite/auto-rename each publish once; Skip/Cancel publish zero and retain exact originals. Source bytes, independently decoded results, original retained record spelling and actual cleanup are checked. Successful incoming fixture roots are deleted after owner disposal.

The ZIP preservation fixture checks14 namespace/decision cases plus correct password, wrong password and password cancellation. Original bytes, zero/one candidate publication, journal, no partials, exclusive source reopening and independent encrypted ZIP decoding are asserted.

Original refresh/result selection, retained deliberate Add forms, shared connection guards, true editing phases, Root drag units and stale/decreasing progress rejection remain covered. HZ3 retry assertions require the exact original `RetireBatch → Start` sequence and one new Start.

### Static and compiler gates

- Desktop tests and archive isolated-codecs all-target Clippy: `-D warnings`, both exit0.
- Scoped rustfmt check: ten Rust files, exit0.
- `spec-contracts.py`:125 documents/14 owners, PASS.
- `gpui-release-contracts.py`: locked0.8.0/pre0.3.9 and two approved deltas, PASS.
- `markdown-editor-contracts.py`: pinned editor provenance/contracts, PASS.
- `archive-tree-root-drag.py`:3 tests, PASS.

All Cargo calls use offline mode, jobs1, incremental0 and `RUSTFLAGS=-C codegen-units=256`, with an idle single build lane. Reproduction commands and artifact accounting are in the evidence `verification-index.md`.

## Limitations

- Synthetic GPUI and owned native fixtures do not certify physical DOpus “scatter then collect”, visual/DPI/IME/user acceptance or installed/Release behavior.
- Historical exact external inputs for the recorded correlation are unavailable; the evidence proves the same defect with normal owned inputs and read-only sample copies, not exact historical-input reconstruction.
- Source/static gates verify their stated contracts, not product visual or release acceptance. Phase E remains in progress; this batch has no remaining implementation or directed-verification blocker.
