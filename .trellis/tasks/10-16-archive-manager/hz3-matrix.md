# Step5 HZ3 — exact verification matrix

2026-10-29, **Phase E — Format expansion and ZIP editing**. Fifth item only; implementation evidence for coordinator independent acceptance, not user/physical/release acceptance. [Route](hz3-route.md), [outcomes](hz3-outcomes.md), [owner executable contract](../../spec/nxtools-desktop/backend/archive-batches.md). No HZ4/HZ5.

Evidence root `$H`: `C:\Users\Administrator\.pi-desktop\scratch\0c0e5320-11c4-40dd-bb50-f6a437188fc1\hz3`. Raw logs retained; PowerShell logs may be UTF16BOM. `test-counts.json` reads BOM, `command-ledger.json` retains expanded final commands/exits, `native-final/results.json` exact LIST/RUN accounting, `opening-drift.json`, `delivery-source-hashes.json`, `identities-utf8.json` and final evidence ledger retain provenance. Normal incremental build, not clean build.

## Required behavior × original owner × evidence

| Behavior | Boundary / verdict |
| --- | --- |
| Three members, two original Starts, one queued; separate batch/logical/attempt IDs | original submit_batch/JobQueue/PlatformBridge production regression PASS; scheduler2/32/2GiB/FIFO/head-overlap unchanged |
| Waiting/pause/queuedcancel/warning/failure/partial publish/cleanupunknown/unrelated batch | original records projected; explicit state/fact assertions PASS; cleanupfalse retains original slot, unrelated batch uses remaining slot |
| Admission is not Started | independent ever_started fact; queued/admitting/running distinct; neverstarted includes queue cancellation; unknown percentage absent |
| All cancelled not success; bounded relationship retention | production owner allcancelled/64 completed-group regression PASS; active/cleanupheld/retrypending/pendingconfirmation excluded from retirement |
| Duplicate/stale terminal and explicit retry | original current operation/session/generation guards; same logical member, new attempt/generation; old event and duplicate recheck do not redispatch PASS |
| Finite readmission not a new member/terminal | existing original real coordinator test gains batch assertions; exact one member and one succeeded logical terminal after second native attempt PASS |
| Structural/memory/capacity batch refusal and defer reset | original validate_batch/queue, invalid member/33requests/next valid batch PASS; no task on structural batch rejection |
| Overlap/aggregate/duplicate/FIFO | original queue tests retained within full Desktop342 PASS; no scheduler policy change |
| Explicit safe retry | single-source Test/unselected Extract, failed/rejected knownprecommit/zero publish/cleanuptrue + native preterminal identity baseline; NEW observation compares original source_set+target; native execute re-admits; legal newattempt PASS |
| Source/target change, unknown/unclean/published/missing qualification | failclosed summary retry; app source_changed/unknown refusal and actual native expectedsource/target tests PASS; edit/create retain original form confirmation instead of generic replay |
| Multiple original forms | same-launch forms share identity; pending1 before second confirmation, submission-before-start cancel1 without extra logical job PASS |
| Whole-selection / split-volume dedup | launch helper consumes original native source sets, not basename/case heuristic; later-volume refusal only covered by same-selection validated family; same basename different directories stays separate PASS; actual C# held family projection PASS |
| Rendered original summary entry / details / disabled controls / minimum | production original JobProgressView at480x260 plus official Table, pointer and Tab/Space/no unintended Start PASS |
| Enabled retry pointer/repeated keyboard | select actual current-frame enabled Button among same-label siblings, one native recheck and one new attempt PASS; disabled sibling never authorizes replay |
| Result-location/controls/password/focus | original detail/control/location owners reused; original HZ1/HZ2/WAC/edit/preview/launch tests preserved in342; native authority/source/commit compatibility exact21 PASS |
| Light/Dark/long content | original minimum/theme/long-CJK progress/dialog tests retained; HZ3 summary uses original bounded scrollbody/footer and official theme tokens. No new physical screenshots/IME/UIA/visual acceptance claim |

## Exact new Desktop test names (10)

All included in full `archive` filter; existing332 +10 =342. Not a separate test engine.

1. `hz3_original_batch_exposes_rendered_summary_without_replacing_two_running_queue`
2. `hz3_retry_uses_original_native_recheck_once_before_new_attempt`
3. `hz3_projection_preserves_prompt_pause_queued_cancel_warning_partial_and_cleanup_slot`
4. `hz3_all_cancelled_is_not_success_and_retention_never_evicts_cleanup_held`
5. `hz3_rendered_summary_pointer_keyboard_details_disabled_retry_and_minimum`
6. `hz3_retry_refuses_changed_source_target_unknown_and_late_baseline`
7. `hz3_batch_invalid_capacity_memory_is_atomic_and_does_not_leave_deferred_pump`
8. `hz3_original_confirmation_forms_share_batch_and_cancel_is_never_started`
9. `hz3_rendered_eligible_retry_click_rechecks_once_then_original_new_attempt`
10. `hz3_native_source_sets_deduplicate_volumes_not_same_basename` (original launch helper)

