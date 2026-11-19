# REF-19 — Names, path policy and typed declarations

Status: **slice GREEN / not accepted**. REF-19's declared sample-driven capabilities and converged verification are complete. The separate [REF-19 handoff](ref19-outcomes.md) records ready_for_acceptance / not_accepted; this slice retains its original evidence boundary.

## Capability

| Profile | Independent prerequisite | Actual product result |
| --- | --- | --- |
| Raw unsafe names in ordinary Stored ALZ and EGG | Two project-owned specification archives,15names per format,30literal payloads/1,410bytes | Browse preserves names, ordinals, sizes and UnsafePath warnings; read-only Test verifies15files/705bytes per archive; extraction returns UnsafePath before native candidate creation |
| EGG locale0/932/949 and parent-relative declarations | Five project-owned specification archives; UInt16 locale before UInt32 parent ID before encoded name, independent codec/field validation | Worker Browse/Test/extract and production Read/Jobs return UnsupportedMethod, with no catalog or output |
| Encrypted-name flag and reserved Global Encrypt declaration | Two project-owned declaration fixtures | Worker Browse/Test/extract and production Read/Jobs return UnsupportedEncryption, with no password prompt or output |
| Genuine UTF-8, nested and automatic CP949 names | Existing unchanged CC0 ordinary, split and encrypted samples | Exact catalog/native output remains covered by the final production selections; automatic CP949 remains Heuristic949, not user Explicit |

All nine new inputs are specification fixtures, not producer archives. Encrypted-name/Global Encrypt inputs qualify declaration refusal only; they do not qualify encrypted-header structure or decryption. Literal raw-name payloads are independently checked by CPython struct, format CRC and exact extents before product execution.

Unsafe cases include traversal, rooted/drive/UNC paths, empty components, dot components, reserved device names, trailing dot/space, alternate streams, controls and an empty name. Original `path_policy::components/annotate` owns the warnings; no path is sanitized into an accepted output name. Read-only Test establishes payload integrity, not filesystem admissibility.

## Ownership and implementation

The only product change in this slice is one existing category-priority expression in `ArchiveReadSession.OnRunnerEvent`. Native source admission's CorruptArchive, UnsupportedFormat, UnsupportedMethod and UnsupportedEncryption retain priority over consequential metadata-pump cancellation, alongside the existing source/resource/missing-volume categories. Original cleanup-incomplete and timeout priorities remain unchanged.

The native `ArchiveVolumeSources.ReadEggPrefix` rejects Global Encrypt during acquisition and releases acquired leases. Other unsupported name flags fail in the Worker after source acquisition; their sources remain protected until actual cleanup. Production tests assert these distinct lifecycle boundaries, zero candidate writes/commits, CleanupComplete, exclusive source reopen and owned-tree deletion.

No Rust product, Runner, native source, Job, protocol, UI, dependency or runtime owner changes are included. Existing correlation, accepted/start/state/terminal, elapsed and native-code diagnostics cover the operation without new member/path/payload logs.

Three tests extend the real-child `ref19_alzeg_contract.rs`; two methods extend `ArchiveAlzegProductTests.cs`; four explicit reviewed Fixture registration lines extend `Program.cs`. Ten task-created fixture paths comprise nine archives and one independent manifest. Eleven existing slice-before copies are attribution witnesses only; the original B7 baseline remains authoritative.

## Verification

| Boundary | Result |
| --- | ---: |
| Real isolated Worker, nonzero LIST and same selection RUN | 74PASS / 0FAIL |
| Final production Read/Job/native lifecycle and preservation selections | 30unique PASS / 0FAIL |
| Matching isolated Worker build | exit0 |
| C# final-tail build | exit0, zero warnings/errors |
| Strict Clippy for the affected real-worker test target | exit0 |
| Single-file rustfmt check | exit0 |
| Specification contract gate after record updates | exit0 |

Total successful GREEN executions: **104**, a slice count rather than whole-B7 acceptance. Production authority is15successful rows retained from `b7-ref19-names-product-green2.results.json` plus15unique rows from `b7-ref19-names-product-green3.results.json`. Every row has LIST1/RUN1/PASS1 and zero FAIL/SKIP/not_run. Earlier successful rows and Worker executions were retained rather than repeated.

The three new Worker tests preserve the prior71-test prefix. They verify exact raw-name catalogs, read-only payload totals, rejection before output, seven declaration profiles across open/test/extract with and without a password, actual child wait/stderr/pipe drain and temporary token/source release. Production coverage includes all25REF-19 product selections, two original password selections and three original ZIP/7z source/Job shortfall selections.

A real initial production observation returned `cancelled` for Global Encrypt Browse instead of UnsupportedEncryption. The terminal ordering is scheduling-dependent: the retained final-fixture original-branch control passes. This is observed failure plus a passing control, not a deterministic identical-byte RED/GREEN claim. Final typed outcomes and native acquisition/cleanup assertions pass with the priority repair enabled.

## Evidence

Immutable `evidence/ref19/names-paths/` retains qualification sources/results, commands/environment/logs/exits, the observed typed failure and original-branch control, final production ledgers, quality results, four compiled-source witnesses, current fixtures, slice-before inputs, exact changes, resource audit and the original B7 baseline record. `files.txt` enumerates the retained files; `audit.json` records the final counts and byte attribution.

Authoritative scratch prefixes are `b7-ref19-names-green`, `b7-ref19-names-product-{green2,green3}`, `b7-ref19-names-tail-quality-r1` and `b7-ref19-names-final-spec`. Historical prerequisite failures remain separate from behavior results.

## Limitations

Locale/parent-relative decoding, encrypted filenames and Global Encrypt payload support are not implemented. Their typed declaration refusal is verified separately from producer-profile support. Previously unqualified method/encryption/volume combinations remain separate rows in the capability matrix. Physical UI, installed/Release and independent acceptance are not established by this slice. Parent phase names/order, B1–B6 acceptance and earlier frozen evidence remain unchanged.
