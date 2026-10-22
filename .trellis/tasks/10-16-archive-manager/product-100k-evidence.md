# Explicit finite product100k read — implementation evidence

Date: 2026-10-20. Canonical **Phase A — Engine contracts and safety foundation** remains in
progress. This closes only the explicit finite100k read/metadata-transport slice. No fullENG08,
ENG09,SAFE04,PhaseA/A–F/native-user/deployment acceptance. [Contract/matrix](product-100k-contract.md),
[durable results](product-100k-results.json), [unique current checkpoint](phase-a-checkpoint.md).
Original resource/product independent reports and all oldresults remain byte-identical.

## Implementation and preserved boundaries

Actual positive chain: Rust `ArchiveReadClient` -> original typed platform pipe -> production
`ArchiveReadSession` in isolated TEST apphost -> original `ArchiveWorkerRunner` suspended Job +
source/helper lease -> actual Rust worker -> immutable snapshot -> bounded metadata credit ->
actual process/pumps/Job cleanup -> original once-only terminal. No direct-worker/runner-only
substitute, formal Desktop or installed PlatformHost launch.

Explicit archive version2 requires per-operation finite limits. Defaults/v1 remain4096entries,
4MiBmetadata/retention,2entry batches and8MiBstdout. v2maxima:100000entries,128MiBengine metadata,
64MiBwork minimum (512MiBmaximum unchanged),64MiBprivate cumulative transport and64MiBcumulative
consumer receipt,256entries AND256KiBperbatch, one unacknowledged batch. Actual100k request Job
reservation256MiB=64work+128metadata+64overhead, under original1GiBprocess/2GiBaggregate/max2.
Cumulative transfer/receipt is NOT simultaneous RSS; ACK never refunds cumulative receipt quota.
Only one finite batch is constructed over the existing snapshot, no complete business JSON/second
100kVec. JSON byte accounting includes Unicode/HTML-sensitive/DEL escaping; exact C#/Rust final
serialization checks remain. Actual terminal includes bounded count-only transfer statistics.

Old strict workers first receive legacy-shaped schema2/read_contract1+conservative limits. Actual
accepted must report all old capabilities plus v2/bounded_metadata before ONE `read_upgrade` on
the ORIGINAL pipe. FIFO upgrade precedes source_acquire ACK; worker applies/seals before engine/
codec reads. No unknown-field early-exit misclassified crash, no large-read fallback, no second
process/control runtime. Unknown version wins over version-specific budget validation. Legacy
private/native requests and source-delete original run/session/Rust epoch protections remain.
Original30sproducttimeout, reliable Drop/local-error cancel, fair/start-before-cancel delivery,
overflow epoch, saturated ordinary notification actualEOF, max2reservations until actualcleanup,
source sharing/native publication/codec/recycle policies unchanged. Production helper fixed,
no PATH/env/request override. Native recycle stays native_protection_unproven/NotAttempted.

## Actual vertical red -> green

All Cargo commands inherited jobs1/incremental0/RUSTFLAGS `-C codegen-units=256`; one machine lane
checked before commands, bounded wait for unrelated Desktop/wallhaven builds, none terminated.
C#isolated apphost built first. Python used PYTHONDONTWRITEBYTECODE=1. All logs remain session
scratch `30608b1f-4d13-4908-94fc-6b4d37c1bb7d`; project regressions do not depend on those files.

| Actual command / seam | Red | Green |
| --- | --- | --- |
| `cargo test -p nxtools-archive --features isolated-codecs --offline --test product_read_chain explicit_finite_v2 -- --test-threads=1 --nocapture` |1failure, IncompatibleVersion at actual owner admission, exit101; conservative actual chain first refused ResourceLimit/cleanuptrue |1pass, exit0; v2literal snapshot metadata/cleanup/originalconnection |
| same product target `v2_` |Unicode byte-bound fixture returned MetadataBudget,3pass1fail/exit101 |receiver serialization charge corrected;4pass/exit0 |
| product target `v2_del_` |1fail/exit101, C#DEL escaping made batch too large |DEL charged, actual chain green in final21chain tests |
| C#`archive product fixture v2 finite negotiation` |unknown version+large limits misreported invalid_request,1fail/exit1 |version-first fixed; then strict-old-parser case independently red: worker_crashed/exit1; compatible bootstrap+upgrade green1/1exit0 |
| `cargo test -p nxtools-ipc --lib --offline interrupted_large_frame -- --test-threads=1` |1fail/exit101: after8192consumed bytes and interrupted read, actual function returned only tail |original connection owns prefix across select cancellation; fullIPC30/30exit0 |

One bounded readonly review identified DEL escaping, oldworker preaccepted strict parsing and
version precedence; parent reproduced/fixed/retested each. No second review. New cases also cover
exact member/byte boundary and one less/over, no successful truncation, long Unicode, missing
capability, invalid transport/unauthorized writes, cumulative consumer quota after ACK, multibatch
slow/noACK/wrong/duplicate ACK/cancel/Drop, fullcommandqueue operation/clientDrop, disconnect/
reconnect/oldgeneration, source/helper held until actualcleanup and sameconnection reuse.
Existing cancel/epoch/ordinary-noticeEOF/source-delete/TAR/MOTW/publicationcleanup remain covered.

