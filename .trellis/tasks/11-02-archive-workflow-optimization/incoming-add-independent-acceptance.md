# Incoming Add — independent acceptance

Status: accepted scoped automated and owned-native verification, 2026-11-05.
Overall: Phase E — Format expansion and ZIP editing / in_progress.

## Result

The incoming action is “添加到当前压缩包”. Choosing it submits one original Modify/Add request without creating an OperationForm. Deliberate Add and Paste retain their forms. Open-first and New retain their existing owners.

Normal file/folder Add succeeds against ZIPs with backslash-ended directory records, both at root and in an existing subdirectory. The original read-only sample succeeds through isolated copies at both destinations. Existing encoded member spelling, external sources, validation, candidate verification, one-shot native publication and cleanup remain intact.

Failures use the existing NotificationCenter with an operation-specific title and friendly cause. Native publication/journal facts govern retained-original, published and unknown-result wording. The original passive screen presenter reaches preparation, first frame and reveal with the main window hidden. Generic duplicate progress failure prose is absent; actual partial-output, cleanup and unknown-result risks remain.

## Source and boundary review

The reviewed implementation is the current working tree at HEAD `0940eec23286c1a750b3aff4284f32e76da68164`, including its uncommitted implementation. Product behavior and test assertions are unchanged by this acceptance; `output_readmission.rs` follows rustfmt with the original guards intact.

- `application.rs::handle_drop/drop_add_target` freezes the full original request, every source, internal directory, archive/browser generation/snapshot/source and read connection. The rendered outlined Medium Button takes the intent once and defers application work.
- `add_drop` checks the active originating tab and `edit_request_current`. `submit_drop_add` assigns origin/connection before presentation and pumping. `send_job_attempt` refuses a stale queued target before native Start. Closed, navigated, replaced or reconnected origins cannot redirect Add.
- `worker_zip_edit::additions` passes original members to `worker_conflict::add::resolve`. Existing `folder\\` directories previously retained an empty terminal component after slash-only trimming. The resolver now normalizes admitted separators before trimming directory markers in the comparison namespace. Original member indices and retained encoded ZIP names are not rewritten. `path_policy::validate_relative`, protected source identities, resource ceilings and original publication remain authoritative.
- `incoming_notice`, Add form submission errors and `notify_job` use the original notification ingress. Terminal deduplication and origin-window metadata remain. Failed deliberate Add drafts recover only after precommit/cleanup checks and affirmative `output_published=false`; zero committed files alone is insufficient.
- The existing Root Dialog/Footer/Close and official `Button::outline().with_size(Size::Medium)` retain visible boundaries without Toolbar child adaptation. The existing Notification/NotificationList and native passive host own presentation. Saved Bandizip incoming action3 provides the direct current-archive update reference; no new runtime, control owner or dependency is introduced.

## Fresh verification

Evidence: `.diagnostics/archive-incoming-add/20261105-independent-acceptance/`.

| Selection | Executed / passed | Fail / skip |
| --- | ---: | ---: |
| Desktop archive, bridge excluded | 474 / 474 | 0 / 0 |
| Original notifications | 46 / 46 | 0 / 0 |
| Worker output / cross-member abort | 10 / 10 | 0 / 0 |
| Worker Add namespace | 5 / 5; 145 scenarios across five formats | 0 / 0 |
| Worker extraction / native ResourceLimit | 11 / 11 | 0 / 0 |
| C# original progress and native Create exports | 3 / 3 | 0 / 0 |
| C# incoming / notification HWND / descriptor / drag progress | 7 / 7 | 0 / 0 |
| C# ZIP Add namespace and password preservation | 1 / 1; 17 scenarios | 0 / 0 |

Rust total: 546 distinct executed passes. C# total: 11 distinct fixture passes, `not_run=0`, each using identical preceding LIST and RUN selections. Four explicit executions of the normally ignored Desktop bridge are separate from the 474-test count.

The actual Desktop bridge consumes native Browse context and entries, qualifies the original browser, enters the requested folder, clicks the original rendered Add Button and exports exactly one unchanged typed command with `forms=0`. That command proceeds through the original C# Job, worker and native publication. All four slash/backslash × root/subdirectory combinations pass.

Incoming native operations total14: Desktop4, ordinary4, sample copies2 and conflict decisions4. Ten ordinary/sample operations plus Overwrite and AutoRename publish exactly once; Skip and Cancel publish zero and preserve exact originals. Independent decode/content checks, source preservation and actual candidate cleanup pass. All ten saved success terminal DTOs independently agree on `Committed`, `output_published=true`, one published journal entry and completed cleanup.

The notification business test verifies title/cause, repeated-terminal deduplication, originating archive target and the original independent popup preparation/frame/reveal route while main is hidden. The native HWND fixture separately verifies hidden/nonactivating preparation, all six anchors, exact client readback and separate reveal. Stale button and queued-target tests assert zero native Start. Original refresh/result selection, deliberate Add drafts, conflict/password/cancel, drag progress/Root units, abort cleanup and 7z ResourceLimit regressions pass.

Worker and Desktop test builds pass; C# build has zero warnings/errors. Strict Desktop bin/tests and archive isolated-codecs all-target Clippy pass with warnings denied. Ten-file rustfmt and scoped whitespace checks pass. Spec contracts, locked GPUI release, pinned editor and Root drag gates pass. All Cargo commands use offline mode, jobs1, incremental0 and codegen-units256.

## Release and limitations

Release46 was already delivered before these changes and does not contain them. Formal packaging of this accepted working tree is a separate subsequent operation.

This acceptance does not certify physical DOpus/Explorer interaction, visual/DPI/IME/user acceptance, installed behavior, service deployment or end-to-end performance. G-00, OPT-00/01 overall, complete G-04, real G-18/G-28 and G-29 prerequisites remain open. Historical reports and evidence retain their original scope. No user archive was modified and no member was executed.

## Packaging preservation

The workspace format gate passes. Four additional original output-readmission contract/chain tests pass after the format-only change; these are separate from the546incoming verification total. No admission, reclamation, retry, native publication or cleanup policy changes.

The original ZIP output-readmission test adapter projects native terminal authority before later generic worker frames, matching the existing 7z/RAR adapters. Native source change remains typed `SourceChanged`; publication uncertainty remains authoritative. A deterministic C# projection regression passes exact LIST1/RUN1 with zero skip, and the original default Rust readmission chain/contract4passes retain their assertions. Product runner, retry and publication behavior remain unchanged.

## Default workspace validation

The unfiltered serial workspace run (`cargo test --offline --workspace --jobs 1`, `RUST_TEST_THREADS=1`) exits101. Across69completed test binaries it records2487passes,1failure and2inherent ignores; Desktop records1744passes,1failure and1ignore. Targets after Desktop are not executed. The separately executed incoming native bridge retains its original evidence and count.

The sole observed failure is `main_window::tests::download_context_menu_omits_actions_that_cannot_run_for_the_selection` at `main_window/tests.rs:4285`. Its source-text assertion rejects every `.disabled(` occurrence, including the existing `.disabled(batch_busy)` behavior required by the current asynchronous download-batch contract in `ui-experience-contracts.md:574`. Capability-plan assertions before that check pass. No download source or assertion is changed by this validation.

Formal packaging is paused at the user's request. Full default verification remains blocked by the unrelated download test; no download test or product logic is changed. No release after the reserved49is started. Release46 remains the last delivered package and excludes this incoming batch. Evidence: `.diagnostics/archive-incoming-add/20261105-independent-acceptance/default-validation/`.