Existing `phase_d_original_readmission_waits_cleanup_and_preserves_logical_once_terminal` retains original coordinator behavior and adds exact batch membership/terminal assertions. No additional test count is manufactured by adding assertions. The full342 run preceded that last test-only assertion addition; the same test was then rerun PASS1 and strict Clippy rerun after it. Product source did not change after full342.

Protocol new test: `hz3_v2_optional_source_set_is_bounded_legacy_and_request_bound`; full Protocol90 PASS. C# existing held-observation and volume-family fixtures gain v2 production-boundary assertions without weakening old v1/no-write/source/namespace checks.

## True RED/GREEN and retained nonbehavior failures

| Round | Actual evidence / meaning |
| --- | --- |
| `entry-red.log` | original production rendered entry missing:0PASS/1FAIL, true RED |
| `entry-first-green.log` | compile API/ElementId/visibility/missing legacy snapshot fields; not behavior RED/GREEN |
| `entry-green-v2.log` | original two Starts + rendered batch entry1PASS |
| `source-set-red.log` | rejected test insertion selected0tests/89filtered, not PASS/RED; retained |
| Protocol immediate rerun | shared Cargo lane busy, no command started; no duplicate build/name kill |
| `source-native-red-run.log` | exact original C# held-observation1FAIL on v2; true native read-only boundary RED |
| `source-native-green-run.log` | same exact fixture1PASS after original v2 projection |
| `hz3-core-first/v2.log` | local predicate bracket / missing trait compilation failures, not behavior RED |
| `hz3-core-v3.log` | initial2Desktop+1Protocol PASS |
| `lifecycle-first/diagnosis.log` | unbounded completed batch retention true failure; some invalid fixtures bypassed original IPC cleanup or dropped endpoint, causing not_submitted/early disconnect; failures retained and distinguished |
| `lifecycle-green-first.log` |3PASS/1fixture full-notification-channel failure; fix test consumption through original receiver, do not enlarge queues |
| `lifecycle-rendered-first.log` |7PASS/1missing summary accessible text failure; same real content receives original Label semantic wrapper |
| `rendered-green-v2.log` | missing StatefulInteractiveElement trait compile error; no fake content |
| `desktop-archive-first.log` |340PASS before confirmation/enabled-retry final coverage |
| `strict-first.log` |4local lint errors; fixed formatting/then_some/collapsed if, no suppression |
| `confirmation-first` / `hz3-final-slice.log` | bracket / module-visible helper compile errors; fixed local scope only |
| `hz3-confirmation-v2.log` |9Desktop+1Protocol PASS; strict-v3 PASS |
| `hz3-last-interactions.log` |9PASS/1enabled-retry test selected disabled same-label sibling; test hit-target ambiguity, not unsafe product retry; use current enabled semantic node |
| `eligible-retry-hit.log` | actual enabled Button pointer/repeated key -> one recheck/one new attempt1PASS |
| `desktop-final.log` | complete342PASS,0FAIL,0ignored,1167filtered |
| `finite-readmission-batch.log` | final original coordinator1PASS with added batch assertions; `strict-last-test.log` PASS after it |

Never count a compile error/zero selection/busy lane as product RED; never delete failed logs or call the history all-green.

## Final Rust / build commands and counts

Every Cargo operation checked idle cargo/rustc/clippy immediately before starting, same process jobs1/incremental0/RUSTFLAGS `-C codegen-units=256`, locked/offline, serial tests, long timeout. No background duplicate build/channel reservation. Scripts in `$H/verify-final.ps1` and exact argv JSON are authority.

