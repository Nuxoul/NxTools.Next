# Phase E — authorized P202/P203 delivery outcomes

Date: 2026-10-22. Current phase remains **Phase E — Format expansion and ZIP editing**.
The [single checkpoint](phase-a-checkpoint.md) owns current status. This is implementation/isolated
verification evidence, **not independent Phase E acceptance or first-release completion**.
[Plan](implement.md) and [contract](phase-e-implementation-contract.md) retain the exact user-approved
ZIP adapter and edit-only source-FILE handoff decisions. Both earlier decision gates are CLOSED.
Historical capability/native32/uppercase failure reports are unchanged.

## 1. Delivered production paths and evidence levels

| Requirement | Current implementation / actual evidence | Remaining boundary |
| --- | --- | --- |
| Windows .Z01 | Public discovery parses exact canonical family ordinal and compares canonicalized exact member using platform semantics. Actual initial regression red CorruptArchive, then zip_volume_contract5PASS. Final native routing and actual product Browse created.Z01 PASS. No broad scan/path-equality bypass. | Case-sensitive branch is explicit source/test logic; this Windows run does not claim Linux execution. |
| Edit-only handoff | OpenForEdit only; candidate generation/verification while protected, actual worker readers close, acknowledged edit_reads_complete, still-held native fullID/version/hash/target checks, final cancel, FILE-only release, one original atomic path replacement. Source/output parents and candidate remain protected. | Accepted final-check/close/publication TOCTOU, not target-ID CAS. No global sharing change. |
| Original3native cases | Actual same-path validZIP publication PASS; pre-handoff cancellation and late-publication truth PASS; changed-native-revision refusal PASS. Additional2fixtures exercise missing/wrong evidence, reader-open32, candidate gate, ownership, ordinary lease compatibility, native5/64 and no retry. | Direct handle fixtures are not worker evidence; separate actual-worker test proves source reader closure. |
| P202 ZIP read/create | Native complete-source-set acquisition activates worker container routing; actual new-family creation, production Browse/Test/native Extract and independent stock7Zip extraction/byte comparison PASS. Uppercase first member actual Browse PASS. | NEW families only; no existing-family overwrite or set-wide atomicity. |
| P202 7z read/create | Protected encoded-length-bound source volumes, existing7z encoder + continuous encoded segmentation; actual Browse/Test/native Extract and independent stock7Zip exact bytes PASS. | Small plain nonsolid fixture, not every encrypted/solid/resource combination. |
| P202 RAR4/RAR5 read | Genuine independent RAR4three-volume20111bytes and RAR5solid four-volume117398bytes through actual Test/Extract with independent hashes; later missing member rejected. Modern part01 width and old-family source routing are implemented. | Actual independent product data uses modern part families; old .rar/.r00 family naming is source-reviewed, not a separate real old-family fixture acceptance. Never RAR creation. |
| P202 publication facts | Bounded native first/final/surplus-family preflight before effects; no-replace each member after ALL candidate readback/flush/validation. Real later-final-target race retains earlier published volumes and foreign final target. Native identity/publication/cleanup survives candidate removal and actual Dispose. Encoder spool excluded. | Partial publication is real, not rolled back or described as atomic. |
| P203 actions | Add files/directories, delete, rename, create directory, edit archive comment all through original public Job/worker/native boundary. Independent .NET read confirms exact final names/bytes; stale browser source refused. | Ordinary single-file ZIP only, including unchanged encrypted records; unsupported structures refuse before publication. |
| P203 record preservation | Separate local/central time extras, attrs/comments, unchanged encoded/encrypted records/descriptors; AES/ZipCrypto rename keeps packed bytes. Tiny forcedZIP64 edit and independent read PASS. Unknown dependent extra refuses without source changes. | Finite known-safe extra allowlist, not arbitrary metadata support. No large ZIP64/count test this round. |
| P203 outer file metadata | Actual product red reproduced lostMOTW/Hidden after replacement; native edit-only restoration turns green. Uses bounded fixed Zone.Identifier owner and ordinary DOS bits0x27. | No arbitrary ADS/ACL/filesystem-managed flag or source last-write-time preservation claim. |
| P203 additions encryption | Existing ZIP writer Deflate/plain files and Stored directories. Existing encrypted records are preserved, not re-encrypted. Existing Form/Field description explicitly warns additions are not encrypted. | No automatic inherited/new-entry encryption claim. |
| UI/FIFO/lifecycle | Current0.8.0official Form/Field/Input/Textarea/Root and approved details ListItem virtual-list owner; stable original ordinals, draft retention, original queue, exact generation/tickets, deferred callbacks and matching-browser refresh. Final archiveDesktop75PASS. | Real visual/input/DPI/UIA/user acceptance NOT RUN; tests/imports do not certify it. |

