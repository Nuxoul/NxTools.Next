# Planning convergence and authority record

2026-10-25. This is documentation/research only; product implementation is NOT started. User confirmed all seven in scope, HTTP declared public/loopback/private endpoints, and storage upgrade-preserve/uninstall-retain/explicit-clear/same-ID-explicit-restore. Final proposed MVP/defaults/architecture and P0–P8 execution order await a fresh explicit approval of the final summary.

## Requirement convergence pass
PRD intentionally rewritten once into Goal/UserValue, Authority/Baseline, R0–R8 with A0–A8, approved product behavior, proposed first-version scope, safety/exclusions and completion/acceptance levels. Temporary open-question/repeated brainstorm sections removed; resolved choices kept in authoritative policy section. Original exact source file/line anchors remain in research/current-owners.md; no findings or cleanup/trust limits discarded. Blocking open product questions empty; final proposed quotas/serial W10/PNG-only/nonstreamingHTTP still subject to final-plan approval, not assumed from prior narrow answers.

Complex-task artifacts exist: prd.md,design.md,implement.md plus curated implement/check JSONL. Seven serial deliverables P1–P7 map to seven capabilities, parent P0/P8 owns shared protocol and finalintegration. No child task or implementation dispatch created before approval. Native UI authorization remains false; install/package/service/realuserdata/externalnetwork/commit/delegation authority not granted.

## Self-check corrections before final review
- Preserved previously valid duplicate legacy string choices/hash; unique values required only for new object choices.
- Separated declaration/storage validity from required-field readiness: required-empty imports but cannotinvoke until configured, no fabricated default.
- v1 budgets not enlarged. Explicit v2 separate binary budgets, one reader/bounded8call+4transfer multiplexer; per-chunk64KiB ACK does not consume64host_call total.
- Blob8MiB/object,16MiBlive/run allows input and output coexist,64MiBglobal; decoder64MiBRGBA with128MiBreserved bounded peak/canonicalencoded sink, no implicit thumbnail.
- Retained storage generations count quotas, metadata pre-reserved so removal/disable not blocked by allocating orphan entry; no silent eviction/secureerase, reconcile ambiguous metadata conservatively.
- Original index session holds sharedservice mutex and one supersedingSearchExecutor; no secondpipe workaround. Proposed scoped backward-compatible secondlane uses immutable snapshots/finiteworkers/originalreader/auth, no MFT/USN/cache/maintenance rewrite.
- W10 separate process/request reservations, lazy2instances/serialpackagecall/idle120s/perrequest deadlines/lifetimebudget reserve, unresponsivesession killed only after logicalretirement and actualcleanup retained, never replay effects.

## Executed documentation checks
- task.py validate: implement10entries/check8entries valid. Initial fullQuickLaunch reference entry warned138112bytes>32768 contextinjection; replaced with short Protocol quality spec and bounded currentowner research; full reference remains required manual reading.
- spec-contracts:PASS114documents/14workspaceowners; navigation/source/evidence structure only.
- Local task check: unique JSON, statusplanning, implementation/final-summary/nativeUI approvalfalse, HTTP/storagepolicy recorded, no blocking decision, R0–R8/A0–A8 and P0–P8 present, fourconverged docs below32768bytes, UTF8/7local links/actualJSONL targets valid, no duplicateparagraph/stalebudgets/TBD/TODO.
- Scoped gitdiff--check PASS. Final warning-free task/context rerun recorded in tool output.

No Cargo/dotnet/product/referenceprocess/network/nativeGUI/capture/userconfig/Clipboard/credentials/indexerservice/build/package/install/commit/push/archive/delegation executed. Prior1196fullDesktop is unchanged historical evidence; not these capabilities' acceptance. Do not mark sourceimplemented or waive future native/service gates from planning checks.