### First actual100k failure and the single declared repeat

First `python tests/performance/archive-resources/run.py ROOT product100k-v2 correctness`:
driverexit1/probeexit101, Stopped/probe_deadline, measurementComplete, cleanupIncomplete. Default
ResourceLimit proved. Original host/worker exited around4.2s; no v2remote terminal capture. Probe
waited to its120sdeadline and retained uncertaincleanup evidence. **Not a100ksuccess**.

A concrete original-pipe issue was then red-reproduced: read_bounded_frame held consumed prefix
in a disposable future-local Vec inside select; sweep/command branch could drop it. The original
connection now retains <=1MiBpending_frame, no extra reader/queue/runtime. This is source-backed
and deterministic causal evidence for partial-frame corruption; the failed run had no saved raw
frame/pipeerror proving an exact byte offset. Do not reinterpret it as confirmedcleanup/success.
A new5000member actualchain regression passed. Probe now notices local failedterminal promptly,
without equating it to remotecleanup. No timeoutincrease or algorithm optimization.

Before running again the parent declared ONE repeat to verify this specific repair with SAME
manifest/budgets/30sdeadline, not best-score hunting. `run.py ROOT product100k-v2 repeat` then
exited0/PASS, complete measurement/cleanup, no stop/force/failure. No further100krun occurred.
The two attempts/sample JSONL hashes and original failure are in results; bytes unchanged.

## Independent manifest and actual Debug observation

`fixtures.py ROOT --catalog-only` generated CPython ZIP_STORED exactly100000members:100directories,
100Unicodefiles,9081emptyfiles,1362285payloadbytes. Streaming JSONL manifest; no rawduplicate,
1GiB or largeZIP64 generation. ZIP12,161,983bytes SHA256
`34a9f9fe2e1d669b825d3557e523cadf00aeabf1ebfe187c99fcd4475cda956d`;
manifest SHA256 `4fd4bdca3f5ceb0f388206c1c2a12cf827be97856313f3be85c5efb31e2768dc`.

Actual Rustowner stream compared every ordinal/name/kind/size/packed_size plus encoding/codepage,
actual same snapshot `32cc12eb-a18d-4ec9-b845-93ee192d4fbc`, contiguous sequence and fullterminal
counts.100000entries/391batches; no skipped failedinput or successfultruncate. Owner and captured
production terminal exactly matched. After actualcleanup, sourcewritable/helperremovable,
sameoriginalconnection creditZIPreuse succeeded. Defaults still ResourceLimit; v1explicit100k
stillInvalidRequest. No readonly result grants extract/create/recycle authority.

| Debug observation | Actual |
| --- | --- |
| First business metadata batch from original dispatch |2275.9759ms |
| Full product terminal/cleanup observed |7006.9153ms (runner7001ms), below unchanged30s |
| Whole probe including setup/defaultrefusal/reuse |8093.3534ms |
| Private cumulative bytes including headers/control frames |16745255 |
| C#serialized business-batch cumulative bytes |16708854 |
| Rustserialized business-batch cumulative bytes |16707854 |
| Largest batch / maxentries / in-flight |42817bytes /256 /1 |
| Job approved process budget |268435456bytes; NOT sampled memory |

C#and Rust totals differ by Unicodeescaping, not missingentries; both explicit quotas enforced.
Requested20ms sampling/50msdiscovery; actual100kworker maximumvalidgap32.534ms. Samples include
normalprocesslifetimes; no cacheflush/pressure, ordinary access not controlledcold/warm.
Firstbatch is NOT firstpaint, noUI. Debug7s does not promise Release1.5s or universalformatresources.

| Exact PID/role | Validpairs / failures | Sampled WS / Private bytes |
| --- | --- | --- |
|25592 Rustcaller |245 /0 |9232384 /2789376 |
|2556 C#host, parent25592 |243 /0 |67211264 /36499456 |
|18788 actual100kworker, parent2556 |223 /0 |67010560 /70864896 |

All exactimages/parentprovenance verified, allalive_at_end=false. Defaultrefusalworker1776 and
smallreuseworker24796 are separate records (9and1samples), not mixed into actual100kworker peak.
Sampled peaks may miss subintervalpeaks; PrivateBytes is processcommit, not systemRAM or reserved
codec/model budget. Caller/host lifetime includes setup/refusal/reuse, not pureopenphasepeak.

## Final integrated verification on repaired product source

