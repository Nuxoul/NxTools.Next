# B3 — Split-file usability

## Goal and scope

Deliver REF-17 and REF-18 in the approved serial reference-improvement pipeline after [B2 coordinator acceptance](../11-10-reference-b2-intake-control/coordinator-acceptance.md). Users can specify precise split-archive sizes and explicitly join ordinary numbered file parts without confusing joining with archive decoding.

Archive parent remains **Phase E — Format expansion and ZIP editing / in_progress**. B3 is a delivery batch, not a new canonical phase. [Taskbook](../../../docs/reference-improvement-taskbook.md), [archive parent](../10-16-archive-manager/prd.md), its [checkpoint](../10-16-archive-manager/phase-a-checkpoint.md) and [current workflow plan](../11-02-archive-workflow-optimization/prd.md) retain their existing boundaries. B3 remains not_accepted until coordinator review.

## REF-17 — Split-size presets and explicit units

- Extend the original create form with familiar labelled presets and a positive-integer custom value with explicit KiB/MiB/GiB. Checked normalization produces exact bytes once. Preset selection, unit changes and draft reopen must not silently round, reinterpret or change an applied byte value.
- Existing integer-MiB settings and existing entrances retain their exact legacy meaning when new fields are absent. Opening the form does not rewrite configuration. Save only owned changed leaves through the original section transaction; stale whole-section snapshots must not overwrite concurrent changes.
- ZIP disk-coordinate volumes and continuous7z segmentation remain separate existing writer operations, not arbitrary file chopping. Original format/level/method/encryption/solid compatibility and positive-size/capacity/family-count limits remain authoritative.
- Format changes and final Submit revalidate capabilities. Unsupported choices have an actionable reason, not silent unit/format changes or relaxed validation.
- Keep fresh-target/no-overwrite, validation of all candidates before publication, required CRC/auth/readback/source checks, truthful per-volume publication and actual cleanup. Do not claim whole-family atomicity or erase a published result on late cancellation.
- Preserve original stable InputStates, official Form/Field/number/select slots, approved variants/parent adaptations, focus/IME, scroll/footer, native geometry/placement and cancellation.

## REF-18 — Explicit ordinary numbered-file join

- Provide an explicitly named raw join entry under an existing owner. It is ordered byte concatenation, not extraction, repair or format conversion. Do not automatically hijack signature-bound ZIP/7z opening or treat classic `.z01` structures as raw `.001` streams.
- Accept a member from a same-directory `.001/.002/...` family, including a later part. Before output, preview discovered ordered names/count and destination, and require explicit confirmation.
- Reject detectable missing middle parts, duplicate/ambiguous ordinals and invalid family names before output. Without a format manifest, generic raw joining cannot establish whether an unknown final part is missing; show the discovered set without claiming completeness.
- Retain exact bounded same-directory family/source protections, checked total length and finite-buffer streaming. Do not retain whole files/families in memory or add part/output hashes, a second content scan or unused fingerprints.
- Output is fresh no-replace and never overwrites a source part or existing output; source parts are never implicitly deleted. Failure/cancel cleans only this attempt's incomplete owned output, retaining successful publication and actual native/cleanup errors. Resource release follows actual worker/handle completion.
- Long I/O must not block GPUI, the serial platform-command reader or shared Windows event thread. Original Shell/clipboard/hotkey/other commands remain responsive. Reuse original bounded execution/cancellation facilities, not an unbounded alternate runtime.
- Rust owns state/layout, GPUI renders, C# owns Windows handles/APIs. A genuine new operation entity owns only its own operation state; it is not a competing manager, transport or store. Any Other Tools entrance uses the approved official ListItem cell/tile shape.

## Shared requirements

Preserve accepted B1/B2, original archive create/edit/extract behavior, server information, icons, input-Link, parallel ZIP extraction and the entire unrelated dirty tree. Keep current Kit/Base/Component/Assets0.8.0 and default pre0.3.9. Preserve original weak-owner/window/generation/focus/IME/cancel/disconnect boundaries; defer cross-entity work beyond the current GPUI lease, including cache hits.

Each new user/cross-process operation defines accepted/start, meaningful state transitions, terminal outcome, elapsed time and native codes under one correlation ID. C# owns native-code facts. Diagnostics are bounded/nonblocking and coalesce byte progress; never log paths/names/contents/clipboard/URL/headers/cookies/credentials/raw errors. Confirmation UI may display user-requested names/paths.

## Acceptance

### REF-17

1. Production GPUI input, preset/unit/format switching, Submit and save/reopen preserve exact intended bytes and stable inputs. Exercise unit boundaries, overflow, legacy-MiB defaults, changed-leaf concurrency and invalid supported-format combinations.
2. Real original worker/native ZIP and7z split creation reaches independent project-approved readers/oracles and extracts exact source bytes. Cover existing-family refusal, maximum count, cancellation and partial publication while retaining source and cleanup facts.

### REF-18

1. Actual GUID-owned local Windows file fixtures prove byte-exact streaming join and later-part selection with numeric ordering. Reject missing middle/ambiguous ordinals, source changes and output conflicts; exercise write failure, cancellation and cleanup without executing output.
2. Data exceeds the streaming buffer; observe actual read/write blocks and bounded retained memory. Verify the ordinary command reader stays responsive and original confirmation/progress/window close/cancel lifecycle remains correct until actual work/handle cleanup.

### Evidence and delivery

- Obtain behavioral RED before each corresponding minimal GREEN through the approved production input/setting, writer/worker/native, file-stream/command-reader and GPUI lifecycle seams. Imports, DTO-only tests, self-writer/self-reader, source tokens and screenshots do not substitute for these boundaries.
- LIST exact fixture selections/effects before RUN using existing reviewed harnesses and allowed independent readers. Report actual RUN/PASS/FAIL/SKIP/not_run, command/environment, counts and absolute log paths. No missing oracle or skipped native action is a pass.
- Use one Cargo lane, inspect live cargo/rustc/link before starting, and set jobs1/incremental0/codegen-units256/scratch TEMP/TMP in each Rust process. Wait for actual owned processes/resources to finish before handoff.
- Report scoped source/TestPlatform/deterministic/native-owned evidence separately from physical/IME/DPI/provider/human visual/installed/full-workspace acceptance. Keep known unrelated message8/9, Desktop lint/source-token and unavailable historical native-export baselines attributed.

## Exclusions and permission boundary

No REF19ALZ/EGG, REF20SFX, RAR/ZIPX creation, split-set editing, later download batches, extraction performance experiments, dependency/vendor/theme upgrades, GPL/proprietary code/assets, unnecessary hashes, broad refactoring or repository-wide formatting. No actual product/reference launch, package/install/deploy replacement, extension/association registration, registry/service/power operation, commit or push. Release packaging requires separate authorization and its existing preflight. Do not edit coordinator-owned `docs/reference-improvement-coordination.md` or recursively delegate implementation. Coordinator owns B3 acceptance and any following batch.
