# Step4 — exact edit interaction verification matrix

2026-10-29. **Phase E — Format expansion and ZIP editing**, fourth item only. Implementation delivery for coordinator independent acceptance, not self-acceptance/PhaseE/product/release completion. Read [outcomes](edit-interaction-outcomes.md), [before-code route](edit-interaction-route.md), [owner contract](../../spec/nxtools-archive/backend/edit-interactions.md).

Evidence root (this implementation session): `C:\Users\Administrator\.pi-desktop\scratch\0c0e5320-11c4-40dd-bb50-f6a437188fc1\edit-interaction`. All names below refer to files there unless a repository path is given. `command-ledger.json`, `evidence-ledger.json`, `delivery-source-hashes.json`, `opening-drift-delivery.json` retain exact commands/exits/paths/hashes. Raw PowerShell redirected logs are UTF-16 BOM; summary parser reads the BOM, not assumed UTF-8. Ordinary incremental build, not clean build. No skipped test is counted as PASS.

## Actual operation × entry × state

| Operation | Qualified ZIP | Qualified7z/TAR/TGZ/TXZ | Original entry and guard |
| --- | --- | --- | --- |
| Add local file/tree/empty member | yes | yes | outlined toolbar/edit menu/right-click/drop -> same Modify form; protected picker/opaque native IDs; target current folder frozen; Ask current-job policy |
| Delete | yes | yes | toolbar/menu/right-click/Delete -> one form; immutable original ordinals, descendant expansion/overlap dedup; legal empty result; no delete-old-first |
| ReplaceContent | **unavailable** | one regular member + one protected input | official toolbar/menu; directory/multiple/warning/missing member refused; distinct typed action, no Add masquerade |
| Rename file | yes | yes | toolbar/menu/right-click/F2 -> same form; original ordinal, not sorted row |
| Rename explicit directory subtree | yes | yes | slash-ended target and original member/descendants; occupied/self/unsafe collisions are backend refusals, not delete target then rename |
| Rename implicit subtree | unavailable in existing ZIP UI identity route | yes | real original descendant ordinal + slash-ended original source_prefix, same frozen intent |
| NewDirectory | yes | yes | original toolbar/menu/form; capability-specific |
| Comment | yes | **unavailable** | original ZIP form only |
| Read/extract/preview | independent | independent | not globally disabled by edit refusal |

Loading/stale/retired/missing completed-worker facts/failed source/split/unsupported structure retain specific refusal. Application entry AND final form recheck original window generation/path/browser/snapshot/source/capability/member presence and kind. Worker/native independently revalidate original revision/native identity/ordinal/source state. Frozen form does not rebind to a changed current selection. Drop requires unchanged generation/snapshot/current folder before opening the original Add form. Form/picker/dialog cancellation has no task/config write or source I/O. Physical external drag is not claimed by synthetic app-handler tests.

## New production regression names / true RED→GREEN

| Exact test / seam | RED evidence | Final evidence |
| --- | --- | --- |
| `step4_tar_original_pointer_menu_key_form_dispatch_once_and_stale_refuses` | `entry-red.log`, no form 0 vs1 | `entry-green.log`; full Desktop PASS |
| `step4_tgz_original_pointer_menu_key_form_dispatch_once_and_stale_refuses` | same real3FAIL batch | same3PASS; full Desktop PASS |
| `step4_txz_original_pointer_menu_key_form_dispatch_once_and_stale_refuses` | same | same |
| original7z worker conflict safety test, exact object display assertion | `conflict-red.log`, display Null | `worker-conflict-green.log`/Archive44 PASS |
| `step4_tar_edit_ask_identifies_archive_objects_before_single_candidate` | `tar-conflict-red.log`, display Null | original TAR worker/matrix PASS |
| `step4_modify_conflict_dialog_identifies_internal_objects_and_scoped_decision` | `dialog-red.log`/`presentation-red.log`, missing archive scope | real rendered Dialog labels/unknown vs0/pointer Skip+ApplyAll/current ticket PASS |
| `step4_edit_warning_terminal_closes_original_form_without_retry` | `presentation-red.log` | warning/skip completion PASS, native commit still explicit |
| `step4_add_form_displays_internal_folder_and_truthful_conflict_policy` | `presentation-red.log` | original rendered Form/Field policy/internal folder PASS |
| `step4_drop_add_uses_format_neutral_dialog_and_cancel_has_no_edit` | `presentation-red.log`, ZIP-only title | actual app dialog/Escape/no Job IPC PASS |
| `step4_committed_edit_refresh_restores_folder_and_new_name_not_old_ordinal` | `refresh-real-red.log`, actual refreshed ready snapshot folder empty vs a | actual original read IPC handshake/ACK/new snapshot/new ordinal1 restoration PASS |
| `step4_zip_add_original_conflict_tickets_and_one_candidate` | `zip-conflict-red.log`, worker EOF on unsupported Ask | actual worker stale+duplicate tickets; four decisions; independent zipfile exact bytes/names; one commit or zero PASS |
| `step4_zip_autorename_reserves_future_incoming_names` | `future-name-real-red.log`, later incoming name collided/candidate refused | `future-name-green.log`2PASS; Archive44 PASS |
| `step4_original_app_and_final_form_refuse_missing_or_wrong_kind_member` | `member-guard-red.log`, missing999 opened original form | `member-guard-green.log`1PASS; before form and final queue/IPC both refuse |
| `step4_drop_add_rechecks_frozen_folder_snapshot_and_generation` | preservation coverage, not separate product RED claim | valid unchanged target opens same Add form; wrong generation/snapshot/folder/None zero forms/jobs PASS |
| `step4_directory_refresh_transforms_paths_and_drops_stale_generation` | preservation coverage | implicit subtree paths rename, deleted selection removal, wronggeneration hint discard/new RowIds PASS |