| Exact command | Log | Actual result |
| --- | --- | --- |
| `cargo test -p nxtools-desktop --locked --offline --bin nxtools-desktop archive -- --test-threads=1` | desktop-final.log |342PASS/0FAIL/0ignored/1167filtered,exit0 |
| `cargo test -p nxtools-protocol -p nxtools-ipc --locked --offline -- --test-threads=1` | protocol-ipc-final.log |Protocol90+IPC43 PASS,0FAIL/ignored/filtered,exit0; doc-tests0 separate |
| `cargo test -p nxtools-core --locked --offline -- --test-threads=1` | core-compatibility.log |lib93+integration6+1=100PASS,0FAIL,exit0; no Core product changes |
| `cargo test -p nxtools-archive --features isolated-codecs --locked --offline --no-fail-fast --lib` plus10exacttargets below, `-- --test-threads=1` | archive-focused-final.log |98PASS/0FAIL/0ignored/0filtered,exit0 |
| `cargo clippy -p nxtools-desktop -p nxtools-protocol --locked --offline --tests -- -D warnings` | strict-final.log / strict-last-test.log |exit0, no suppression |
| `cargo check --workspace --tests --locked --offline` | workspace-final.log |exit0 |
| `cargo build -p nxtools-archive --features isolated-codecs --bin nxtools-archive-worker --locked --offline` | worker-build-final.log |exit0, current frozen worker6CD1…C0E1 |
| `dotnet build csharp\NxTools.PlatformHost.Tests\NxTools.PlatformHost.Tests.csproj --no-restore -m:1 -nr:false -p:UseSharedCompilation=false` | dotnet-final.log |exit0,0warnings/0errors |
| `rustfmt --check --edition 2024`15Rust files / `git diff --check --`19source paths | fmt.log/diff.log/source-checks.json |exit0 (final docs/checkpoint rechecked separately) |

Archive exact targets/pass counts: lib42; `edit_interaction_worker_contract`2; `eic_repair_worker_contract`4; `sevenz_edit_safety_contract`4; `sevenz_edit_worker_contract`1; `tar_edit_matrix_contract`8; `tar_edit_raw_header_contract`4 (20raw samples); `tar_edit_worker_contract`3; `worker_wire`19; `zip_edit_contract`10; `zip_edit_worker_contract`1. Expanded `--test` argv is `archive-focused-final.json`, not a claim to run full45targets.

No100k/resource/performance/fullnative. Original `output_readmission_chain`/other independent readmission chains were NOT selected this round. Prior coordinator80PASS/1FAIL SourceChanged-vs-Cancelled history remains OPEN, not repaired or rerun seeking green by HZ3.

## C# exact21 effects → LIST → identical RUN

All existing Fixture classifications reviewed against current effects: GUID-owned files/directories/native leases, original held observation and small verified candidates, exact owned children/injected original boundary probes, no product/GUI/Shell/system/user archives. Every item individually ran `NxTools.PlatformHost.Tests.exe --fixtures --list "<exact name>"` and then `... --fixtures "<same exact name>"`; every LIST selected1/exited0, RUN selected1/run1/pass1/skip0/fail0/not_run0. One complete sweep **21selected/21run/21PASS/0SKIP/0FAIL/0not_run**, `native-final/01..21-list/run.log` and results.json, script `native-final.ps1`.

1. archive space fixture held read-only observation
2. archive space fixture native allocation identity
3. archive space fixture missing suffix target binding
4. archive space fixture expected source target changes
5. archive space fixture capacity byte override
6. archive space fixture typed native open failures
7. archive space fixture native reparse refusal
8. archive space fixture cancellation publication
9. archive space fixture strict wire private diagnostics
10. archive space fixture original dispatcher retained bounded slots
11. archive space fixture detached cancellation no late receipt
12. archive Phase E source all-volume protection missing
13. archive Phase E source single ZIP exact disk count
14. archive EIC AC01 native directory merge zero publish
15. archive EIC AC02 native cancelled reply only ZIP TAR 7z
16. archive ZIP edit native handoff gates and ownership
17. archive ZIP edit native handoff failure unknown no retry
18. archive tar edit native source cancel handoff
19. archive sevenz edit native source cancel handoff unknown
20. archive preview fixture actual output budgets
21. archive job fixture actual conflict global admission cancel

Source-set volume fixture is native family/lease identification using tiny segmented existing encoded bytes; it is not new volume codec interoperability proof. Existing actual ZIP/TAR/7z fixtures continue original independently verified/native publication semantics. Injected failures/unknown/latecancel are approved tiny owned seams, not power-loss/network/real disk-fill proof. Current worker/actualtestDLL/oracle identities in outcomes and UTF8 manifest.

## Static / limits

Spec118documents/14owners, componentcatalog79historicalrows, GPUIrelease0.8.0/pre0.3.9/twoframeworkpatches, pinnededitor57files and test-selection gates exit0. Final spec/diff after reports/checkpoint still required; source gates do not certify screenshots/interaction/native release.

No physical mouse/external drop/Explorer/DPI/IME/UIA/LightDark screenshots/user/installed acceptance. HZ3 remains session-only/bounded, not persistent history. Summary generic retry is intentionally unavailable for create/edit/selected extraction/missing identity/unknown/published/unclean; use original form/reopen. Original source_unavailable/helper_untrusted/FB-AC05/readmission instability retained. Stop after fifth-step delivery, no HZ4/HZ5.
