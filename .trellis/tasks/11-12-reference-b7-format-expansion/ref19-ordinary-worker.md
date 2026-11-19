# REF-19 — ordinary ALZ Deflate and EGG methods

Status: ordinary Worker slice GREEN; REF-19 remains in progress / not_accepted.

## Capability

The original isolated worker now reads ordinary ALZ Stored/Deflate and ordinary non-solid EGG Stored/Deflate/Bzip2/LZMA through Browse, Test and capability-output extraction. EGG empty files without blocks and mixed per-file Stored/Deflate archives retain their original catalog and exact content. Each EGG block has its own declared expanded/packed interval and CRC; the adapter validates all section ends and retains strict decoded size and compressed-consumption checks. LZMA uses the existing bounded dictionary/model and known-size decoder with the EGG-specific opaque four-byte prefix.

No dependency or Cargo lock change is required. `worker_formats` remains the dispatch/output owner; `worker_alzeg` and its private `egg` module adapt container records. Existing ZIP stream state is shared through crate-private entry points, without synthetic ZIP metadata. ZIP keeps its original properties-size requirement, EOS flags and work counters.

## Independent sample prerequisite

Seven unchanged CC0 samples from the pinned unalz/unegg revisions were parsed and decoded independently with CPython zlib, bz2 and lzma. All27members/8,147,005bytes match their declared block/member sizes and format CRCs. ALZ Deflate and EGG Stored/Deflate/mixed/LZMA yield the same5members/9,401bytes. The8,100,000byte Bzip2 member also equals the fixed45-byte sentence repeated180,000times. Four small raw expected payloads are retained beside their fixture provenance. No member is executed and no added file digest is used.

## Original Worker RED / GREEN

Both commands use the locked offline `nxtools-archive` package, `isolated-codecs` feature and `ref19_alzeg_contract` target, with the same whole-target selection:

```text
cargo test --locked --offline -p nxtools-archive --features isolated-codecs --test ref19_alzeg_contract -- --list
cargo test --locked --offline -p nxtools-archive --features isolated-codecs --test ref19_alzeg_contract -- --nocapture --test-threads=1
```

| Run | LIST | RUN | PASS | FAIL | ignored / not_run | Cargo exit |
| --- | --- | --- | --- | --- | --- | --- |
| RED | 8 | 8 | 1 | 7 | 0 / 0 | 101 |
| GREEN | 8 | 8 | 8 | 0 | 0 / 0 | 0 |

The existing Stored regression passes in RED. New ALZ Deflate fails at payload with UnsupportedMethod, and EGG fails at Browse with UnsupportedFormat. GREEN uses byte-identical tests and fixtures. Every successful sample checks read-only Browse/Test, exact original names/sizes/counts, all extracted bytes, actual child wait, stderr drainage and source reader release.

Frozen [evidence manifest](evidence/ref19/ordinary/files.txt) lists67files excluding itself. [Result](evidence/ref19/ordinary/result.json) retains exact selections/counts/exits, equality and unfinished capabilities. Raw commands, environment, logs, oracle prerequisite and current/RED/before source bytes are retained separately from the immutable first Stored evidence.

## Remaining capability and acceptance boundaries

These tests use the real original Rust child with controlled native replies. They do not establish C# protected-volume handles, production Read/Job/password routing, physical UI or Release acceptance. ALZ modified Bzip2, EGG AZO/solid, name profiles, passwords and actual split families remain separate implementation/evidence rows. Failure/cancel/resource controls are recorded in subsequent evidence. REF-20 is not started by this slice.
