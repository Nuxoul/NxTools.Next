# REF-19 — EGG POSIX kinds and short authentication metadata

Status: **slice GREEN / not accepted**. B7 remains in progress; REF-20 is pending.

## Capability

| Profile | Independent prerequisite | Actual product result |
| --- | --- | --- |
| Ordinary EGG Deflate with POSIX metadata | Unchanged pinned CC0 Android `posix.egg`; three files/98bytes, strict raw-zlib EOS/consumption and format CRC | Worker Browse/Test/extract and production Read v2 → Test Job → Browser-bound native extraction preserve every byte and exact Unix mtime |
| Windows/POSIX type identifiers in both orders | Project-owned specification fixture; six entries: File, Directory, two Links, two Specials/FIFOs | Exact catalog kinds; only the ordinary file is decoded/materialized,39bytes; native directory and four truthful special skips |
| AES128/256 and LEA128/256 short salt-plus-verifier metadata | Four project-owned specification fixtures; exact11/19-byte records, mandatory End, independent PBKDF2/cipher/raw-zlib/size/CRC | Browse/Test/extract and production Read/Jobs return UnsupportedEncryption before catalog/output, with or without an available password |
| Invalid short length, End or truncation | Twelve owned structural mutations | CorruptArchive, no catalog/output, actual resource exit |

The POSIX source README identifies its Android producer and dedicates the sample directory to CC0-1.0. The genuine record carries mode0100660, uid10121, gid1023 and Unix mtime1785088885. Native metadata policy restores the timestamp; this slice does not claim POSIX permission/uid/gid restoration.

The six qualification inputs contain13catalog entries. Independently verified ordinary-file contents total2,297bytes, including four540-byte short-profile payloads that the product deliberately refuses. Successful native output is separately **four files/137bytes** across the genuine POSIX and mixed-kind fixtures. Short records lack a complete HMAC and are not an authenticated extraction capability or genuine producer-profile claim.

## Ownership and implementation

The only product source changed in this slice is `rust/crates/nxtools-archive/src/worker_alzeg/egg.rs`:

- Accumulate kind identifiers from both metadata headers. Special and Link cannot be negated by a later regular header; Directory/File retain original materialization rules.
- Recognize only exact short AES/LEA declarations ending at the mandatory End marker, then return UnsupportedEncryption. Malformed boundaries remain CorruptArchive.
- Preserve the original full authenticated21/29-byte and sampled full-plus-seven path, raw names, ordinals, finite metadata admission, native metadata restoration, password preparation, source leases, selected output quotas and output summaries.

Original `worker_formats` File-only iteration, native directory output, skipped-special facts and source-deletion ineligibility remain authoritative. There is no replacement codec/runtime, dependency, protocol, native authority or UI change. Existing operation/correlation lifecycle, elapsed and native-code diagnostics cover this slice without new member/payload logs.

Tests extend the original `ref19_alzeg_contract.rs`, `ArchiveAlzegProductTests.cs` and four reviewed Fixture registration lines in `Program.cs`. The fixture import adds20new paths: six archives,13independently decoded gzip envelopes and one manifest. The eleven existing slice-before copies are attribution witnesses, not a replacement for the original B7 baseline.

## Verification

| Boundary | RED | GREEN |
| --- | ---: | ---: |
| Real isolated Worker, identical71-test selection and inputs | 69PASS / 2FAIL | 71PASS / 0FAIL |
| Two new production Read/Job selections, identical final C# tests | 0PASS / 2FAIL | 2PASS / 0FAIL |
| Final production/password selections | — | 25PASS |
| ZIP crypto/method/boundary preservation | — | 5 / 13 / 2PASS |
| Native complete-source preservation | — | 6PASS |
| Prior ZIP/7z source and Job shortfall preservation | — | 3PASS |

Total successful GREEN executions: **125**, not a whole-B7 acceptance count. Production authority comprises15completed main-ledger rows and10unique tail-ledger rows. Every selected behavior uses nonzero LIST and same-filter RUN; ignored/SKIP/not_run is zero. Matching Worker builds and C# builds pass; C# reports zero warnings/errors.

The six new Worker tests cover genuine payloads, order-independent kinds, four typed short profiles, malformed short framing, original finite metadata/working/entry/output budgets, actual first-write cancellation and native first error. Every child is waited, stderr/pipes are drained, temporary output tokens are released and sources can be removed after terminal.

Production tests verify exact catalog ordinals/kinds, byte-exact native output and POSIX mtime, the original succeeded_with_warnings/four-skips result, no link/FIFO materialization, no short-profile password prompt/output, protected sources through before_cleanup, actual cancellation/native error112 and exclusive source reopen/owned-tree deletion.

Five scoped gates pass: Archive all-target check with isolated-codecs, strict all-target isolated Clippy, strict default-library Clippy, two-file rustfmt and the specification-contract gate. No lint suppression or unrelated repair is included.

## Evidence

Immutable `evidence/ref19/egg-posix-kinds-short/` contains **573files excluding files.txt**. It retains independent qualification inputs/results, RED/GREEN commands/environment/logs/exits, production ledgers, preservation and quality results, four compiled-source witnesses, current fixture bytes, eleven slice-before inputs, scoped changes and the original B7 baseline record. `audit.json` records125successful executions and the frozen empty-lane resource observation at2026-11-12T21:04:50.5373809+08:00, with25,784,119,296available commit bytes.

Authoritative scratch prefixes are `b7-ref19-kinds-short-{red,green}`, `b7-ref19-kinds-short-product-{red,green,green-tail}`, `b7-ref19-kinds-short-preserve-*`, `b7-ref19-kinds-short-{native,prior}-preserve` and `b7-ref19-kinds-short-quality-r1`. Helpers and exact commands are retained with the evidence.

## Limitations

Locale/relative names, encrypted names/global headers and unqualified method/encryption/volume combinations retain separate capability rows. Mixed-kind and short-header specimens are specification evidence, not additional producer samples. Physical UI, installed/Release and independent acceptance are not established by this slice. Parent phase names/order, B1–B6 acceptance and existing frozen evidence remain unchanged.
