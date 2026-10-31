# Phase F spike — independent acceptance

Date: 2026-10-22. **Bounded source/controlled evidence accepted; workspace integration NOT PASS.
Not full P208, P206 live Apply, P207, Phase F or first-release acceptance.**
Current status remains owned only by [checkpoint](phase-a-checkpoint.md).

## Scope and conclusion

Independently read the delivered [outcomes](phase-f-spike-outcomes.md),
[P208 contract](phase-f-spike-contract.md), [Shell contract](phase-f-shell-contract.md),
[OLE reference](../../../docs/reference-code/archive-virtual-drag.md) and
[Shell interface reference](../../../docs/reference-code/archive-shell.md), then traced Rust callback /
wire validation / pipe admission / dedicated STA / IDataObject / original ArchiveEntrySession /
worker / cache / native IStream and the pure integration transaction.

- P208 controlled seam: descriptor requests do not materialize. Indexed contents enter the original
  worker/Job/source-binding/output/cache owners; repeated requests reuse materialization and open
  independent native stream cursors. Target-held streams survive session retirement; new requests
  stop. Exact cancellation, Escape, source-close and disconnect wait for actual worker cleanup.
- The real native path has one COPY DoDragDrop call on a dedicated background STA, connected-peer
  HWND validation, existing input-queue/capture helpers and final real button/cancel checks. The
  serial reader is not the OLE loop owner. Production ArchiveBrowser rows remain disconnected;
  the GPUI seam is cfg(test), using synthetic HWND42.
- P206 accepted only as default-off pure current-user Classes plans plus fake-store compensation /
  exact-value uninstall. Capture precedes all writes; attempted index precedes a potentially
  mutation-then-throw Write; uninstall preserves modified or foreign values/children. There is no
  Registry-backed store or actual Apply, UserChoice/default-association mutation or Shell refresh.
- P207 research correctly identifies IExecuteCommand + IObjectWithSelection / IShellItemArray.
  A dedicated signed x64 COM local-server/helper, CLSID/DelegateExecute registration and release /
  Shell lifecycle need new explicit authority; none was implemented or registered here.
- No new confirmed actionable Phase F source defect was found in this bounded parent inspection.
  No second optional review was started; the implementation's aborted review remains INCOMPLETE.

## Independent execution

Evidence root:
`C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\phase-f-independent-20261022-212819`.
Scratch runner: `phase-f-independent.ps1`; per-command raw logs and `commands.jsonl` retain exits.
Every Cargo command inherited jobs1/incremental0/codegen-units256 and checked the shared lane.
Host build preceded Rust checks; the matching isolated-codecs worker was built after those checks
and before exact C# LIST/RUN. No archive split/edit/100k/performance matrix was rerun.

| Check | Independent result |
| --- | --- |
| Isolated C# host build | PASS, 0 warnings/errors |
| Exact Phase F C# selection | LIST10 = RUN10 = PASS10; skip0/fail0/not_run0; all names equal |
| Composition of 10 | 2 direct controlled COM/STA + 2 actual tiny-ZIP worker fixtures + 1 COM ABI/pre-cancel admission fixture + 5 pure fake-store plan fixtures |
| Matching worker | Before/after SHA256 `922C36AF1A19A73C076F936B8344E0D6C615B51E9B9D6DC112160B95A51268ED`, identical |
| Protocol spike | 2 PASS, 65 filtered; not a full Protocol rerun |
| Exact isolated GPUI pressed callback | 1 PASS, 1011 filtered; no physical/native-HWND claim |
| Desktop + Protocol strict Clippy, full dependencies | PASS, `--tests -- -D warnings` |
| Workspace `--tests` | Compilation PASS, not workspace test execution |
| Global `cargo fmt --all -- --check` | PASS at this run, unlike the retained implementation observation |
| One complete Desktop | **1005 PASS / 7 FAIL**, 0 ignored/filtered, total1012, 130.94s test time, exit101 |
| Source/ownership, GPUI release, editor provenance, component catalog | PASS |
| Spec/task/PlatformHost selection/diff gates | PASS; specs109 documents/14 owners, task82 implement/79 check entries |