Invalid-fixture/compile rounds are **not** product RED: `refresh-red.log` missing test traits/ID; `refresh-red-second.log` missing handshake/valid ordinal; `future-name-red.log` wrong fixture `source_id` instead of actual `token`; `step4-final.log` unqualified preview fixture in valid-drop control; `desktop-archive-complete.log` private format field access. Each retained, repaired at fixture/original public install boundary, no product weakening.

## Desktop / lint / workspace / build

| Exact command | Log | Result |
| --- | --- | --- |
| `cargo test -p nxtools-desktop --locked --offline --bin nxtools-desktop archive -- --test-threads=1` | `desktop-archive-delivery.log` | **332PASS/0FAIL/0ignored/1167filtered**, exit0; original321+11 Step4 cases. Includes HZ118/icon8/WAC12 and main-window archive integration |
| `cargo clippy -p nxtools-archive -p nxtools-desktop --features nxtools-archive/isolated-codecs --locked --offline --tests -- -D warnings` | `strict-delivery.log` | exit0; first two Step4 cmp_owned/collapsible_if errors fixed without suppressions |
| `cargo check --workspace --tests --locked --offline` | `workspace-delivery.log` | exit0 |
| `cargo build -p nxtools-archive --features isolated-codecs --bin nxtools-archive-worker --locked --offline` | `worker-build-v2.log` | exit0; current frozen worker3396A7A9…044FA7 |
| `dotnet build csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj --no-restore` | `dotnet-final.log` | exit0,0warnings/0errors |
| `rustfmt --check --edition 2024` with exact17Rust paths in source ledger | `fmt-final.log` | exit0 |
| `git diff --check --` with exact23source paths | `diff-check-final.log` | exit0 |

Every Cargo invocation first checked idle shared cargo/rustc and set in that same process `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`. Long timeouts, serial tests, locked/offline. No name-kills/duplicate build/channel reservation. Archive-only direct changes; Protocol/IPC/Core product sources were not modified by Step4, so their earlier run counts are not claimed as rerun here. `archive::` selection298 was discovered narrower than the original `archive` selection; final restored the full range332, not a shrinking baseline.

## Archive44 exact target set

Command: `cargo test -p nxtools-archive --features isolated-codecs --locked --offline --no-fail-fast --lib` plus `--test <name>` for every non-lib row below, followed by `-- --test-threads=1 --skip default_budget_lists_one_hundred_thousand_independent_entries`. Full expanded argv is in `archive-delivery.json` and `command-ledger.json`. Env `NXTOOLS_TAR_STREAM_FIXTURES` points to this round's independently generated GUID-owned fixtures; `NXTOOLS_TAR_AC01_EVIDENCE` points to `raw-ac01-delivery`.

Latest full batch `archive-delivery.log`: **260PASS/1FAIL/0ignored/1filtered**, exit101. Exact unchanged `output_readmission_chain` rerun: **1PASS**, exit0, `output-readmission-recheck.log`; **do not relabel the full batch PASS**. The failure was historical `source_changed` expected SourceChanged, observed Cancelled. First broad batch `archive-complete.log`:259PASS/2FAIL because missing stream fixture env; both `tar_stream_readmission_contract` cases passed after explicit env and also in latest full batch. Its output chain passed, illustrating instability.

