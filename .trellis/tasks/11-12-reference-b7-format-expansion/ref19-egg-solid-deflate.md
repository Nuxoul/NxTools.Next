# REF-19 — EGG solid Deflate and selected-source admission

Status: slice GREEN, not accepted. B7 remains in progress; REF-20 is pending.

## Capability and independent inputs

| Input | Origin | Files | Decoded bytes | CRC spans |
| --- | --- | ---: | ---: | ---: |
| `solid.egg` | Unchanged pinned alkegi/unegg CC0 producer sample | 5 | 6,342 | 1 |
| `spec_egg_solid_deflate_three_blocks.egg` | Project-owned specification fixture; CPython raw Deflate and fixed literal | 4 | 196,656 | 3 |
| Total | Origins retained separately | 9 | 202,998 | 4 |

The genuine archive attaches its only compressed block to the last file; four files own no block. The specification fixture has an empty file, predecessor-dependent data, three compressed payload ranges and decoded CRC spans that do not coincide with file boundaries. Independent CPython struct/raw-zlib validation requires complete framing, exact compressed consumption, EOS, total/file lengths and every block CRC. Independently decoded contents are retained as gzip envelopes, not product-generated expectations. No archive member or reference application is executed.

Browse remains catalog-only. Test and extraction verify the complete solid stream, not each file as an independent Deflate stream. The supported profile is ordinary plain solid Deflate with clear UTF-8 names.

## Original owners and bounded implementation

- `worker_alzeg/volumes.rs` validates bounded Solid Info framing while preserving linked-volume/header-boundary rules.
- `worker_alzeg/egg.rs` retains independent file-size and block-integrity spans. It does not label a block CRC or packed size as belonging to a file crossed by that block.
- New private `worker_alzeg/solid.rs` reads catalogued file ranges through fixed buffers and the original controlled Deflate decoder. It verifies all block CRCs, exact decoded size, EOS and packed consumption before the first file publication, then replays through one decoder retained across sequential file outputs. Unselected predecessors are decoded without being materialized. The state is operation-private and heap-owned; no whole encoded/plaintext stream buffer, new spool or dependency is introduced.
- `worker_password::execute` keeps its finite, no-output full-archive preparation. Original Job selections receive the same finite default decode allowance as Browser single-member/output-readmission requests; no output/conflict/delete authority enters preparation.
- Original `SourceSet::open_formats` and C# `ArchiveWorkerRunner` ALZ/EGG preflight distinguish encoded source size from selected target output credit. Job selections reuse the existing finite encoded ceiling under complete-family leases. Actual entry/output/metadata/working limits, immutable selection, source identity, per-commit native protection and publication ownership remain unchanged.

The two-pass solid strategy preserves verification before publication without a plaintext spool. It is not a throughput claim. Direct Worker admission is checked at256KiB; public Job working-memory admission remains unchanged.

Production diagnostics reuse the original operation/correlation ID, accepted/start, source/password state transitions and terminal elapsed/native-error fields. No per-byte, per-block or per-member diagnostics are added; payloads, secrets, paths and filenames remain outside production logs.

## Behavioral evidence

| Boundary | Same-filter LIST / RUN | RED | GREEN |
| --- | --- | --- | --- |
| Actual isolated Worker target | 58 / 58 | 53 PASS, 5 FAIL; Cargo101 | 58 PASS; Cargo0 |
| Final paired production selections | 3 individual LIST1 / RUN1 | 2 PASS, 1 FAIL | 3 PASS |
| Final production/password selection | 21 individual LIST1 / RUN1 | — | 21 PASS |
| Protected native source preservation | 6 individual LIST1 / RUN1 | — | 6 PASS |
| Original ZIP/7z source and Job shortfall preservation | 3 individual LIST1 / RUN1 | — | 3 PASS |
| ZIP crypto / methods / boundary preservation | 5 / 13 / 2 | — | 20 PASS |

Authoritative Worker pair: `b7-ref19-solid-red` → `b7-ref19-solid-green3`, with identical final Rust tests and manifest. The five new tests cover genuine and three-block exact Browse/Test/extract, late CRC/payload/unused-input/expanded-size failures before the first commit, finite workspace and original output limits, real cancellation, native first failure and actual child/stderr/source/output-token drain. The previous53tests remain unchanged.

Authoritative production pair: `b7-ref19-solid-product-red2` → `b7-ref19-solid-product-green3`, with identical final C# tests. The final three selections cover:

- Actual Read v2 → Test Job → Browser-bound extraction:9files/202,998exact native bytes, exact catalog/names/ordinals/kinds/sizes and no extra files.
- Tail-only immutable Jobs for both solid inputs and both genuine three-part Stored ALZ/EGG families:4files/84,143exact native bytes. Each Job retains8KiB metadata and grants output credit only for its selected file. The two complete160,000-byte families each produce only the selected40,000bytes.
- Late block CRC failure, lawful one-byte entry limit, current Job cancellation and native write error112: typed outcomes, zero file commits, source protection through `before_cleanup`, incomplete-output removal, exclusive source reopen and owned-tree deletion after actual cleanup.

Final successful GREEN executions total108:58Worker +21production/password +6native +3prior +20ZIP. Ignored, SKIP and not_run counts are zero. Matching worker and C# builds pass; the C# builds report zero warnings/errors. Existing password preparation, low-output readmission, selected-member limit, source-stage, plain/split/crypto/DLZ and ZIP preservation cases pass on final source.

Five scoped quality checks pass: Archive all-target check and strict Clippy with isolated codecs, default-library strict Clippy, seven-file rustfmt check, and the source/link/structure spec gate. These checks are not physical UI or Release acceptance.

## Evidence and attribution

[Frozen evidence](evidence/ref19/egg-solid-deflate/files.txt) contains1,032files excluding its manifest. It retains raw commands/environment/logs/exits/counts, both paired test versions, independent oracle and inputs, compiled-source witnesses, source snapshots, generated slice diff, original B7 baseline record and [audit](evidence/ref19/egg-solid-deflate/audit.json). The destination is immutable.

Product changes are five existing Rust files (`worker_alzeg.rs`, `worker_alzeg/{egg.rs,volumes.rs}`, `worker_password.rs`, `worker_codecs/volumes.rs`), new private `worker_alzeg/solid.rs`, and one C# Runner predicate. Tests extend the existing Rust/C# contracts; Program adds only six explicit Fixture registration lines for three selections. Twelve new fixture/expected paths have explicit provenance. The15-file slice-before snapshot is attribution only; the original B7 dirty/protected baseline remains authoritative. A separate source-review addition records the AZO arithmetic source chain and Nayuki MIT notice; it makes no AZO support claim.

No Protocol, UI, native authority, output runtime, dependency/lock, metadata policy, deployment, system, packaging or parent-phase change belongs to this slice.

## Limitations

Other solid methods, AZO payloads, encrypted solid data and producer split-solid profiles remain unqualified. Ordinary AZO, remaining names/special kinds and separately unqualified encryption profiles retain their own capability rows. B7 convergence checks, the bounded final review, independent acceptance and REF-20 remain pending. B1–B6 acceptance and frozen performance evidence are not reopened.
