# Step3 TAR-family exact evidence matrix

2026-10-28. Evidence root `C:\Users\Administrator\.pi-desktop\scratch\0c0e5320-11c4-40dd-bb50-f6a437188fc1\tar-edit`. **Phase E — Format expansion and ZIP editing**, implementation delivery pending coordinator independent acceptance. [Outcomes](tar-edit-outcomes.md), [contract](../../spec/nxtools-archive/backend/tar-edit.md). PASS below is scoped automated/owned-fixture evidence, not whole-product acceptance.

## Operation / fidelity / refusal

| Case | Actual boundary and exact test | Result / limitation |
| --- | --- | --- |
| Plain rename; corrupt held readback; all old readers closed | `tar_edit_real_worker_renames_and_closes_source_before_single_commit` | PASS; real child worker/native protocol fixture; share_mode0 exclusive reopen at edit_reads_complete; corrupt readback abort/zero commit |
| TGZ/TXZ same path and metadata | `tgz_edit_real_worker_preserves_content_metadata_and_source_handoff`, `txz_edit_real_worker_preserves_content_metadata_and_source_handoff` |2PASS; independent CPython content+uid/gid/mode/mtime |
| Add files/tree/empty directory/zero file; ReplaceContent; NewDirectory | `tar_edit_add_tree_empty_file_directory_new_directory_and_replace` |PASS loops3formats, protected source-ID reads/EOF/close; Python exact output |
| Delete overlap/subtree and all | `tar_edit_delete_overlap_subtree_boundary_and_legal_empty_oracle` |PASS loops3formats; a/ vs ab/; legal empty output independently read |
| Explicit/implicit/empty folder and Unicode rename | `tar_edit_explicit_implicit_empty_directory_rename_and_unicode` |PASS original ordinals/prefix, no invented implicit records |
| PAX precision/uid/gid/uname/gname and GNU long paths | `tar_edit_pax_precision_identity_and_gnu_long_path_oracle` |PASS exact PAX mtime/atime/ctime strings/precision and identity; independent content |
| Duplicate/case/file-prefix/unsafe/link/unknown local/global/tail | `tar_edit_unsafe_ambiguous_lossy_and_outer_shapes_refuse_before_native_candidate` |PASS refusal before native begin; not full POSIX metadata support |
| Outer garbage/truncation/multi-member/stream | same test |PASS gzip/xz refusal; ordinary multi-stream reads remain separately tested |
| Source forged folder prefix / implicit-parent case alias | `tar_edit::tests::tar_edit_prefix_cannot_promote_file_to_directory_and_implicit_case_is_ambiguous` |PASS after1behaviorRED; no file-kind promotion/partial subtree change |
| Conflict overwrite/skip/auto-rename | `tar_edit_conflict_overwrite_skip_autorename_are_one_transaction` |PASS; all-skipped begin0/commit0 |
| Write/readback/source-check/EOF/extra/cancel/limit/commit replies | `tar_edit_write_read_source_eof_extra_cancel_commit_failures_keep_source` |PASS; worker protocol injection, not native publication proof |
| Actual edit capability versus suffix/readability | `tar_edit_actual_source_qualification_is_separate_from_readability_and_suffix` |PASS; links still readable but not editable; TBZ2/TZST/streams missing backend |
| Plain TAR finish and flush errors | `tar_edit::tests::tar_edit_plain_finish_and_flush_errors_are_terminal` |PASS; test writer at production end+flush function |
| Gzip/XZ finish failure after input EOF | `tar_edit_outer::tests::tar_edit_gzip_encoder_finish_error_is_terminal`, `tar_edit_outer::tests::tar_edit_xz_encoder_finish_error_is_terminal` |2PASS; actual production edit encoder finish hits injected storage error |
| Gzip/XZ final flush failures | `tar_edit_outer::tests::tar_edit_gzip_final_flush_error_is_terminal`, `tar_edit_outer::tests::tar_edit_xz_final_flush_error_is_terminal` |2PASS; explicit ignore-flush mutation made both FAIL; restored exact SHA before final build |
| Gzip meaningful fields; XZ check0/1/4/10; signature mismatch | `tar_edit_outer::tests::tar_edit_outer_meaningful_properties_signature_and_independent_decode` |PASS; independent Python compressed seeds/decode; mtime/OS/name/comment retained; parameters disclosed |
| Original Browser controls/ordinal/missing facts | `archive::browser::tests::tar_edit_worker_facts_enable_original_actions_and_missing_facts_refuse` |PASS in321Desktop archive set; worker facts not suffix; Comment denied |
| Original Form/prefix/cost/missing facts | `archive::forms::tests::tar_edit_original_form_discloses_cost_and_keeps_one_replacement_and_prefix` |PASS in321; actual Form/Alert bounds; physical/visual/user approval unexecuted |