| Target | PASS | FAIL | Ignored | Filtered |
| --- | ---: | ---: | ---: | ---: |
| lib | 42 | 0 | 0 | 0 |
| creation_plan_contract | 4 | 0 | 0 | 0 |
| creation_worker_contract | 1 | 0 | 0 | 0 |
| edit_interaction_worker_contract | 2 | 0 | 0 | 0 |
| encoding_contract | 4 | 0 | 0 | 0 |
| engine_contract | 14 | 0 | 0 | 1 |
| extraction_plan_contract | 5 | 0 | 0 | 0 |
| interaction_contract | 4 | 0 | 0 | 0 |
| job_execution_contract | 2 | 0 | 0 | 0 |
| metadata_wire_contract | 1 | 0 | 0 | 0 |
| output_readmission_chain | 0 | 1 | 0 | 0 |
| output_readmission_contract | 3 | 0 | 0 | 0 |
| product_read_cancellation | 10 | 0 | 0 | 0 |
| product_read_chain | 24 | 0 | 0 | 0 |
| product_read_consumer | 7 | 0 | 0 | 0 |
| prompt_display_contract | 2 | 0 | 0 | 0 |
| rar_readmission_contract | 1 | 0 | 0 | 0 |
| sevenz_edit_safety_contract | 4 | 0 | 0 | 0 |
| sevenz_edit_worker_contract | 1 | 0 | 0 | 0 |
| sevenz_readmission_contract | 1 | 0 | 0 | 0 |
| source_deletion_contract | 9 | 0 | 0 | 0 |
| stream_contract | 12 | 0 | 0 | 0 |
| tar_edit_eligibility_contract | 1 | 0 | 0 | 0 |
| tar_edit_matrix_contract | 8 | 0 | 0 | 0 |
| tar_edit_raw_header_contract | 4 | 0 | 0 | 0 |
| tar_edit_worker_contract | 3 | 0 | 0 | 0 |
| tar_stream_readmission_contract | 2 | 0 | 0 | 0 |
| tar_stream_readmission_guard | 1 | 0 | 0 | 0 |
| tar_time_contract | 6 | 0 | 0 | 0 |
| transaction_contract | 5 | 0 | 0 | 0 |
| tree_creation_worker_contract | 1 | 0 | 0 | 0 |
| worker_cancel_contract | 1 | 0 | 0 | 0 |
| worker_extraction_contract | 9 | 0 | 0 | 0 |
| worker_wire | 19 | 0 | 0 | 0 |
| xz_read_contract | 5 | 0 | 0 | 0 |
| zip_boundary_contract | 2 | 0 | 0 | 0 |
| zip_creation_worker_contract | 1 | 0 | 0 | 0 |
| zip_crypto_contract | 5 | 0 | 0 | 0 |
| zip_edit_contract | 10 | 0 | 0 | 0 |
| zip_edit_worker_contract | 1 | 0 | 0 | 0 |
| zip_methods_contract | 13 | 0 | 0 | 0 |
| zip_volume_contract | 5 | 0 | 0 | 0 |
| zstd_read_contract | 1 | 0 | 0 | 0 |
| zstd_safety_contract | 4 | 0 | 0 | 0 |

100k performance excluded explicitly; separate `rar_readmission_chain`, `sevenz_readmission_chain`, `tar_stream_readmission_chain` not selected. No broad resource/native/100k claim. TAR-AC01 four worker tests still run **20 raw combinations** with independent CPython/bsdtar validation; the admitted TAR raw engine file was unchanged.

## C# effects → exact LIST → identical RUN

All35 reviewed Fixture names below were individually invoked as `NxTools.PlatformHost.Tests.exe --fixtures --list "<exact name>"` then `NxTools.PlatformHost.Tests.exe --fixtures "<same exact name>"`. Every LIST exit0, selected exactly1matching name. First execution evidence `native-final/01..35-{list,run}.log`, exact script `run-native.ps1`. Rechecks only rows4/28 with identical LIST/RUN in `native-recheck/1..2-{list,run}.log`; no test or native source change. **First: selected35/run35/pass33/skip1/fail1/not_run0. Recheck:2/2/2/0/0/0. Latest per-name35PASS/0SKIP, not a single all-green batch.**

