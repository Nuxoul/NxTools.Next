# B2 ZIP extraction conflicts — independent scoped acceptance

2026-10-20. **PASS for the delivered ZIP-only conflict slice.** No concrete in-scope blocker
found. Reviewed [evidence](zip-conflict-evidence.md), [results](zip-conflict-results.json),
[source boundary](research/zip-conflict-sources.md), worker conflict/interaction/output and ZIP
extraction code, C#target observation/runner/terminal projection and actual conflict fixture.
This does not complete other-format integration, full B2, Phase A or A–F.

## Accepted behavior

- Ask uses installed original InteractionPort tickets, not a preconfirmed-map substitute.
  Rust owns fixed decisions and one task-local apply-all; C# forwards typed answers only.
- Same output session saves one target observation token/path/full identity-version-hash signature.
  Overwrite consumes matching observation and uses original BeginFile and finalcommit rechecks.
  Waiting target changes return target_changed with externalbytes retained. Approved recheck-close
  to single atomic path publication TOCTOU remains unchanged; no targetID-CAS/delete-first/fallback.
- Skip creates no filepartial, preserves oldbytes and reports exactskipcount. Worker summary is
  sent after extraction returns, including error results; C#retains counts and successwithwarnings.
  Original source-delete qualification refuses members_skipped; no native deletion was invoked.
- AutoRename uses bounded same-directory numbering, original pathvalidation and shared immutable
  archive-name reservations. Finalbegin/publication is no-replace. Raced numberedtarget survives
  with OutputCollision; no blind overwrite or automatic publicationretry.
- apply-all affects later ordinaryfile conflicts in this task only. Old/duplicate/priorjob/retired
  tickets do not decide another wait. Cancellation stops latermembers while retaining committed
  outputs/count; only owned unpublishedpartials are cleaned. Internalarchivecollisions refuse
  before native observation/prompt. Private opt-in cannot grant ArchiveRead write authority.
- Small additions reuse original runner/Job/worker/outputsession; no new transaction/runtime,
  dependency, generic framework or duplicated pathpolicy. Otherformats retain original behavior.

## Independent verification actually performed

One foreground serial batch, C#isolated apphost first, matchingisolatedworker rebuilt after
feature-unifying checks before C#execution. Cargo checked machine lane per command and inherited
CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256. No delegate or broadreview.

| Check | Independent result |
| --- | --- |
| C#Debug build |exit0,0warnings/errors |
| Archive6small targets |29pass/0fail/0ignored,exit0 |
| Strict Clippy isolated/default tests+examples |both exit0,-Dwarnings |
| Archive scoped fmt |exit0 |
| Matchingisolatedworker build |exit0 |
| Exact C#LIST/RUN/PASS |same4names,selected4/run4/pass4/skip0/fail0/not_run0,exits0 |
| Specs/context/fixtureselector/diff |all exit0 |
| Identity/sourcehash assertions |PASS;61reviewedsource/evidencefiles unchanged during checks |

Actual newfixture executes16small .NET-produced ZIP scenarios through original runner/Job/actual
worker/nativepublication: Ask choices, explicit policies, all3apply-all decisions/oneprompt,
nexttask isolation/stale tickets, waitingtargetchange, cancelbefore/aftercommit, renamedtargetrace,
internalcasecollision and reservedoriginal numberedname. It asserts exactold/newbytes, actual
FilesCommitted/FilesSkipped/category, extraction_summary, foreignpartialretention, ownpartial
absence, sourceexclusivereopen and cleanuptrue/ActiveCount0. Domain Skip regression separately
asserts3skips/0partials/0commits/members_skipped. No real source-recycle/delete API.

The same4C#collection also exercises original atomicpathoverwrite, actualworker operationisolation
and basic native ZIP extraction. Explicitworker/codec/ZIPfixture/Python environment was configured
before this independent run: noSKIP or repeat required. Implementer's earlier3pass/1environmentSKIP
remains historical, not relabeledPASS. No complete password/longpath/format matrix was repeated.

Specs106documents/14owners and context82implement/79checkentries pass; preexisting
native-contracts.md35497byte contextwarning remains. No globalfmt/workspace compile/test result
is claimed in this bounded run. No unrelatedfile was repaired or reformatted.

## Evidence and limits

Scratch C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df:
`conflict-independent-verify.ps1`, commandledger, per-step logs, baseline and
`conflict-independent-assertions.json` retain exactnames/counts/exits/sourcehashchecks.
Acceptance changed no implementation/tests/submittedreports/results; this report alone is added.
Current61file baseline was verified, not a fresh claim to have independently compared all107
historical files to the implementer's original baseline. Closing0archiveworkers/testhosts,
0archiveproduct/resourcepipes,fixedhelperabsent.

No100k/largefiles/performance,fullpassword/longpathmatrix,UI/FIFO/risk,formalDesktop/installedhost,
UNC/systemconfiguration,sourceRecycle/delete,package/install/sign/commit/push. NativeRecycle stays
native_protection_unproven/NotAttempted. **ZIP conflict slice accepted; stop. Otherformats, B3 and
fullB2/PhaseA/A–F remain unfinished.**
