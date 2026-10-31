# HZ4 bounded 7z / Release Tests repair — exact matrix and command index

2026-10-29, original Step6 of **Phase E — Format expansion and ZIP editing**. Companion: `hz4-7z-release-repair-outcomes.md`. This is a new matrix, not an overwrite/reclassification of original HZ4 FAIL reports. Parent narrow acceptance remains pending.

`R = C:\Users\Administrator\.pi-desktop\scratch\0c0e5320-11c4-40dd-bb50-f6a437188fc1\hz4-7z-release-repair`.

Exact argv/exit/timing/PID/env/log hashes are in `R\command-ledger-final.json`; raw immutable `<label>.json/.log` remain available. Matrix payloads are `R\matrix-detail.json`, final `fc\results.json`, `fp\results.json`, `fr\results.json`, `final-normal-data.json`, `rcf\results.json`, plus per-root `case-result.json`. Their hashes are sealed, not inferred from this display. Wrappers and controls are not summed as extra product cases.

## 1. Original-bridge real process matrix (final script / current frozen bundle)

Common gates for **every** row: original Job source/native authority; selected receipt stage/visit1/nonce/operation/session and retained testapphost PID/creation/image/parent verified; terminate only exact retained Popen handle; actual worker Job exit verified; old/new complete independent stock7Zip `t` + exact members/content/known metadata; unrelated canary unchanged; fresh original Session `resume` refuses `stale_operation`, does not replay or clean, entire disk snapshot unchanged. No successful terminal is invented for an interrupted Job.

| Exact case suffix (`edit-SevenZ-…`) | Compressed profile process/integrity | Compressed bsdtar | Plain profile process/integrity | Plain bsdtar | Publication/source fact |
| --- | --- | --- | --- | --- | --- |
| `edit_verifying` | PASS | PASS | PASS | PASS | Old source bytes/native identity/metadata exact |
| `candidate_write` | PASS | PASS | PASS | PASS | Old source exact, actual partial retained as observed |
| `edit_source_rechecked` | PASS | PASS | PASS | PASS | Old source exact; exclusive source read denied before handoff |
| `commit_in_flight` | PASS | PASS | PASS | PASS | Old source exact; actual old readers closed, FILE-only handoff/exclusive source read succeeds |
| `native_published` | PASS | **reader-unsupported** | PASS | **reader-unsupported** | Complete new archive, original known metadata retained |
| `before_cleanup` | PASS | **reader-unsupported** | PASS | **reader-unsupported** | Complete new archive, original worker already exited |
| `after_cleanup` | PASS | **reader-unsupported** | PASS | **reader-unsupported** | Complete new archive, original worker already exited |
| Totals | **7 PASS** | **4 PASS / 3 unsupported** | **7 PASS** | **4 PASS / 3 unsupported** | Fourteen actual interruptions and fresh refusals |

Final command labels `final-compressed-seven` / `final-plain-seven` both exit0. Controlled process exit is **`0x485A3401` /1213871105**, not an arbitrary nonzero exit or an admission failure. No skipped case is used to claim a reached hook. The `skip=0` case field does not mean every reader passed; the above six unsupported results are explicit.

Source SHA256s (all rows carry full actual native IDs/size/mtime/attributes/links/hashes in the JSON):

- Default compressed old source225bytes: `5772D8F69B2F1146B27F8F42C6A430E52173B8E0275D0986AC8CEA728CE522C4`.
- Explicit plain old source: `C552721BE88B77AC3E8C625DE41FA33B05B552CDE07E99A8D71273908744C329`.
- Complete new published source255bytes, both profiles: `07191B13A8E1D74F80F39F893FED1B7EA76E422686DFD6FC9E63849EBAB34A8C`.
- Four prepublication rows retain the exact original native file ID per case; three published rows have the actual replacement native ID. The JSON retains IDs rather than treating a path/hash alone as identity.
- Full decoded header180bytes SHA256 `179A1F8F18F3D53E3CE66C97696C74100F5686932ADBA939770299788291D384`. New `published/` attributes remain undefined; old file/file/directory values32/32/16 persist.

### Normal and default-accounting controls (not interruption rows)

| Control | Actual result | Secondary result |
| --- | --- | --- |
| `final-normal-data.json`, `mode=run` | exit0/succeeded,1verified FILE/1committed FILE/one journal publication/cleanuptrue, no TerminateProcess; original known metadata exact; fresh refusal preserves disk | reader-unsupported |
| `current-unqualified-control` | process/integrity PASS and fresh refusal PASS; overall **FAIL**, command exit1, because opt-in is absent | Actual bsdtar exit1 remains FAIL |
| Final caller-import and unrelated-reader controls | Independent module import succeeds; unrelated exit2 or unrelated exit1 message remains FAIL even for the conforming archive | No automatic exemption |

A passive hook receipt in `mode=run` is not a process interruption. The normal control explicitly checks mode/exit/terminal/absence of termination, not absence of every receipt.

## 2. Secondary reader controls — real binaries, not mocked reader success