| # | Exact name | First RUN | Same-name recheck |
| ---: | --- | --- | --- |
| 1 | archive edit interaction native ZIP Ask decisions single publish | PASS | — |
| 2 | archive tar edit wire shapes capabilities | PASS | — |
| 3 | archive tar edit native actions metadata empty | PASS | — |
| 4 | archive tar edit native source cancel handoff | **FAIL** | PASS |
| 5 | archive tar edit native failure unknown late cancel | PASS | — |
| 6 | archive tar edit native conflict injected lowspace | PASS | — |
| 7 | archive sevenz edit wire ordinary replace accepted | PASS | — |
| 8 | archive sevenz edit wire strict shapes ZIP preserved | PASS | — |
| 9 | archive sevenz edit wire admission snapshot capabilities | PASS | — |
| 10 | archive sevenz edit native actions subtree all delete | PASS | — |
| 11 | archive sevenz edit native AES header replace conflict | PASS | — |
| 12 | archive sevenz edit native source cancel handoff unknown | PASS | — |
| 13 | archive sevenz edit native empty subtree implicit unicode | PASS | — |
| 14 | archive sevenz edit native independent metadata retention | PASS | — |
| 15 | archive sevenz edit native conflict skip native terminal | PASS | — |
| 16 | archive Phase E delivery ZIP edit all actions stale source | PASS | — |
| 17 | archive ZIP edit native held source same path commit | PASS | — |
| 18 | archive ZIP edit native same path cancellation truth | PASS | — |
| 19 | archive ZIP edit native changed revision refusal | PASS | — |
| 20 | archive ZIP edit native handoff gates and ownership | PASS | — |
| 21 | archive ZIP edit native handoff failure unknown no retry | PASS | — |
| 22 | archive job fixture strict wire picker cancellation | PASS | — |
| 23 | archive job fixture actual conflict global admission cancel | PASS | — |
| 24 | archive job fixture actual cancel after publication native facts | PASS | — |
| 25 | archive job fixture actual password diagnostics native failure | PASS | — |
| 26 | archive job fixture actual risk pause prompt timeout | PASS | — |
| 27 | archive conflict fixture actual ZIP decisions | PASS | — |
| 28 | archive conflict fixture actual 7z RAR TAR decisions | **SKIP** | PASS |
| 29 | archive conflict fixture actual standalone stream decisions | PASS | — |
| 30 | archive output fixture native failure diagnostics privacy | PASS | — |
| 31 | archive output fixture rechecked atomic path overwrite | PASS | — |
| 32 | archive preview fixture strict readonly purpose | PASS | — |
| 33 | archive preview fixture actual output budgets | PASS | — |
| 34 | archive temp fixture live child browser release prompt return | PASS | — |
| 35 | archive temp fixture disconnect cancellation drain | PASS | — |

Row4 first failure: worker/output diagnostics cancelled/partial_removed, zero publication, completed cleanup, but Job terminal `failed/io_failure` instead of cancelled at final held-source cancellation. Owned evidence `C:\Users\Administrator\AppData\Local\Temp\archive-product-c2188708-5348-4e63-9b9d-f2d942215042\evidence\2.terminal.json` and diagnostics retained. Exact unchanged rerun PASS is not root-cause repair. Preserve as cancellation terminal instability; no native/source lease changes in Step4. Row28 skip missing `NXTOOLS_ARCHIVE_CODEC_FIXTURES`; recheck explicitly used repository `tests/fixtures/upstream` RAR samples and passed. No skip counted as pass.

Row1 exercises actual original browser ACK/Job/conflict/native publish for four decisions, independent .NET ZIP exact entries/content; Skip/Cancel preserve original source bytes/0commit, overwrite/autorename one native journal entry. Rust fixture additionally checks stale/duplicate ticket and independent CPython testzip/exact manifest. Native5/64/latecancel/injected112 are existing owned test seams, not real disk-fill/network/power-loss proof.

## Static/source gates

- `python tests/architecture/spec-contracts.py`:117documents/14owners, exit0 (rerun after final documentation/checkpoint required).
- `python tests/architecture/gpui-component-catalog.py`:79historical pages/rows,78historical entries, current0.8.0 rules, exit0.
- `python tests/architecture/gpui-release-contracts.py`: coherent0.8.0/pre0.3.9/exactlytwo approved framework deltas, exit0.
- `python tests/architecture/markdown-editor-contracts.py`:57original files and current approved pinned-editor deltas, exit0; no editor/vendor edits by this task.
- `python tests/architecture/archive-tree-root-drag.py`:3PASS, exit0.
- `powershell -NoProfile -ExecutionPolicy Bypass -File tests/architecture/platformhost-test-selection.ps1`:exit0.
- References PeaZip clean pinned commit/file hashes rechecked; source-only gates do not certify physical/native visual/user acceptance.

## Unexecuted / retained limits

No physical external drop, real Explorer/DPI/IME/UIA/theme screenshot/user acceptance, product launch, package/sign/install, Registry/service/ACL/UserChoice/Recycle or real user archive operations. Synthetic rendered controls/long-CJK/minimum bounds/focus/keyboard do not certify actual GUI usability. Dates remain unknown because no date facts were introduced. Add policy remains Ask/per-prompt choices, not a persisted policy selector. Unknown/unclean terminal does not auto-retry. Historical source_unavailable/CLI helper_untrusted/FB-AC05/ZIP readmission instability remain open; current first-run TAR cancellation projection instability is also retained. No Step5/HZ3,Step6/HZ4,Step7/HZ5 or whole phase completion.
