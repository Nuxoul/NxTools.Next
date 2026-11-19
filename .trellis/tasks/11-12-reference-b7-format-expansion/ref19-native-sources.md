# REF-19 — protected native source families

Status: source-acquisition slice GREEN; REF-19 remains in progress and not accepted.

## Delivered boundary

The original `ArchiveVolumeSources` and `ArchiveSourceLease` now acquire and protect genuine ALZ/EGG families before the existing source-acquire reply exposes paths to the worker. ALZ follows actual version10/ordinal and CLZ03 continuation/CLZ02 final framing, bounded to1000parts. EGG resolves a finite same-directory Header-ID index, rereads each selected prefix under its acquired lease, validates reciprocal links and termination, and orders the family even when entry starts at a continuation. Missing links, duplicate referenced IDs, cycles and malformed framing cannot become a single-file success.

Source identity, all-source verification, existing native diagnostics and actual Run cleanup retain their original owners. Every selected file and its parent chain stay protected from writers and renames. Unreferenced EGG archives are not retained as family members. Failed acquisition releases every acquired lease; successful disposal permits exclusive reopen and owned-tree deletion.

## Evidence

Frozen [native-source-families/files.txt](evidence/ref19/native-source-families/files.txt) enumerates187files excluding itself: exact commands, raw logs/exits, RED/GREEN/current/pre-edit inputs, changes, lawful input copies/provenance and the retained independent split oracle. Earlier Worker evidence is unchanged.

| Selection | Nonzero LIST / actual RUN | PASS / FAIL | SKIP / not_run | Exit |
| --- | --- | --- | --- | --- |
| Six original-owner source tests, RED | 6 / 6 | 0 / 6 | 0 / 0 | Each RUN1 |
| Same six tests, GREEN | 6 / 6 | 6 / 0 | 0 / 0 | Each RUN0 |
| Original ZIP/7z source protection | 2 / 2 | 2 / 0 | 0 / 0 | Each RUN0 |

RED/GREEN test bytes are identical. Each case is individually listed and run so fail-fast leaves no unexecuted tests. All three actual C# builds complete with warnings treated as errors.

Inputs cover four genuine3-/8-part families,22physical volumes, ordinary ALZ Stored/Deflate and ordinary EGG Stored. Tests verify exact source order/count, first/middle/final EGG entry, renamed continuations, unrelated-archive exclusion, bounded path-free identity, every-member writer rejection, parent-rename rejection, missing/corrupt/duplicate/cycle outcomes, finite encoded/metadata admission and actual exclusive reopen/removal after success and failure. Payload correctness remains the separately established independent oracle and Worker boundary.

The execution uses `dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore --nologo -m:1 -nodeReuse:false -p:UseSharedCompilation=false`, then the existing test apphost with `--list <exact-name>` and the same `<exact-name>`. Exact six/two filters and counts are retained in the result ledgers. Fixtures write only GUID-owned session scratch files and invoke no member, Shell, product UI, Registry or service operation.

## Remaining integration

Production Read/Browser/Job validators and Auto CP949 provenance, native error-category propagation, stage-bounded Rust source revalidation, output readmission and the remaining codec/password/name variants are not established by this source-acquisition slice. Physical UI, installed and Release acceptance remain separate.