Final command `reader-controls-final` exit0. Same packed file body and valid recomputed signature/start/next CRCs:

| Witness | stock7Zip test exit | bsdtar list exit | Classification/control |
| --- | --- | --- | --- |
| Canonical partial inline attributes | 0 | 1 | reader-unsupported only under the approved exact structure/identity |
| Swap only bitmap/external byte order | 2 | 0 | Classifier must reject nonconforming bytes |
| All-defined diagnostic-only fourth attr | 0 | 0 | Classifier rejects exemption; actual reader PASS applies |
| Current normal encoded output positive | Primary full manifest is separately verified by original Job control | Actual failing reader recorded there | Bounded signature/start/next/packed/decoded CRCs and full exact header/streams qualify |

Nine fail-closed rejection controls pass: bad startCRC, bad nextCRC, main packedCRC, encoded-header packedCRC, other metadata with recomputed valid CRC, trailing bytes, unknown primary hash, unknown secondary hash, unknown version. Final process wrapper additionally checks wrong exit/message rejection. No metadata omission, fabricated new-directory attribute, changed writer/profile, dependency upgrade or oracle substitution is applied to product.

## 3. Final restart supplements (current script/current bundle)

Command `restart-final` exit0; `fr/results.json`: **9 logical groups PASS,14 fresh-process refusals PASS**. Refusal remains `stale_operation` / no durable authority, never resumed recovery.

| Group | Fresh refusals | Mandatory retained facts |
| --- | --- | --- |
| Missing / malformed / schema0 / schema65535 / forged private claims | 5 | Claim bytes/hash, one previously published extraction member, original source and canary unchanged; no product parser/replay/cleanup |
| Source replaced | 1 | Different actual native file ID; refusal preserves replacement and old evidence |
| Target replaced | 1 | New target bytes/native identity retained |
| Target hardlink | 1 | Actual target/canary shared ID and linkcount2 retained |
| Target junction/reparse | 1 | Actual reparse state and redirect canary retained, no following cleanup |
| Two interruptions / same private logical label | 2 | Distinct operations/process identities, old source exact, no duplicate authority |
| Injected before_flush112 | 1 | Failed terminal native112/0commits/false publication; source exact |
| Injected before_recheck5 | 1 | Failed terminal native5/0commits/false publication; source exact |
| Injected commit_in_flight64 | 1 | Unknown commit/null publication/retained uncertain candidate, no retry; source exact |

Injected native errors are not real disk-fill/network-loss tests. Private terminal/CommitJournal/claim hashes are evidence, not durable recovery or cleanup authority.

## 4. Exact effects-reviewed C# LIST → identical RUN selection

Frozen current actual Tests DLL/Host/worker identities are in the outcomes and `native-current/identities.json`. Every line below has its own `current-native-NN-list` and `current-native-NN-run` recorded argv. Each LIST selects exactly that one name, each RUN returns **exit0 / selected1 run1 pass1 skip0 fail0 not_run0**. Total **23 PASS**; legacy password/CLI failing names are not selected.

| NN | Exact name |
| --- | --- |
| 01 | archive HZ4 process fixture capability release gate |
| 02 | archive HZ4 process fixture no durable restart authority |
| 03 | archive sevenz edit wire ordinary replace accepted |
| 04 | archive sevenz edit wire strict shapes ZIP preserved |
| 05 | archive sevenz edit wire admission snapshot capabilities |
| 06 | archive sevenz edit native actions subtree all delete |
| 07 | archive sevenz edit native AES header replace conflict |
| 08 | archive sevenz edit native source cancel handoff unknown |
| 09 | archive sevenz edit native empty subtree implicit unicode |
| 10 | archive sevenz edit native independent metadata retention |
| 11 | archive sevenz edit native conflict skip native terminal |
| 12 | archive HZ3 AC01 native directory publication zero output |
| 13 | archive HZ3 AC01 publication wire retained directories |
| 14 | archive EIC AC01 native directory merge zero publish |
| 15 | archive EIC AC02 native cancelled reply only ZIP TAR 7z |
| 16 | archive edit interaction native ZIP Ask decisions single publish |
| 17 | archive ZIP edit native handoff gates and ownership |
| 18 | archive ZIP edit native handoff failure unknown no retry |
| 19 | archive tar edit native actions metadata empty |
| 20 | archive tar edit native source cancel handoff |
| 21 | archive tar edit native failure unknown late cancel |
| 22 | archive tar edit native conflict injected lowspace |
| 23 | archive job fixture actual conflict global admission cancel |

Effects are only tiny GUID-owned source/target/partial and original worker/Job/native probes. No live product/Explorer/Registry/service/ACL/UserChoice/Recycle or real-user archive effects are authorized. Explicit Python/stock7Zip/bsdtar paths and current worker hash are recorded in `native-current/env.json`. Fresh `native-current/native-terminals.json` feeds Desktop349; an old worker export is not substituted.

## 5. Recorded command/exit and compiled regression index

