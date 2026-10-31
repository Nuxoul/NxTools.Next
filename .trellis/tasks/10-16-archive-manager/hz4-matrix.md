# 第六步 HZ4：真实进程崩溃／恢复测试 — exact matrix

2026-10-29; **Phase E — Format expansion and ZIP editing**, original Step6 only. See [outcomes](hz4-outcomes.md), [before-code route](hz4-route.md). **Not all green / not parent acceptance / not resume or power-loss acceptance.** All original failed runs remain immutable.

## Evidence root / fields

Root: `C:\Users\Administrator\.pi-desktop\scratch\0c0e5320-11c4-40dd-bb50-f6a437188fc1\hz4`.

Each named run has an exact command JSON and raw log at root, and a same-named subdirectory with results.json, frozen identities, per-case GUID root, case-result.json and evidence. The run wrapper stores argv/Popen PID/exit/time/same-process Cargo env/log hash. Process records include retained exact handle/PID/creation FILETIME/image, live driver PID/creation, verified actual parent link, selected nonce/operation/session/stage/visit receipt, exact worker handle/creation/image, Host exit `0x485A3401`, worker Job exit verification and fresh restart identity/refusal. Disk facts include source/archive/output/partials/native fileID/device/link count/attributes/mtime/length/SHA256 and canaries. Oracles save exact members/kinds/content hashes/format metadata. Terminal evidence is absent when not emitted; terminal CommitJournal is NOT a durable recovery record. Private claim schema/hashes do not grant authority.

## A. Instrumentation RED → GREEN

| Run | Exact case | Actual original-owner fact | Verdict |
| --- | --- | --- | --- |
| process-red | edit-Zip-edit_verifying | Actual Job succeeded,1commit,cleanup true,exit0; requested receipt absent | RED FAIL |
| process-red | extract-Zip-before_cleanup-1 | Actual Job succeeded,2commits,cleanup true,exit0; requested receipt absent | RED FAIL |
| process-green | same2 exact names | Real selected receipt → retained-handle termination → old/new oracle → new process stale_operation, no disk mutation | 2PASS |

Initial build-red/build-red-v2 compile errors are retained as fixture infrastructure failures, not these REDs. build-red-v3 and build-green are successful complete Debug test-harness builds.

## B. Main53 cases — default compressed-header7z profile

Edit exact name syntax: `edit-<format>-<point>` with format exactly `Zip`, `SevenZ`, `Tar`, `TarGz`, `TarXz`; visit1. All35 exact names and process/disk/record hashes are expanded in `process-matrix/results.json`.

| Actual point | Zip | SevenZ | Tar | TarGz | TarXz | Meaning |
| --- | --- | --- | --- | --- | --- | --- |
| edit_verifying | PASS | FAIL7Z01 | PASS | PASS | PASS | ZIP verified spool;7z before full candidate verification;TAR before inner verification/outer encode; not identical durable seams |
| candidate_write | PASS | FAIL7Z01 | PASS | PASS | PASS | Original native staging write; old archive/retained partial |
| edit_source_rechecked | PASS | FAIL7Z01 | PASS | PASS | PASS | Candidate+held readback verified, real readers closed, source FILE still protected |
| commit_in_flight | PASS | FAIL7Z01 | PASS | PASS | PASS | FILE-only release complete; native single rename not called yet |
| native_published | PASS | FAIL7Z01 | PASS | PASS | PASS | Actual native success/publication before close; complete new archive |
| before_cleanup | PASS | FAIL7Z01 | PASS | PASS | PASS | Worker/pumps drained, original cleanup about to run |
| after_cleanup | PASS | FAIL7Z01 | PASS | PASS | PASS | Original terminal after actual cleanup; test observation, not a durable seam |

For FAIL7Z01, actual Modify returns corrupt_archive/0commits before verification: five requested hooks absent; two cleanup receipts concern a failed operation and do not prove publication. Independent old-source oracles pass, expected-new assertions fail. Preserve all7FAIL.

Extraction exact names: `extract-<format>-<point>-<visit>`, format `Zip`, `SevenZ`, `Tar`. All18 in process-matrix/results.json. Source bytes/native identity/metadata unchanged in every PASS.

| Point / visit | ZIP | 7z | TAR | Actual published-file count |
| --- | --- | --- | --- | --- |
| candidate_write /1 | PASS | PASS | PASS | 0, real staged partial may remain |
| commit_in_flight /1 | PASS | PASS | PASS | 0 |
| native_published /1 | PASS | PASS | PASS | exactly1 complete file |
| native_published /2 | PASS | PASS | PASS | exactly2 complete files |
| before_cleanup /1 | PASS | PASS | PASS | exactly2 complete files; directories retained |
| after_cleanup /1 | PASS | PASS | PASS | exactly2 complete files; directories retained |

**Main total46PASS/7FAIL/0SKIP**, not53PASS. Independent CPython zipfile/tarfile, explicit stock7Zip and system bsdtar are used as appropriate; no unopened encryption counted as success. Every PASS has an actual fresh-process original Session refusal and unchanged disk snapshot.

## C. Explicit one-variable7z differential — stock7zip-plain

Only `-mhc=off` changes from stock compressed-header seed. Same2files+emptydirectory, non-solid LZMA2, full original metadata/member/content assertions. This is a disclosed different input profile, NOT a fix of B or replacement of its negatives.

