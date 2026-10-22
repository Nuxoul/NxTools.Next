# Independent acceptance — ZIP AES and boundary remediation

## Verdict

**PASS for the delivered ZIP read/encryption behavior slice at its documented interoperability/fixture scope.** Original AES red, directory payload validation defect, EOCD comment metadata defect and archive formatting issue are resolved. No new definite in-scope correctness/security defect was found in one directed source review. **Phase A / A–F first release remain incomplete.** No product source changed in this acceptance.

This is not certification of every encrypted-method cross-product, arbitrary large archives, storage platforms or release operation. AES creation/general archive creation/source traversal were not delivered by this slice.

## Independent verification actually executed

An unrelated `cargo test -p nxtools-desktop --locked --offline` was active initially. This review waited until the Cargo/rustc slot became free before starting serial archive verification; no process was killed. All Cargo commands inherited CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256. No timeout or duplicate review build.

| Check | Actual current review result |
| --- | --- |
| cargo test -p nxtools-archive --features isolated-codecs --offline --no-fail-fast |77passed/0failed/0ignored,exit0; full log summed independently |
| Original independent AES AE1/AE2 strengths test |PASS unchanged within crypto5/5 |
| Directory/comment boundary tests |2/2PASS |
| ZIP methods including independent noEOS LZMA |13/13PASS |
| Same-feature actual worker build |exit0 |
| Archive fmt |exit0 |
| cargo fmt --all -- --check |**exit0 now**, not the earlier report's failure; review made no formatting writes |
| Strict archive Clippy including tests |exit0 |
| cargo check --workspace --tests --offline |exit0; one unrelated Desktop unused import warning in quick_launch/file_dialog_search.rs:33 (COMPACT_RESULT_TRAILING_RAIL_WIDTH); not workspace tests/strict whole-workspace lint |
| C# test-host build --no-restore |exit0,0warnings/0errors |
| Exact original50+new4 C#union |list selected54/run0;execute54/run54/pass54/skip0/fail0/not_run0,exit0 |
| Extensions archive_import / Clipboard --lib archive_ |7/7 and5/5PASS (39clipboard filtered) |
| spec-contracts / catalog / task context / fixture selector |PASS101docs/14owners;79pages/79rows/78locked entries;26implement/23check,no truncation;source selectorPASS |

Current checkpoint's18filters and explicit worker/codec/ZIP/Python-oracle environment used. The reported339 worker operations are internal fixture-loop counts, not339independent test cases; no extra total is claimed beyond54harness cases.

## Original independent defect probes re-executed

Reused scratch-only zip_read_boundary_probe.py, actual worker with unique synthetic CPython files and bounded private protocol. Results:

```text
EMPTY_ZIP_COMMENT_METADATA_LIMIT_1 -> failure ResourceLimit, no native calls
BAD_DIRECTORY_TEST -> failure CorruptArchive
BAD_DIRECTORY_EXTRACT -> failure CorruptArchive, no native calls
```

Thus the exact prior observations changed from success/budget bypass to typed rejection. Probe acknowledges native requests but does not create actual native output; real C# directory/comment boundary fixture is separately in54PASS.

## Source/coverage checks

- `zip.rs` charges combined central retained estimate +4×raw EOCD comment +fixed comment overhead before copying comment and uses it for projection/admission; lower-budget snapshot execution rechecks.
- `planning.rs` retains selected explicit directory IDs. `zip.rs` validates these records before directory effects; implicit parents have no invented payload and unrelated unselected directory payload is not decoded.
- `zip_aes.rs` rejects missing/duplicate/malformed/contradictory AES extra metadata, validates vendor/version/strength/overhead and unsupported encryption; local metadata must match central original records.
- Operation-private password and checked bounded input; PBKDF2-HMAC-SHA1/1000, correct key/salt lengths, constant-time verifier and truncated HMAC check over complete ciphertext including empty ciphertext before decode/output. AES CTR little-endian counter1 and seek offsets inspected. No plaintext spool/whole-entry Vec.
- AE1 content CRC retained; AE2 CRC exception scoped to its version; all paths still require exact size/consumed input, supported method, source recheck/cancel before commit and owned abort on failure. Native source/parent/output/overwrite contracts unchanged.
- Verified matrix: AE1/AE2×128/192/256 with Store/Deflate; AE2AES256 and ZipCrypto with0/8/9/12/14/93/95. Other AES strengths/versions with extra methods are composed but not independently proven as all combinations. Oracle differences for encrypted XZ empty stream remain explicitly documented rather than counted as successful7-Zip whole-archive validation.
- True noEOS LZMA independently produced by stock7-Zip eos=off and checked by public/native tests, not a flag-only mutation. Directory MAC, damaged verifier/cipher/MAC, framing/extra faults, ZIP64/descriptors, budget/cancel/IO/source and partial cleanup cases present.

One directed independent read-only AES review found no concrete in-scope defect; no repeat broad review was requested. This is source inspection plus executed fixtures, not proof of cryptographic implementation correctness for every adversarial input.

## Scope and continuation

The development report's whole-workspace fmt failure was truthful historical evidence but is **not reproducible on current source**: both archive and workspace fmt now pass. An unrelated pre-existing formatting difference should not by itself require stopping independent approved archive source work; preserve others' changes, report global check status and coordinate build ownership. No need to reopen overwrite architecture or request repeated Phase A authorization.

Next: save this green ZIP baseline and continue approved general creation/source traversal (ZIP incl.AES256,7z,TAR/compound), then native attrs/MOTW/time/guarded source deletion, product IPC/capability/package mapping/backpressure/diagnostics and broad resources/performance. Do not mark Phase A passed or start B before its canonical exit.

Not run: unrestricted/default C#suite, full workspace tests/strict workspace lint, UI/native-user Shell/DPI/UIA, FAT/SMB/UNC/old systems, crash/power loss, release/install/service/registry. No user archive mutation, commit/push/package/install performed.

Review archive commands had all returned. Final process inspection observed a **different** `cargo build -p nxtools-desktop --locked --offline` and its rustc; left untouched, no additional Cargo started. No archive worker/test-host/fixed helper remained. Scratch evidence: zip-aes-acceptance-{archive,worker,archive-fmt,workspace-fmt,clippy,workspace,extensions,clipboard,native-list,native-tests}.log. Existing scratch boundary probe was reused; no project test/source edited.