This table is an index; **full exact argv are the immutable command JSON and final ledger**, not reconstructed shell strings. Cargo invocations have locked/offline/serial execution and required jobs1/incremental0/codegen-units256 environment, with idle checks. Full Release .NET compilation uses scratch BaseOutputPath and headroom preflight; no publishing operation is implied.

| Label / recorded command family | Exit | Actual result |
| --- | --- | --- |
| `compressed-native-red` / original bundle compressed edit_verifying | 1 | corrupt_archive; requested hook absent; not crashpoint PASS |
| `plain-native-red` / original bundle plain native_published | 1 | Original publication reached; stock PASS / bsdtar FAIL retained |
| `header-adapter-red-v2` / isolated-codecs lib header controls | 101 | Explicit1PASS, omitted-default1FAIL; causal product RED |
| `header-adapter-green-v2` / same owner lib | 0 | Full lib44PASS |
| `worker-build-current` / cargo build isolated-codecs worker | 0 | Current frozen worker produced |
| `debug-tests-build-current` / complete Tests csproj | 0 | Full Debug0warnings/0errors |
| `full-release-build-red` / complete Tests csproj Release | 1 | Original CS0162 at risk fixture72 |
| `full-release-build-current` / complete Tests csproj Release | 0 | Full Release0warnings/0errors |
| `full-release-gate-list` → `full-release-gate-run` | 0 / 0 | Complete actual Release assembly exact2PASS/0SKIP |
| `debug-gate-list` → `debug-gate-run` | 0 / 0 | Complete actual Debug assembly exact2PASS/0SKIP |
| `full-release-entry-refusal` | **2 expected** | release_fixture_disabled before FS; nonexistent path stays absent |
| `current-unqualified-control` | **1 expected** | Default secondary assertion still fails; process/integrity passes |
| `final-compressed-seven` / `final-plain-seven` | 0 / 0 | Process14PASS, bsdtar8PASS/6unsupported |
| `final-process-checks` | 0 | Final14+normal+caller import+unrelated-secondary controls |
| `restart-final` | 0 |9groups/14freshrefusals PASS |
| `reader-controls-final` | 0 |3reader pairs,1encoded positive,9negative rejections |
| `native-current-exact` / exact23 fixture pairs | 0 |23PASS/0SKIP |
| `desktop-current` / cargo test Desktop bin archive | 0 |349PASS,1167filtered |
| `layers-current` / cargo test Protocol+IPC+Core | 0 | Protocol91+IPC43+Core(93lib+6storage_management+1storage_runtime)=234PASS |
| `archive-current` / lib +11 focused targets | 0 |102PASS; breakdown below |
| `strict-current` / clippy Desktop+Protocol+IPC tests -Dwarnings | 0 | StrictPASS |
| `strict-archive-current` / clippy Archive isolated-codecs tests -Dwarnings | 0 | StrictPASS |
| `workspace-current` / cargo check workspace tests | 0 | CheckPASS, not whole workspace test execution |
| `static-current-all` and recorded child commands | 0 | Spec/CLI/release/editor/selection/scopedfmt/diff/AST/Debug active-source preservation PASS |
| `accounting-protection-v2` | 0 |1186baseline:7declared changes/1179unchanged, original1645evidence unchanged, separate owner spec protected |

Final Archive actual test breakdown:

| Target | PASS |
| --- | --- |
| lib |44 |
| edit_interaction_worker_contract |2 |
| eic_repair_worker_contract |4 |
| job_execution_contract |2 |
| sevenz_edit_safety_contract |4 |
| sevenz_edit_worker_contract |1 |
| tar_edit_matrix_contract |8 |
| tar_edit_raw_header_contract |4 |
| tar_edit_worker_contract |3 |
| worker_wire |19 |
| zip_edit_contract |10 |
| zip_edit_worker_contract |1 |
| Total |**102** |

The complete `.json/.log` ledger also retains nonzero infrastructure attempts: early fixture CRC-category expectation/private field-access compiler errors, differential UTF-8/raw-LZMA/filename/source-symbol issues, long-path inventory FAILs, normal-wrapper receipt assumption and initial baseline-membership bookkeeping mismatch. They are not erased, counted as product REDs or confused with expected Release/default refusal controls. See outcomes§7.

## 6. Seal and limits

`delivery-seal-final.json` records final source/binary/oracle/report/command/evidence hashes, protected old-file changes, old1645evidence identity and entire217143-byte checkpoint-opening exact-suffix proof. No old report is replaced. The separately read owner spec is accounted outside the1186baseline. Expected final baseline includes8declared changes (the previous7 plus checkpoint prepend),1178unchanged; new repair route/outcomes/matrix/reader tests are separately listed.

No full historical53-process rerun, full fixture suite, full workspace test execution, physical-user/installed-product/Release-worker/power-loss/large-resource/performance/package acceptance is implied. Password/CLIhelper_untrusted/readmission/source_unavailable/FB-AC05 remain OPEN. Fresh refusal is not durable recovery. Pending parent narrow acceptance; no HZ5, package/sign/install or duplicate callback task.