| Run | Exact case | Result |
| --- | --- | --- |
| sevenz-plain-hypothesis | edit-SevenZ-edit_verifying | PASS |
| sevenz-plain-boundaries | edit-SevenZ-candidate_write | PASS |
| sevenz-plain-boundaries | edit-SevenZ-edit_source_rechecked | PASS, actual readers closed before source FILE handoff |
| sevenz-plain-boundaries | edit-SevenZ-commit_in_flight | PASS, actual exclusive source reopen after FILE-only release |
| sevenz-plain-boundaries | edit-SevenZ-native_published | FAIL, bsdtar rejects new7z |
| sevenz-plain-boundaries | edit-SevenZ-before_cleanup | FAIL, bsdtar rejects new7z |
| sevenz-plain-boundaries | edit-SevenZ-after_cleanup | FAIL, bsdtar rejects new7z |
| sevenz-disagreement-restart | same3 postpublication names | Still3FAIL; complete stock7Zip exact manifests retained, fresh original Session refusal PASS and disk facts unchanged |

Stock7Zip t/list/individual member extraction succeeds for the published new sources; exact first/second content/hash/known mtime/attribute survives and only published/ is added. Bsdtar -tf returns1 `Damaged 7-Zip archive`. Both observations are retained; no secondary assertion removed, no whole-format correctness claim.

## D. Fresh restart / claim / identity supplements

Run `restart-supplement`: original9groups7PASS/2FAIL, earlier test-assumption errors retained. Run `native-supplement-green --native-only`: corrected exact native seams/optional publication assertions3PASS. Latest logical-group results below combine the retained originals with only necessary native corrections, not a broad rerun.

| Group / exact scenario | Original real boundary | Fresh restart / disk evidence | Latest |
| --- | --- | --- | --- |
| restart-untrusted-claims: missing | ZIP extraction first native_published | No product durable journal; stale_operation; first complete output and partial evidence unchanged | PASS |
| same: malformed `{broken` | same killed attempt | Claim bytes/hash preserved, no parser/cleanup/replay | PASS |
| same: unsupported-old schema0 | same | Same explicit refusal, claim unchanged | PASS |
| same: unsupported-new schema65535 | same | Same explicit refusal, claim unchanged | PASS |
| same: forged-untrusted schema1 with replay/delete fields | same | Fields ignored, canary/outputs/source unchanged; still no durable authority | PASS |
| restart-source-replaced | ZIP extract first publication, then owned source inode replacement | New valid source remains; original copied to private evidence; no replay/cleanup | PASS |
| restart-target-replaced | same boundary, then owned first output inode replacement | Replacement bytes/identity remain untouched | PASS |
| restart-target-hardlink | same boundary, then first output links to owned canary | Native fileID/linkcount2/canary preserved | PASS |
| restart-target-reparse | same boundary, then output replaced with GUID-owned junction | Junction/redirect canary/prior published output and partials unchanged; no follow/cleanup | PASS |
| repeat-crash-fresh-restart attempt1 | ZIP edit commit_in_flight | Complete old source, retained partial, new-process refusal | PASS |
| same attempt2 | New actual operation/session, same private logical label | Complete old source, both partials retained, another new-process refusal | PASS |
| native-injected-before_flush-112 | Original before_flush; injected native112 | Failed/0commits, output_published=false, old source intact, fresh refusal | PASS |
| native-injected-before_recheck-5 | Real Flush completed, original before_recheck; injected5 | Failed/0commits, false, old source intact, fresh refusal | PASS |
| native-injected-commit_in_flight-64 | Actual FILE-only handoff; injected64 | CommitOutcomeUnknown, absent/null publication stays UNKNOWN, retained_uncertain, old source intact, fresh refusal | PASS |

Latest9groups/14fresh-refusals, no resume success. Native112/5/64 are owned injections, not real whole-disk exhaustion/network loss. Old `after_flush-5` failed because no such original hook exists; old64 assertion failed by incorrectly requiring a present null instead of the accepted absent-or-null unknown. Corrected test selects actual before_recheck and protects unknown; neither error is a product RED. Repeated logical label is explicitly private, not product persisted history. Original Desktop/HZ3 tests protect true same-logical new-attempt deduplication; repeated fresh refusal never republishes the already complete extracted member.

## E. Native and compiled/source regression disposition

`native-regressions/effects-reviewed-names.json` expands all13 names. Every name has original same-name LIST/RUN logs. HZ4 gates2, HZ3 publication2, EIC2, ZIP native handoff2, TAR native handoff1,7z native handoff1, actual conflict/admission/cancel1 allPASS=11. Password wait/retry/cancel FAIL before actual pending. CLI cleanup initially lacks explicit evidence-root; configured followup stillFAIL first admission, retained native category helper_untrusted. Neither counted as reached fault point. No broad native/performance/full-product run.

Desktop archive349 / Protocol91 / IPC43 / Core100 / Archive targeted100 PASS; strict/workspace PASS. Fresh original HZ3 native export is used. No Rust source changes. Source gates (spec, framework/editor provenance, archive-cli16, diff) PASS. Relevant11archive-file fmt PASS; broader package fmt FAIL in3unmodifiednetwork files.

Release product Host actually builds. Full Release Tests fails unchanged ArchiveRiskTests.cs:72 CS0162. Scratch exact-source Release probe plus actual Release Host:2exact LIST→RUN PASS; fault entry returns2 release_fixture_disabled before any path/native access. Not full Release Tests/package acceptance.

## Stop / preservation

991-file opening source baseline: exactly4declared old-file modifications before checkpoint prepend, all987others unchanged; exact unchanged portions proven for the4modified files. Original checkpoint209948bytes is retained as an exact full suffix after top prepend; final seal records size/hash. All earlier HZ3/readmission/EIC/HZ4 failures remain. No user source reclamation, privilege expansion, delete-old-first, arbitrary path cleanup, GUI bypass, package/sign/install, commit/push or Step7 work. **Stop for host callback and parent independent acceptance or bounded Step6 repair.**
