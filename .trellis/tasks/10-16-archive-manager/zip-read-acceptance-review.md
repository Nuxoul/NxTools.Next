# Independent acceptance — ZIP extra methods / ZipCrypto partial delivery

## Verdict

**Requested ZIP read/encryption slice NOT ACCEPTED as complete. Phase A / first release NOT complete.** Delivered plain9/14/95/93 and ZipCrypto+Deflate have real passing regression evidence, but AES remains intentionally unimplemented/red and zip.rs fmt fails. Two additional concrete bounded validation defects below were independently reproduced. No product source was modified by this acceptance.

## Independently executed checks

All Cargo inherited CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256; serial, no timeout/duplicate build.

| Check | Actual result |
| --- | --- |
| archive tests --features isolated-codecs --offline --no-fail-fast |71pass/1fail/0ignored,exit101; counts summed from complete log |
| Known AES test | independent_aes_ae1_ae2_all_strengths_require_authentication_and_exact_bytes fails at zip_crypto_contract.rs195: actual UnsupportedMethod, expected PasswordRequired |
| fmt --all -- --check |exit1; diff paths uniquely identify src/formats/zip.rs |
| Same-feature worker build / strict archive Clippy --tests / workspace check--tests |exit0 each; check not workspace tests |
| C# build |exit0,0warnings/0errors |
| Exact original44+ZIPmethods4+ZipCrypto2 |list50/run0;execute50/run50/pass50/skip0/fail0,exit0 |
| Extensions archive_import / clipboard --lib archive_ |7/7 and5/5PASS (39clipboard filtered) |
| spec-contracts/catalog/task context/fixture selector |PASS101docs/14owners;79/79/78;25implement/22check,no truncation;source selectorPASS |

No full/default C# suite, UI/DPI/UIA/live Shell, arbitrary user archive, native system-state mutation, release/install/service/registry/commit/push performed. No remaining owned worker/test-host/Cargo/rustc observed; fixed test helper absent.

## Known incomplete requirements (not hidden regressions)

- AES is not connected: zip.rs415 supports only0/8/9/12/14/93/95, and zip_crypto.rs59-60 rejects99. Archive manifest retains no newly proposed AES adapter dependencies. Its independent AE fixtures/red remain. Error means unfinished dispatch/authentication, not a proved cryptographic algorithm failure.
- No-EOS LZMA positive interoperability sample and encrypted-method combinations beyond delivered ZipCrypto+Deflate are not established; ledger acknowledges this.
- Rustfmt difference is formatting (long signatures/expressions/match blocks), not Rust compilation failure; Clippy/build pass does not satisfy fmt.

## Additional confirmed findings

### P2 — Unencrypted directory payload bypasses extraction integrity

`rust/crates/nxtools-archive/src/formats/zip.rs:376-378,490-504`: preflight only demands directory declared uncompressed size0, while extract streams encrypted directories only. Plain directory payload CRC/StreamEnd/method may therefore never be verified. Test streams every entry at330-339, so test and extraction disagree.

Independent CPython fixture: ZIP_DEFLATED directory `folder/`, empty data; corrupt first compressed byte to0xff without changing declared size/CRC/extents. Actual worker test returns failure/CorruptArchive. Actual worker extract returns complete and emits native_call action=directory. The scratch protocol host acknowledges that call but does not create a real native directory; this proves public extraction's accepted result/output request, not an additional C# filesystem write test.

Required regression and repair: validate appropriate directory payload integrity before creating its corresponding destination. Define legitimate zero-data directory encodings; don't reject all compressed empty directories or silently ignore corrupted/unsupported payloads. Preserve encrypted-directory authentication behavior, selected extraction policy, budgets/cancellation and avoid unbounded duplicate decode. Add public test/extract consistency plus actual native small-fixture assertions. No arbitrary dangerous file output was observed in this probe, but success on structurally corrupt directory content violates the stated full-integrity behavior.

### P2 — Empty ZIP comment is outside metadata reservation

`zip.rs:140-155,282-295`: central retained estimate counts records and central bytes, but EOCD archive comment is copied to Vec then a lossy String without charging comment length/expansion to max_metadata_bytes. A valid empty ZIP with65535-byte ASCII comment, request max_metadata_bytes=1 and normal working budget opens successfully (worker complete/files0).

Required: charge retained raw/comment decoding budget before copy/String allocation and include it in the combined entry/projection/retained metadata estimate; don't double-use independent full budgets for simultaneous retained objects. Add empty comment-only ZIP exact/under-budget cases, non-UTF8 lossy expansion and regular-central+comment combined budget tests. ZIP comment size is bounded by format65535, so this is a finite budget-contract bypass, not proof of unlimited allocation.

## What the reported edit error means

Report records three per-path failed Edit calls in one user turn: malformed deletion syntax, EDIT_LINES_UNSEEN, then `EDIT_RANGE_INVALID: two ops target the same insert gap (index414)`. The last is an edit payload conflict, not Rust line414 compilation failure or AES algorithm failure. Two operations normalize to the same insertion location; tool rejects before mutation. Without raw rejected payloads this review cannot identify the exact two headers; the report and current file support the unfinished integration, not independently verified failure-count provenance.

The policy requires stopping after three failed edits on that path in that user turn. Stopping then was compliant; repeatedly producing invalid edits is an execution-quality problem. The restriction is not a permanent file lock: in a fresh authorized development turn, read the live file, use current tags/visible ranges, merge same-gap edits and continue. Do not bypass via shell/delegate/formatter during the stopped turn. No product architecture or scope decision is needed to resume AES.

## Next bounded work

Fresh grounded continuation: finish AES extra/version/strength/local-central validation, borrowed private password, authentication before publication and AE1/AE2 CRC semantics with independent public/native fixtures; fix the two reproduced validation defects; run formatting normally after permitted edits; retain all existing tests and rerun full suite/native union. Do not merely change AES expected PasswordRequired to UnsupportedMethod to green the test. Full ZIP slice must converge before general creation/traversal. Don't reopen already approved overwrite contract or treat this tool stop as a lasting technical blocker.

Evidence: review scratch zip-read-acceptance-{archive,worker,fmt,clippy,workspace,extensions,clipboard}.log and zip_read_boundary_probe.py. One independent read-only directed review proposed the two defects and parent confirmed both via bounded synthetic probes. No repeated broad reviews. Test counts above are baseline suite evidence, not passing acceptance of the reproduced missing regressions.