Source entry points: zip_container.rs, zip_edit.rs, worker_zip_edit.rs, worker_volume_output.rs,
worker_codecs/volumes.rs; ArchiveVolumeSources, ArchiveSourceLease, ArchiveOutputSession,
ArchiveWorkerOutputBridge.PhaseE, ArchiveWorkerRunner; original archive browser/draft/forms/jobs.
Rust remains state/layout/codec owner, C# Windows owner, GPUI rendering owner.

## 2. Final executed checks

All Cargo commands checked the shared lane and used jobs1/incremental0/codegen-units256. No duplicate
Cargo/foreign-process termination. Unique logs/append-only commands.jsonl are retained at:
`C:\Users\Administrator\.pi-desktop\scratch\7c2ebec3-f1a7-4623-9ed1-760f9adb94f5\phase-e-delivery-20261022-130740`.

| Check | Actual result | Log |
| --- | --- | --- |
| Small Archive public/worker boundaries |28PASS: engine14 + ZIPboundary2 + edit6 + editworker1 + volumes5; engine100k explicitly skipped1, no other filter/ignore | convergence-small-archive.log |
| Protocol/IPC |65/65 and38/38PASS | convergence-protocol-ipc.log |
| Archive + Protocol strict Clippy isolated-codecs lib/bins/tests |PASS, no suppression | settled-archive-protocol-clippy.log |
| Workspace --tests compile |PASS | settled-workspace-tests-check.log |
| Desktop archive production scope after final narrow fixes |75PASS/0FAIL/0ignored,923filtered(current concurrent suite998total) | closeout-desktop-archive-tests.log |
| C# isolated host |BuildPASS/0warnings/0errors | final-compatibility-host-corrected.log |
| Final Phase E exact same-set LIST→RUN |LIST11=RUN11=PASS11,SKIP0/FAIL0/NOT_RUN0;5direct handoff +2direct source +4actual product fixtures | settled-delivery-phase-e-{list,run}.log |
| Final original compatibility exact same-set |LIST6=RUN6=PASS6,SKIP0/FAIL0/NOT_RUN0; creation overwrite, native output, conflict/global admission/cancel, latepublication and actual7zworker commit lifecycle | settled-delivery-compatibility-{list,run}.log |
| Matching worker after feature checks |BuildPASS; SHA256 before/after same84557E7DA99FE7372EC1030C8CCD92CCF7151340C33D2E88C5C602E439E52A8C | settled-matching-worker.log; settled-delivery-worker-{before,after}.json |
| Scoped rustfmt Archive/DesktopArchive/ProtocolJob/IPCJob |PASS after final narrow changes | closeout-scoped-format.log |
| GPUI release/catalog + spec/task navigation |PASS; task validator only warns large reference injection truncates (explicit reads required); documentation/source only | closeout-documentation-gates.log |

Independent stock7Zip26.00 oracle SHA256:
`4BCC005758C5E604C0AAA257B6097846A4A708E62A9C0C7471FA360FAD863F1F`.
RAR fixtures retain libarchive79c571f845dbdd250d69bff9d93ae034f674135e, COPYING, .uu originals,
decoded volume hashes/manifests and independent extraction outputs under rar-independent.
These are independent oracles/data, not product executable backends.

## 3. Integrated failures / NOT RUN — not waived

- **Exactly ONE complete Desktop run:994PASS/1FAIL/0ignored/0filtered (995tests),119.95s**,
  final-full-desktop.log. Failure:
  ui::kit_migration_tests::queue07_text_form_edits_saves_temp_worker_and_restores_focus,
  kit_migration_tests.rs280, real interactive Markdown input semantics. All74archive tests in THAT
  run passed. Later75archive-only runs do not turn this full result green; no second full suite.
