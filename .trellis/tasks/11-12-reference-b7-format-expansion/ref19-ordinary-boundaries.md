# REF-19 — ordinary failure and resource boundary

Status: ordinary Worker controls GREEN; REF-19 remains in progress / not_accepted.

The15-test `ref19_alzeg_contract` target covers the ordinary samples plus CRC corruption, structural section/final end markers, truncated/trailing source data, trailing bytes within a declared packed block, finite metadata/working/entry/total budgets, pipelined native write failure, real cancellation and an opaque EGG LZMA prefix. All operations use the original isolated child and actual child wait/stderr drain.

The controlled native peer owns actual temporary file tokens. Successful output is read back from those files and compared with the independent expected bytes. Unverified members do not commit. Native write failures remain ResourceLimit through the next non-abort control boundary, matching the existing bridge's first-write-failure contract. Cancellation remains Cancelled; the parent closes/removes unfinished files after actual child/pipe exit and verifies their removal. These controls exercise the worker and owned filesystem peer, not C# protected handles or deployed native output.

| Selection | LIST | RUN | PASS | FAIL | ignored / not_run | Cargo exit |
| --- | --- | --- | --- | --- | --- | --- |
| RED | 15 | 15 | 14 | 1 | 0 / 0 | 101 |
| GREEN | 15 | 15 | 15 | 0 | 0 / 0 | 0 |

The exact commands are the locked offline package/feature/target selection recorded in [ordinary methods](ref19-ordinary-worker.md), with no additional filter. Test bytes are identical across the authoritative RED/GREEN. The one-line repair classifies an illegal signature inside the already-recognized EGG prefix as CorruptArchive; explicit unsupported version, solid and encryption branches retain their own categories.

[Result](evidence/ref19/ordinary-boundaries/result.json), [manifest](evidence/ref19/ordinary-boundaries/files.txt), exact commands/counts/exits/raw logs and source/test bytes are retained separately from the earlier ordinary and Stored evidence. Split/native/product/password/method expansion and REF-20 remain unfinished.

## Scoped preservation and quality

ZIP method preservation passes13/13 with isolated codecs and13/13 in the default feature graph; ZIP boundary preservation passes2/2. All selections use nonzero LIST and the same RUN filter, with no ignored/not_run successes. Archive isolated all-target check and strict all-target Clippy pass. Default-library strict Clippy, eight-file scoped rustfmt and the specification-contract gate pass. The two ALZ/EGG-specific codec entry points use the existing isolated-codecs condition, preserving the default library boundary without warning suppression.

[Scoped gate evidence](evidence/ref19/ordinary-gates/result.json) records28successful preservation executions, exact commands/results/raw logs and current feature-boundary source. These are interim ordinary-slice checks; final B7 cross-layer, native, preservation and attribution checks remain pending.
