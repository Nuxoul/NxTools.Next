# EIC-AC01 / EIC-AC02 bounded repair route

2026-10-29. Read independent [acceptance](edit-interaction-acceptance.md) and current checkpoint first. Only these two concrete Step4 blockers, still **Phase E — Format expansion and ZIP editing**. No HZ3–HZ5, broad review/subagent, engine/native lease/commit rewrite, package/install/product/Shell/system/user archives or commit/push.

Opening whole checkpoint186377bytes SHA256838C846EC529934425386A110359A7ECEB37DC3AC74894627B3AD67FCE288CA4 saved under session scratch `eic-ac01-ac02/checkpoint-before.bin`; exact opening source/protected/history hashes/status/HEAD saved. Only prepend at end, re-read tag and prove entire opening bytes exact suffix. Preserve original Step4/TAR/AC01/negative reports and independent acceptance.

## Smallest gaps / owning files

- AC01: original ZIP adapter only checks an explicit directory entry. A preexisting descendant already establishes an implicit directory; merging that incoming directory is no change, and all colliding child files Skip must mean no candidate/begin/publication. `worker_zip_edit.rs::additions` alone should recognize that original logical directory (exact directory or slash-boundary descendant), but retain exact file/type collisions and truly new empty directories/real selected files. Must not consult future incoming names as evidence that a directory already exists in ORIGINAL archive. Same case-insensitive collision policy, finite work/cancellation/metadata bounds, original candidate/readback/FILE-only singlepublish.
- AC02: `worker_output.rs::call_payload` currently maps native error_code=cancelled to IoFailure. Map the exact native failure semantic to ArchiveError::Cancelled at the actual reply boundary, without looking at whether a cancel request was sent. C# Job can keep original precedence: native publication/unknown/sourcechange/cleanup/timeout outrank worker details. No blanket cancellation override, public DTO/native authority/permission changes.

## RED→GREEN seams

New actual worker contract + independent tiny fixture: original ZIP has implicit/explicit directory, local protected folder child collision Ask/Skip; zero selected file opens, candidate begin/read/write/commit zero, warning and exact source hash. Controls retain new empty directories and true non-skipped files, distinguish `folder/` from `folder-other/`, file-kind conflicts. Independent zipfile names/content/testzip, same original tickets.

Deterministic native reply-only worker test: TAR/ZIP/7z candidate/readback/oldreaderclose then exactly one commit native reply success=false/error_code=cancelled, no cancel control frame -> failure/Cancelled, source hash unchanged. Adjacent source_changed/target_exists/native-failure strings retain existing mapping. Original parent scripts are copied/replayed without changing their assertions; negative snapshots remain immutable.

C# exact audited fixtures: implicit+explicit directory allSkip through original Browse ACK/Job/native output -> succeeded_with_warnings/files_skipped1/files_committed0/no publication/before_flush checkpoint/source exact; new empty dir and real file controls independently decode. Deterministic cancelled at original held-source native checkpoint for ZIP/TAR/7z plus existing real control-cancel/handoff/native5/64/latecancel tests ensure final Job cancelled while no overriding real native facts. No new test seam if original commit probe suffices.

## Diagnostics and checks

No new operation or diagnostics subsystem; same correlated original accepted/start/state/terminal/elapsed/native-code events. Exact fixed cancellation category travels original private reply; no paths/member/content/password/request Debug/chunk logs. No temporary production probes.

Every Cargo checks idle lane then same-process jobs1/incremental0/RUSTFLAGS codegen-units256, locked/offline/serial/longtimeout. Run actual new tests RED before product edits, GREEN after; affected Archive strict lint, workspace, current isolated worker build/freeze/hash, focused existing ZIP/7z/TAR/read/output/raw AC01 contracts and Desktop archive332 preservation as appropriate. C# build effects audit->exactLIST->sameRUN with current hash-frozen worker, zero skips required for claimed passes. Original broader acceptance83/Archive261/native35 is historical independent evidence, not this repair's execution counts. No100k/fullnative. Finish independent repair outcomes/exact commands/counts/hashes/drift, checkpoint prefix/suffix proof, stop for coordinator narrow acceptance.
