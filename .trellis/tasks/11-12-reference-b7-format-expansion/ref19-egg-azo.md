# REF-19 — ordinary EGG AZO

Status: slice GREEN / not accepted. Overall B7 remains in progress.

## Qualified inputs

The public CC0 EGG AppendixA supplies the arithmetic, probability, history,
code-table, internal-block and x86 rules. The pinned sources and exclusions are
recorded in [source review](ref19-source-review.md). The independent Python
oracle parses physical EGG framing, decodes AZO without product code or a
reference executable, and verifies every decoded size, packed extent and format
CRC. No archive member is loaded or executed.

| Input | Origin | Files | Decoded bytes | Distinct evidence |
| --- | --- | ---: | ---: | --- |
| `azo.egg` | Unchanged pinned CC0 | 1 | 1,602 | Version1, x86 flag, arithmetic26-bit lookahead; no address needs restoration |
| `optimal5.egg` | Unchanged pinned CC0 | 5 | 9,401 | Stored/AZO/Deflate/Bzip2 mixed catalog; all bytes equal the existing five-member corpus;28-bit lookahead |
| `azo_pe.egg` | Unchanged pinned CC0 | 1 | 148,781 | 151actual x86 restorations,140,436-byte distance,24-bit lookahead |
| `spec_egg_azo_multiblock.egg` | Project-owned specification fixture | 1 | 159,176 | Three raw/compressed internal blocks, effective x86 across64KiB, fresh/reused/overlapping matches and65,539-byte distance |
| Total | Separate producer/specification origins | 8 | 318,960 | Exact independent bytes |

The specification fixture is built from literal data and explicit match tokens
by an independent arithmetic encoder. Gzip envelopes under `expected/azo/` and
`expected/azo-pe/` retain decoded bytes, not product extraction output. The
three genuine archives contain7files/159,784bytes. The largest measured
arithmetic lookahead is28bits.

## Original owners and bounded reader

Product changes are limited to private `worker_alzeg/azo.rs`, its module and
`Method` dispatch in `worker_alzeg.rs`, and the method3 mapping in `egg.rs`.
The reader uses an8KiB encoded buffer, fixed adaptive models and a circular
unfiltered history dictionary. A256KiB fixed working reservation precedes
allocation; remaining working credit bounds the dictionary, capped by the
internal block's expanded size. Valid distances beyond that credit return
ResourceLimit. Filtered bytes never replace LZ history.

Models, histories and x86 coordinates reset at actual internal-block boundaries.
The32-bit arithmetic tag permits at most32zero lookahead bits. Invalid range
states, out-of-history/overrun matches, unused packed bytes, malformed size/check
fields and missing/nonzero terminal framing return CorruptArchive. Unknown
version/flags return UnsupportedMethod. Complete size, terminal and packed
consumption feed the original `copy_checked` CRC check before native publication.

Original source revisions, native leases, immutable Jobs, password preparation,
output limits, first-error priority, tokens and cleanup remain authoritative.
No dependency, plaintext spool, complete encoded-member buffer, replacement
runtime, C# production owner, Protocol or UI change is introduced. Existing
operation/correlation, accepted/start, source transitions, terminal elapsed/native
error and cleanup diagnostics cover the slice without payload or per-block logs.

## Behavioral evidence

| Boundary | LIST / RUN | RED | GREEN |
| --- | --- | --- | --- |
| Actual isolated Worker target | 65 / 65 | 58PASS / 7FAIL, Cargo101 | 65PASS / 0FAIL, Cargo0 |
| Two production Read/Browser/Job AZO selections | Individually1 / 1 | 0PASS / 2FAIL | 2PASS / 0FAIL |
| Prior production/password selections | Individually1 / 1 | Not a new behavior pair | 21PASS / 0FAIL |
| ZIP crypto/method/boundary preservation | 5+13+2 | Not a new behavior pair | 20PASS / 0FAIL |
| Native source/prior ZIP-7z/Job preservation | 6+3 | Not a new behavior pair | 9PASS / 0FAIL |

Authoritative Worker pair is `azo-red3` → `azo-green2`; authoritative production
pair is `azo-product-red3` → the two matching `azo-product-green2` rows. Final
Rust tests and C# tests are byte-identical across those pairs. All58prior Worker
tests remain byte-preserved. The complete production/password result is22PASS
in `azo-product-green2` plus1PASS in `azo-product-green2-tail`, with23unique
expected selections. No ignored, SKIP or not_run result counts as success.

The seven additions cover genuine/mixed/PE exact bytes, internal-block and x86
state,12owned malformed/version/size/check/CRC/terminal/truncation/unused-input
cases, dictionary/output limits, real first-write cancellation/native failure,
actual child wait/stderr drain, output-token cleanup and source release. The
65,539-byte dictionary credit succeeds exactly; one byte less returns
ResourceLimit without publication.

Actual production Readv2 → TestJob → Browser-bound native extraction verifies
all8files/318,960bytes, exact catalogs and no extra files. Damaged CRC, lawful
entry limits, current Job cancellation and native write112 preserve typed
outcomes, zero commits, protected sources through before_cleanup and actual
exclusive reopen/owned-tree deletion. Only the two reviewed Fixture registrations
are added to the existing harness; no global test-effect policy changes.

Final successful GREEN executions total117:65Worker +23production/password
+20ZIP +9native/prior. Archive all-target check, isolated/default strict Clippy,
four-file scoped rustfmt and the source/link/structure documentation gate pass.
C# builds complete with zero warnings/errors, using the matching actual worker.

## Frozen evidence and attribution

[Evidence inventory](evidence/ref19/egg-azo/files.txt) lists879files, excluding
itself. It contains qualification commands/oracles, unchanged samples, independent
contents, final RED/GREEN commands/logs/exits/counts, compiled-source witnesses,
pre-edit snapshots, the generated scoped diff and the original B7 baseline record.
The collector verifies paired bytes, the expected selection set, prior58-test
prefix, unchanged preservation product sources and four added Program lines
before writing this new immutable directory.

`b7-ref19-azo-start/` contains nine existing inputs for slice attribution only;
it does not replace `b7-start/`. New task-created paths are the private AZO reader,
14fixture paths, this outcome and the frozen evidence. Earlier B7 evidence,
accepted B1–B6 and concurrent unrelated work remain outside this slice.

## Limitations

Claims cover ordinary plain non-solid AZO and the listed mixed archive. Split,
encrypted and solid AZO producer profiles remain unqualified. Remaining locale,
relative/unsafe names, special kinds, short unauthenticated metadata and other
separately unqualified format combinations retain their own capability rows.
Physical UI, installed/Release, whole-B7 convergence and independent acceptance
are not established by this slice. REF-20 and the canonical parent phase remain
open.
