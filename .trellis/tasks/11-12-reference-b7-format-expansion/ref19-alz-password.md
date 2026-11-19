# REF-19 — ordinary ALZ traditional encryption

Status: slice GREEN; B7 remains `in_progress / not_accepted`.

## Capability

The original Browse / Test / extract chain reads ordinary ALZ Stored/Deflate with traditional ZipCrypto. Three unchanged CC0 inputs from the pinned unalz sample directory contain13members/140,748decoded bytes. CPython struct/zipfile/raw-zlib independently validates complete physical records, packed extents, encryption headers, decoded sizes and format CRCs. Expected contents are retained as gzip envelopes, not product-generated output or added member digests.

| Sample | Method | Members | Exact decoded bytes |
| --- | --- | ---: | ---: |
| `enc_store.alz` | Stored | 1 | 3,600 |
| `enc_deflate.alz` | Deflate | 2 | 3,662 |
| `zip20.alz` | Deflate plus unencrypted empty entry | 10 | 133,486 |

Clear catalog browsing retains actual encrypted flags, nested paths, empty files and automatic CP949 name provenance. Test and extraction require payload verification. Native output equals every independently decoded byte; no archive member or reference application is executed.

## Original ownership

`formats/zip_crypto.rs::Classic` remains the bounded, zeroizing traditional-key reader. Its explicit password-byte/check-byte constructor serves ALZ; ZIP keeps UTF-8 derivation, packed-inclusive-header coordinates and original seek behavior. ALZ encodes passwords as CP949 without lossy substitution and accounts for its separate twelve-byte header. No encoded member or plaintext is buffered as a whole in product code.

`worker.rs::job_password_format` and `worker_password::execute` retain the original finite read-only preparation, private prompt ticket, cancellation and exactly one subsequent extraction. The same native source/helper leases remain protected while awaiting input and through actual worker/pump cleanup. Job/Browser binding, output limits, first native error and publication owners are unchanged. ALZ uses this preparation route like the existing signature-routed 7z/RAR cases.

A missing password is PasswordRequired. The single-byte traditional verifier is probabilistic: mismatch or encrypted payload decode/size/CRC failure is AuthenticationOrDataError, not definitive WrongPassword. Ambiguous failure does not automatically retry. Cancel/resource/source/native-write outcomes retain their original types.

## Boundary evidence

| Selection | Nonzero LIST / RUN | RED | GREEN |
| --- | ---: | --- | --- |
| Real worker `ref19_alzeg_contract` | 36 / 36 | 30PASS/6FAIL, Cargo101 | 36PASS, Cargo0 |
| New production Read/Browser/Job cases | 2 / 2 | 0PASS/2FAIL | 2PASS |
| Original plain production/readmission/password preservation | 12 / 12 | — | 12PASS |
| ZIP crypto/method/boundary contracts | 5+13+2 | — | 20PASS |

Worker and new production tests are byte-identical across their authoritative RED/GREEN. Final selections total70successful executions with0FAIL/ignored/SKIP/not_run. Production cases exercise actual private password replies and stale-ticket rejection, exact native output, missing/wrong/corrupted-ciphertext/cancel outcomes, zero preparation writes, protected source/helper handles and exclusive reopen/owned-tree removal after terminal cleanup. The original RAR password fixture admits its366-byte source with a1MiB output ceiling while retaining its100-byte member limit and unchanged password assertions.

Archive all-target check and strict Clippy with isolated-codecs, default-library strict Clippy, seven-file rustfmt check and the source/link/structure documentation gate pass. Actual C# test-host builds succeed with zero warnings/errors. Both explicitly matching Worker builds succeed. These are source/owned native pipeline results, not physical UI or Release acceptance.

Raw commands, environments, counts, logs, independent inputs, source snapshots and scoped diffs are frozen under [ALZ password evidence](evidence/ref19/alz-password/files.txt). Shared Program ownership in this slice is only the two explicitly reviewed Fixture registrations; other owners' changes are preserved. Source/sample licensing and the exact public call chain remain in [source review](ref19-source-review.md) and [archive formats](../../../docs/reference-code/archive-formats.md).

## Limits

Encrypted split sets, non-ASCII password fixtures, descriptor-time verifier fixtures and encrypted combinations with unfinished methods have no acceptance claim. ALZ modified Bzip2, EGG AZO/solid/crypto and remaining name profiles stay separate pending rows. REF-20 is pending; parent phase and B1–B6 acceptance are unchanged.