- Desktop full-dependency strict Clippy remains FAIL at concurrent
  vendor/gpui-kit-markdown-editor-010/src/serialize.rs215(nonminimal_bool),
  settled-desktop-strict.log. Desktop --no-deps strict additionally exposed two Archive lints;
  both fixed and retested. Final own-code check now fails only concurrent clipboard.rs3351/3944
  items_after_test_module (closeout-desktop-own-strict.log). No suppression/vendor edit by this task.
- Global cargo fmt --all -- --check FAIL on concurrent Clipboard/UI/QuickLaunch/Core/Download/
  Markdown files (settled-workspace-format.log). Archive scoped format PASS. Do not label globalfmtPASS.
- Static lifecycle gates: clipboard-lifecycle,desktop-shell-lifecycle,ui-state-lifecycle PASS;
  clipboard-ui-lifecycle FAIL on expected Textarea::new(&self.edit_input) under concurrent Markdown
  migration. This task neither changes that gate nor certifies the new editor.
- Sole optional bounded read-only reviewer was cancelled after failing to return a final report;
  review is INCOMPLETE, not independent acceptance. No second review/debate cycle was started.
- Real GUI/Shell/physical input/DPI/mixed-monitor/UIA/user acceptance remains **NOT RUN** under
  existing safety restrictions. No launch/self-app bypass.
- Earlier Phase A/B,JOB07/SAFE08 strict source recycling and A–D live/native/user obligations remain.
  OptionalP204/P205,F,packaging/install/signing/registry/service/source recycle/commit/push not run.

## 4. Preserved red/failed attempts and narrow corrections

Historical native32(1PASS/2FAIL),case defect and independent reports remain byte-preserved history.
This round retains uppercase-red,zip-edit-red,metadata test compile failures,initial Clippy findings,
worker/source/signature/owned-root/volume-password fixture attempts; none overwritten by green logs.
New journal red reproduced disappearance of unpublished failed-volume facts; retained history and
postDispose snapshot turn it green. New MOTW red reproduced loss after actual edit; restoration green.
Final compatibility first attempt1PASS/1FAIL/4NOT_RUN was a hardcoded parent-relative worker location
in ArchiveJobTests under isolated scratch output. Fixture now prefers explicit absolute
NXTOOLS_ARCHIVE_WORKER_FIXTURE and retains old fallback/original OwnedPackage admission. Corrected
same6 and finalsame6 allPASS. No production safety guard weakened.

Late source self-check found pre-candidate edit failures with no commit_state incorrectly blocked
retry. Existing terminal branch now permits zero-published/cleaned known precommit facts only, leaving
unknown/published blocked. New actual form/button/queue regression PASS; attempted red execution was
blocked first by test-context compile error, then concurrent QuickLaunch syntax error. Do not claim
an observed behavioral red for this last test. Final75scope includes it. FullDesktop was NOT repeated.
Two final Archive UI lint fixes only collapse an if and pass existing window/generation/snapshot as
one tuple; no selection/lifecycle/authority change. Final75scope passes after them.

The prior round's accidental100k run remains disclosed; this convergence explicitly filtered the
known test and verified14PASS/1filtered. No new100k/large/performance matrix was executed here.

## 5. Change boundary and stop

Final per-file audit: opening506,unchanged461,changed45,11new task files; Cargo.toml/lock are explicitly
classified concurrent work. Five selected historical PhaseD/E outcome/acceptance hashes unchanged.
Exact paths/hashes are retained in closeout-per-file-audit.json; no cumulativeHEAD ownership claim.
Opening per-file baseline506files and opening-status.txt remain in scratch. The current audit records
changed/opening hashes plus task-created files; cumulative dirty HEAD is not used as this round's
ownership proof. Concurrent manifest/lock/Markdown/Clipboard/QuickLaunch/Download work is preserved,
not reverted or folded into this archive feature. No commit/push or privileged/system mutation.

P202/P203 production paths and bounded native evidence are delivered, but **Phase E is NOT fully
accepted**: integrated failures above and independent/live acceptance remain. Stop at Phase E
independent acceptance/integration checkpoint, not F and not first-release completion.