## Actual C# Job / native publication — exact final LIST and identical RUN

`NxTools.PlatformHost.Tests.exe --fixtures --list 'archive tar edit'` then same without `--list`: **5selected/5RUN/5PASS/0SKIP/0FAIL/0not_run**, exit0. `native-complete-list.log`, `native-complete-run.log/json`; owned roots recorded in `native-complete-evidence/roots.txt`.

| Exact test name | Actual scenarios |
| --- | --- |
| archive tar edit wire shapes capabilities | TAR/TGZ/TXZ strict actions, bounds/negative formats/comment, named capability fail-closed |
| archive tar edit native actions metadata empty | Independent PAX seeds3formats -> browser original ordinals -> explicit+implicit subtree rename/Unicode -> replace/add/newdir -> all-delete. CPython exact content/uid/gid/mode/PAX time; bsdtar exact set; one same-path native replacement journal per edit |
| archive tar edit native source cancel handoff | writable-source conflict, stale revision, final held-source cancel, source FILE write denied and parent move denied before release, original retained/cleanup |
| archive tar edit native failure unknown late cancel | actual native boundary injected5/64 once/no retry; unknown candidate retained; TXZ late cancel after native success remains Published=true |
| archive tar edit native conflict injected lowspace | original conflict ticket skip warning/commit0, auto-rename, overwrite; injected disk-full112 at original before_flush -> zero commit/source exact/owned cleanup. **Not physical disk-fill acceptance** |

## Preserved regression and aggregate accounting

- Archive final consolidated42targets: **253PASS/1FAIL**, ignored0, one100kperformance filter. Failure: `real_rust_coordinator_reenters_original_native_admission_only_after_cleanup`, source_changed scenario observed Cancelled; same-name bounded rerun1PASS, no product/test assertion fix. Original unified failure remains open/flaky evidence, not overwritten with rerun success. `archive-target-counts.json` lists every target/count/log; initial missing NXTOOLS_TAR_STREAM_FIXTURES2FAIL and OOM are separate histories.
- Archive lib42 includes new7TAR unit tests; new integration11(eligibility1+matrix7+worker3). Original ZIP edit10/crypto5/method13/ZIP64/unknown-delete and7z edit5 remain passed. Three standalone rar/sevenz/tar-stream coordinator-chain targets not run; 100k test not run.
- Desktop archive **321PASS**,0FAIL/ignored, includes new2TAR. Original HZ1 18, icon-layout8, WAC12 retained; exact full names in `desktop-archive-recovered.log`. OOM codegen histories (including no-debug-symbol attempt) are not passes; after resource recovery same original command passed.
- Core93unit+7integration, IPC43, Protocol89PASS. Strict5owner Clippy `--tests -D warnings` and workspace `check --workspace --tests` pass.
- C# compatibility42names individually exactLIST/RUN: first35PASS/6SKIP/1FAIL. Corrected non-ASCII oracle script configuration, exact6SevenZnative rerun6PASS/0SKIP. Consolidated latest per-name: **41PASS/1FAIL/0SKIP**, historical skip logs retained. One unchanged known `archive job fixture actual readmission shortfall no output` FAIL source_unavailable. Includes ZIP5, SevenZ9, space11, preview2, original OLE/retained streams/async/lease and empty-directory location/control. Not all native tests or entire previous56set.

## Explicit unexecuted / bounded claims

No exhaustive large archive/memory/disk-fill stress, native crash/restart/power-loss/HZ4, source space reclamation/HZ5, physical Explorer/OLE receiver/user-pointer/IME/DPI/UIA reader/Light-Dark visual/installed-release acceptance. No package/sign/install/system mutation/commit/push. Unsupported special headers/unknown extensions are refused, not supported or silently converted. TAR transactional SHA256 is not payload checksum. Stop at Step3; no Step4/HZ3/HZ4/HZ5.
