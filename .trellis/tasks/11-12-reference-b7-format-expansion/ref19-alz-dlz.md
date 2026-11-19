# REF-19 — ordinary ALZ modified Bzip2 / DLZ

Status: GREEN at the original isolated Worker and production Read/Browser/Job/native-output boundaries; not accepted. REF-19 and B7 remain in progress.

## Capability and provenance

| Archive | Origin | Members | Decoded bytes | DLZ blocks |
| --- | --- | ---: | ---: | ---: |
| `bzip2.alz` | Unchanged pinned unalz CC0 sample | 2 | 4,500 | 2 |
| `bzip2_dlz.alz` | Unchanged pinned unalz CC0 sample | 1 | 3,600 | 1 |
| `spec_alz_dlz_multiblock.alz` | Project-owned specification fixture | 1 | 1,969,632 | 3 |
| Total | Separate producer/specification evidence | 4 | 1,977,732 | 6 |

The genuine inputs come from alkegi/unalz revision `30b4ad40c5452c686df69b3b45554de3073e57a1`, whose sample directory is CC0. Independent Python parses physical ALZ extents and DLZ Huffman/MTF/BWT/RLE data, verifies original member sizes/CRCs, reconstructs each block with its correct Bzip2 CRC and requires strict CPython bz2 completion. The specification fixture derives from CPython's standard Bzip2 encoder, preserves three exact block intervals and includes long RLE runs. Its complete literal contents are independently reverified. Expected bytes and origins are retained in the [fixture manifest](../../../rust/crates/nxtools-archive/tests/fixtures/alzeg/expected/dlz/manifest.json).

The original reader and Bzip2 source boundaries are recorded in [archive formats](../../../docs/reference-code/archive-formats.md) and the [REF-19 design](ref19-design.md). No upstream implementation is copied and no codec dependency is added.

## Original owners and changes

- New private `worker_alzeg/dlz.rs`: one reusable900,000-entry `u32` BWT/permutation workspace, fixed selector/canonical-Huffman tables and an8KiB encoded-input buffer. A5MiB finite working reservation precedes allocation.
- Existing `worker_alzeg.rs`: eight added lines register the private module, map ALZ method1 and route it through original `decode_payload`/`copy_checked`. EGG standard Bzip2 and other methods retain their original branches.
- Existing `ref19_alzeg_contract.rs`: six new actual-child tests; the prior47tests remain unchanged.
- Existing `ArchiveAlzegProductTests.cs`: two new production selections,35added lines. Existing `Program.cs` adds four registration lines for those reviewed Fixture tests; total B7 registrations are22.
- Eight new fixture/expected paths. Existing task, reference and provenance records are updated; original B7 baseline and prior frozen slices remain authoritative.

DLZ01/DLZ02 markers are read at the Huffman end-of-block bit position. The reader retains unaligned bit state, bounds origins/selectors/code lengths/runs, inverts BWT within its fixed workspace and streams RLE output. The original complete packed-consumption, member size/CRC, source-stage verification and native publication owners determine success. No whole packed member or expanded member is buffered by the product.

Existing operation/correlation, source/password and terminal diagnostics cover this method. There are no added payload, filename, password or per-block logs.

## Qualified RED / GREEN

| Boundary | Nonzero LIST / same RUN | RED | GREEN |
| --- | ---: | --- | --- |
| Full real Worker target | 53 / 53 | 47PASS,6FAIL; Cargo101 | 53PASS; Cargo0 |
| Two new production Read/Job selections | Each1 / 1 | 0PASS,2FAIL | 2PASS |
| Prior production/password preservation | Each1 / 1 | — | 16PASS |
| ZIP crypto/method/boundary preservation | 5+13+2 / 5+13+2 | — | 20PASS |

Final authority is Worker `b7-ref19-dlz-red` → `b7-ref19-dlz-green2`, production `b7-ref19-dlz-product-red2` → `b7-ref19-dlz-product-green3`, and matching `worker-green3-build`. Paired tests/manifests are byte-identical. Final source has91successful GREEN executions, with no ignored/SKIP/not_run cases. Exact commands, environments, resource observations, counts, raw logs and source/input snapshots are retained in the [frozen evidence inventory](evidence/ref19/alz-dlz/files.txt).

Production Browse exposes the exact original catalogs. Test verifies four files/1,977,732bytes, and Browser-bound immutable extraction Jobs publish exactly those native bytes with no extra outputs. These totals distinguish the three genuine files from the separately labelled specification member.

## Failure and lifecycle evidence

- Ten owned bitstream/CRC/extent mutations retain readable outer ALZ catalogs but return CorruptArchive from actual Test/extract without committing the damaged member. Cases cover block magic, origin, empty alphabet, group/selector counts, second-block/final markers, truncation, unused packed bytes and member CRC.
- Worker working budgets below5MiB return ResourceLimit; exactly5MiB extracts all three specification blocks to the independent bytes. Original entry/total-output limits remain enforced.
- Actual first-write cancellation and pipelined native write failure retain the original typed terminal, abort unverified output and drain the child/stderr before owned output/source release.
- Production damaged CRC, one-byte member limit, current Job cancellation and first native error112 all retain zero committed files, protected source handles through cleanup, and actual exclusive reopen/owned-tree removal.
- Original password, plain/split ALZ/EGG, source revision/readmission and ZIP preservation selections pass on final source.

Archive all-target check, isolated-codecs all-target strict Clippy, default-library strict Clippy, three-file rustfmt check and source/link/structure spec gate all pass. Both authoritative C# builds succeed with zero warnings/errors; matching Worker builds succeed. The [original B7 baseline record](evidence/ref19/alz-dlz/original-b7-baseline/record.json), slice-before copies and scoped generated diff retain attribution without assigning unrelated dirty changes to B7.

## Limits

This claim covers ordinary plain ALZ method1. Encrypted/split DLZ producer profiles remain unqualified. Public Jobs retain their original64MiB minimum working budget; the5MiB codec boundary is verified directly at the Worker seam. EGG AZO/solid, remaining names/special kinds and separately unqualified encryption profiles remain in REF-19. Physical UI, installed and Release acceptance are separate levels. REF-20 and the parent phase remain open.