| Command / gate | Actual final result |
| --- | --- |
| Archive `--features isolated-codecs --offline --no-fail-fast -- --test-threads=1` |150pass/0fail/0ignored,26groups,exit0 |
| Protocol / IPC `--lib --offline`, serial |53/30pass,0fail/ignored,exit0 |
| Strict Clippy Archive/Protocol/IPC `--tests --examples`, isolated/default |bothexit0 |
| `cargo fmt --all -- --check` |exit0 |
| `cargo check --workspace --tests --offline` |exit0,compilation only NOTworkspace tests |
| Extensions `--test archive_import`; Clipboard `--lib archive_` |7/5pass,0fail/ignored,exit0 |
| resource_probe example tests |11pass/0fail/ignored,exit0 |
| Python tool unittest discover test_*.py |28pass/0fail/error/skip,exit0 |
| Python AST fixtures.py/run.py |2filesPASS |
| C#Debug product/testbuild |0warnings/0errors,exit0 |
| Exactold132+2new C#LIST |selected134/run0/pass0/skip0/fail0/not_run134,exit0 |
| SAMEexecution |selected134/run134/pass134/skip0/fail0/not_run0,exit0 |
| LIST/RUN/PASSnames |same exact134; no largeZIP64 |

Detailed commands/start/end/exits/loghashes: scratch product-100k-verify-ledger.jsonl and
product-100k-results-full.json SHA256
`32ebaf1ebb87d3a6b194e498df57470de3dd841cc42b2f01c2081cf5f4171f2d`.
The initial related integration148/53/29 preceded review/framefix and remains historical, not
substituted for final150/53/30. Clippy two local mechanical warnings were fixed, noallow added.

Recoverable execution blockers retained: externalCargolane waits; build cache output/fingerprint
for one Archivebuildscript found NUL-filled, causing missingadvapi32link. Source/build.rs/manifests/
lock hashes unchanged. Only the two confirmedcorrupt generateddirectories were guarded-moved to
scratch, regenerated and same tests passed. No cargo clean/globalcache/dependency/thirdpartychange.
C#initialLISTorder assertion stopped BEFORE RUN; corrected comparison used same name set and actual
catalog order, then one integrated134execution. Initialeffectsregistrationreject had RUN0, fixed
reviewedclassification. InitialPythonadhocprecheck importpath failed BEFORE scenario; finaltooltests
exit0. These are not hidden producttestpasses or repeatedbroadgreenhunting.

## Preservation, cleanup and limits

All original independentreports/oldresults match openingSHA256baseline; explicit hashes in new
results. UnrelatedDesktop/Clipboard and externalprojects were not edited/reset/staged/terminated.
One failedsampled attempt and one declaredsuccessfulrepeat retained. Exactselfowned100k.zip and
credit.zip hashverified then removed after no-live-owner and successfulcleanup. Failedattempt's
exacthelper was later removed after hash/exit/exclusiveunlock proof; this is separate manual
owned-file cleanup, NOT retroactive remotecleanup. Failedpackage marker/defaultrefusalterminal,
manifest/status/JSONL/attempts/logs retained. Successfulpackage originalguard removed. No unknown
recursive sweep. Finalprocess/pipe/helper/document/diff observation is recorded below at closeout.

No formalDesktop/installedPlatformHost,Release/package/sign/install/commit/push,realuserarchive,
nativeRecycle/permanentsourcedelete,registry/service/drivemapping/cache/pagefile/OOMpressure,
UI/FIFO/productwrite,1GiB/largeZIP64run. Historical C#earlyexit cause remains unknown and unrelated;
this transportrepair does not diagnose/erase it. Broadformat/dictionary/million/longpath/UNC,
firstpaint/nativeuser/signeddeployment/fullPhaseA gates remain separate. Stopthisslice.

## Final documentation / hygiene

| Final gate | Actual result |
| --- | --- |
| `python tests/architecture/spec-contracts.py` |PASS106documents/14owners,exit0 |
| `python .trellis/scripts/task.py validate .trellis/tasks/10-16-archive-manager` |PASS68implement/65checkentries,exit0 |
| `python tests/architecture/gpui-component-catalog.py` |PASS79pages/79rows/78locked,exit0; localdocs only |
| `tests/architecture/platformhost-test-selection.ps1` |PASSfixture/native selection sourcegate |
| `git diff --check`, scopedwhitespace/conflict/temporary-probe scan |exit0/PASS |
| History/inputpreservation |74historicaltaskfiles unchanged; bothattempts+sampleJSONL hashes unchanged |
| Ownruntime resources |0host/worker/probe,0archiveproduct/resourcepipes,0fixedtesthelpercopies |

Existing unrelated native-contracts.md35497byte contextwarning remains; no new sizewarning.
Reference note32746bytes stays below32768. Externalparalleltracked changes were observed in
Desktopquality/clipboard.rs/ui/polish_tests.rs/ui/preview.rs, not edited/reset by this task.
Finalscope/rawhygiene: scratch product-100k-final-hygiene.json. Initialmarkerregexfalsepositive
matched pre-existing build-mode prose, corrected without rewriting it; adhoc WindowsGBKreader
was corrected to explicitUTF8. These were checkscript failures, not hidden productgatepasses.
All requested work/evidence complete; stop here. No remaining in-scope implementation blocker;
the broader PhaseA/native/deployment limits above remain open.