### Current integration failures — not repaired or rerun here

Full names in `desktop-full-once.log`:

1. `downloads::polish_tests::naming_probe_preserves_name_edited_while_pending`
   (`downloads/polish_tests.rs:3090`, actual `server-package.zip`, expected `late-custom.zip`).
2. `downloads::polish_tests::naming_probe_replaces_non_filename_label`
   (same assertion, actual `Paint.NET 5.1.12`, expected `server-package.zip`).
3. `downloads::polish_tests::naming_probe_replaces_url_auto_suggestion`
   (same assertion, actual `url.bin`, expected `server-package.zip`).
4. `theme::recurrence_tests::recurrence_both_themes_pair_readable_roles_and_component_tokens`
   (`theme.rs:477`).
5. `ui::components::action::tests::primary_apply_text_keeps_readable_contrast_in_normal_hover_and_disabled_states`
   (`ui/components/action.rs:558`, exact HSL comparison differs in floating-point components).
6. `ui::efficiency_tests::height_feedback_history_multiline_rows_grow_with_rendered_content`
   (`ui/efficiency_tests.rs:1023`, final paragraph does not fit the asserted row).
7. `ui::efficiency_tests::height_feedback_long_links_reflow_when_history_width_changes`
   (`ui/efficiency_tests.rs:1057`, 168px vs218px).

These are actual failures in the concurrent workspace snapshot, not an established Phase F causal
attribution. No unrelated product/test assertions were changed. Implementation1003/2 and historical
independent1002/1002 remain immutable observations, not current green evidence. Two concurrent source
files changed during this acceptance: `downloads/new_download_view.rs` and `theme.rs`; therefore the
checks do not certify one frozen final workspace snapshot. No green-hunting full-suite rerun.

## Native evidence gap / stop

Read the preserved implementation `native-abi-run.log`: selected1/run1/pass0/fail1/not_run0,
`immediate OLE cancel: The operation has timed out.` The implementation records the foreground STA
keeping the process alive until its wrapper timeout. This acceptance did NOT repeat that Ole32 loop.
The final passing COM ABI fixture calls the pre-cancel admission boundary and does not enter
DoDragDrop. Neither it nor injected consumers prove a successful actual OLE loop.

Real pressed GPUI HWND/capture release, physical gesture/button continuity, foreign-process target /
COM apartment-marshaled GetData, Explorer drop and full native DoDragDrop lifetime remain
**NOT RUN/unaccepted**. Do not infer Windows unsupported or bypass the binding GUI denial.
Production row hookup is NOT authorized by this bounded acceptance and remains disconnected.
P206 actual registry Apply/unapply and P207 helper/deployment/registration remain unimplemented /
unauthorized. Earlier A–E/JOB07/SAFE08/native/user debts are not waived.

## Preservation

Opening baseline429 files:427 unchanged before report/checkpoint writes; only the two concurrent
files above changed (`preservation.json`). Phase F product/test sources and all preexisting archive
task reports, including handoff outcomes and old negative evidence, were unchanged. No product,
backend, dependency, vendor, system, registration, service, packaging/install/signing, source recycle,
commit or push change was made by this acceptance.

A first background runner ended after printing its evidence directory, before saving any baseline
or launching a command. Its empty directory and runner logs were preserved. The single subsequent
foreground execution produced all results above; no duplicate test/build was started to resume it.
`selection-identity.json` has PowerShell metadata on its summary string; raw LIST/RUN/PASS logs are
canonical, and names/hashes are also checked from those raw logs before documentation closeout.
Task validation still warns that two preexisting context files exceed the32768-byte injection limit;
they were read directly and were not restructured in this scope.
