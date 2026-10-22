# Standalone stream conflicts — independent scoped acceptance

2026-10-21. **PASS for standalone Gzip/XZ/BZip2/Zstd extraction conflict integration.** Reviewed
[evidence](stream-conflict-evidence.md), [source boundary](research/stream-conflict-sources.md),
four format extraction functions, worker admission and actual stream conflict fixture. No concrete
in-scope blocker found. No implementation/test changes were made by this acceptance.

## Accepted behavior

Four formats reuse existing begin_extracted_file, shared InteractionPort/ExtractionConflict,
original target observation and native publication. Skip still calls the SAME full stream decoder
to sink, with actual budgets/end/applicable integrity and source revision checks. It creates no
file partial/write/metadata/commit/abort, returns zero published files/bytes, reports skipped1 and
warnings/members_skipped. Corruption or budget failure remains failure, not successful Skip.
Present-token behavior retains original verified publication and exact owned-token abort on error.
No new framework/protocol/native runtime, dependency or C#production publication change.

Actual seven-scenario fixture covers Gzip AskOverwrite, XZ AutoRename, BZip2 Skip, Zstd Overwrite,
Gzip waitingCancel, skipped-Gzip CRC corruption and skipped-BZip2 actual quota failure. Original
runner/Job/actualworker/native asserts literal276bytes for published results, oldtarget/foreignpartial
preservation, counts/summary/source qualification, no writes on Skip, source unchanged/exclusive
reopen and exited/drain/slot0. Recording recycle backend is never called. No real source deletion.
Existing format/native cases supply numbered race/tickets/target-change/publication/isolation checks,
not four copies of every asynchronous combination.

## Independent verification actually performed

One foreground serial batch; isolated C#apphost first, matchingisolatedworker rebuilt after lint
before native execution. Cargo inherited jobs1/incremental0/RUSTFLAGS=-C codegen-units=256; machine
lane checked. Explicit fixture/Python/stock7Zip paths configured, no environment skip.

| Check | Actual result |
| --- | --- |
| C#Debug build |exit0,0warnings/errors |
| Seven affected Rust targets |48pass/0fail/ignored,exit0 |
| Strict Clippy isolated/default tests/examples |both exit0,-Dwarnings |
| Archive scoped fmt /matchingworker build |both exit0 |
| Exact C#LIST/RUN/PASS |same5names;selected5/run5/pass5/skip0/fail0/not_run0,exits0 |
| Spec/context/fixtureselector/diff |all exit0 |
| Identity/sourcehash assertions |PASS;63currentbaselinefiles unchanged during checks |

No globalworkspace test/compile/format claim. Existing native-contracts context-size warning remains.
No independent claim to repeat implementer's entire128historicalfile comparison; current63file
baseline and exact runtime identities/counts verified. Submitted reports/fixture bytes unmodified.

## Coverage and evidence

Together with [ZIP](zip-conflict-acceptance.md) and [7z/RAR/TAR](format-conflict-acceptance.md)
independent acceptances, ordinary-file conflict handling now connects ALL currently supported
extraction formats. TAR compressed wrappers retain shared-path evidence with TGZ as native
representative; no format×policy×failure exhaustive matrix is claimed. There is no remaining
standalone-stream conflict implementation gap from this slice. Other interaction/phase/native
requirements remain separate; this is not complete B2/Phase A/A–F acceptance.

Scratch C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df:
stream-conflict-independent-verify.ps1, ledger/per-step logs, baseline and assertionsJSON preserve
commands/exits/counts/exactnames. Closing0archiveworkers/testhosts,0testpipes,fixedhelperabsent.
No extra review/test matrix,100k/largeinput/longpathmatrix/performance,UI/FIFO,sourceRecycle/delete,
systemchange,formalapp,package/install/sign/commit/push. **Accepted; stop this slice.**
