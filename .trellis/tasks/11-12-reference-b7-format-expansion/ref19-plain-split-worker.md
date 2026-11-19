# REF-19 plain split Worker outcome

Status: Worker slice GREEN; REF-19/B7 remain in_progress / not_accepted.

## Verified capability

| Genuine CC0 family | Physical volumes | Files | Exact decoded bytes | Cross-volume payload |
| --- | ---: | ---: | ---: | --- |
| ALZ split_store | 3 | 4 | 160,000 | Two Stored members cross one boundary each |
| ALZ split | 8 | 10 | 10,487,896 | The10,485,774-byte Deflate member crosses all7boundaries |
| EGG split_store | 3 | 4 | 160,000 | Two Stored members cross one boundary each |
| EGG split | 8 | 1 | 10,485,774 | The Deflate member crosses all7boundaries |

Independent CPython struct/zlib validation precedes behavioral RED. It checks all repeated ALZ version/ordinal headers, exact nonfinal CLZ03/final CLZ02 framing, EGG unique Header IDs and reciprocal links, continuation-prefix End, physical header-group ranges, declared sizes and format CRCs. Stored contents are identical across ALZ/EGG; both large Deflate contents are byte-identical. The checked-in inputs remain unchanged CC0 samples, not cuts of an ordinary archive. Expected CC0 contents use gzip envelopes with one retained copy for shared bytes. No archive member or reference application is executed.

## Original owners

`worker.rs::execute` obtains the source set only from the original source_acquire reply; startup JSON cannot authorize volume_paths. `worker_formats::run` uses the existing `worker_codecs/volumes.rs::SourceSet` for the ordered all-source revision and verification. `worker_alzeg/volumes.rs` validates actual framing and streams only admitted encoded ranges into an anonymous spool with one64KiB buffer; ordinary sources remain direct inputs. EGG retains physical boundary coordinates and rejects header groups crossing them. The original ALZ/EGG record readers and strict Stored/Deflate payloads consume this logical input.

Browse/Test/extract still use original metadata, path policy, immutable entry ordinals, MemberWriter, output tokens and source checkpoints. Continuation revision checks precede controlled output commit. First native error and cancellation keep their categories. No new discovery, source hash owner, codec dependency, whole-input buffer, output controller or Job runtime is introduced.

## Boundary regression

```text
cargo test --locked --offline -p nxtools-archive --features isolated-codecs
  --test ref19_alzeg_contract -- --list
cargo test --locked --offline -p nxtools-archive --features isolated-codecs
  --test ref19_alzeg_contract -- --nocapture --test-threads=1
```

Final identical test/fixture bytes select and run27tests. With the original multi-volume rejection in place:15PASS/12FAIL, Cargo101. With the acquired ALZ/EGG multi-volume path enabled:27PASS/0FAIL, Cargo0. Both have0ignored/not_run. The earlier ordinary15tests remain successful.

The selection covers exact catalog/names/counts/all bytes, omitted last/middle volumes despite still-present unleased siblings, reordered sets, damaged repeated headers/continuation markers, cross-volume CRC failure before commit, EGG file headers crossing a physical boundary,16finite-budget cases, real cancellation frames, native pipelined-write failure and a continuation changed after spool read but before commit. Actual temporary-file tokens are read back before controlled publication; unfinished outputs are closed/removed only through the original abort/parent-after-drain lifecycle. Every worker is actually waited, stderr drained, and all source files removed after exit.

Raw commands, nonzero LIST/RUN counts, exits, source/test snapshots, independent oracle, licensed fixture provenance and resource observations are retained in `evidence/ref19/plain-split-worker/`.

## Scoped checks

On the GREEN source, archive all-target check and all-target strict Clippy with isolated-codecs both exit0. Seven-file rustfmt --check exits0. The documentation contract gate passes129documents/14workspace owners, exit0. These are interim Worker-slice checks, not final B7 cross-layer preservation or native acceptance. The139-file evidence manifest matches the retained file set; all22sample bytes still equal their pinned CC0 sources.

## Limits

This is original Rust worker plus controlled actual-file peer evidence. C# complete-family leasing, production Read/Browser/Job/password routing and public CP949 provenance validation remain unfinished. Remaining ALZ modified-Bzip2, EGG AZO/solid/crypto and separately recorded unsupported variants are not accepted by this slice. Physical UI, installed and Release acceptance remain separate.
