# REF-20 — Immutable intent, physical layout and protected sources

Status: these boundaries are GREEN. Full REF-20 delivery is **ready_for_acceptance / not_accepted**, indexed in the [handoff](ref20-outcomes.md). Phase E remains open.

## Implemented boundaries

- Public Rust/C# creation contracts carry additive default-false `self_extracting`, omitted when false. ZIP-only clear Store/Deflate, a nonempty `.exe` stem and the measured first-EXE volume ceiling are validated. Passwords and incompatible formats are rejected, not cleared. The original Job dispatch and record snapshot retain the captured scalar; SFX does not enter ordinary overwrite confirmation.
- C# requires `zip_sfx_v1` before accepting worker native calls for an SFX request. The implemented worker advertises it only for explicit ZIP-SFX intent; admission cannot silently fall back to ordinary ZIP.
- Private `zip_sfx.rs` validates the exact bounded x64 PE raw end and48-byte own descriptor, without ZIP signature searching. Product profile admission retains the recipient's65536-entry/4096-name-byte ceiling and `central_size * 8 + entries * 512 + 256 KiB <= 64 MiB` metadata charge.
- `zip_container` retains the original split writer and record relocation. Its SFX adapter writes the known complete prefix before physical coordinates, uses a span marker only for split output, retains intact-record/output/1024-volume admission, forbids ordinary output from changing disks and patches the final descriptor count at the known location. First EXE size is bounded by4,294,967,295bytes; split admission includes the835,636-byte minimum.
- Original normalization, `ValidatedZip` and full ZIP verification read the final prefixed candidates. The original reader accepts qualified ordinary/split Store/Deflate and forced ZIP64 profiles, preserves complete source revisions and read-only capabilities, and refuses incomplete or reordered held sets. Ordinary SFX remains a single source, not a split capability merely because its revision retains a source vector. SFX receives no editing grant.
- `ArchiveVolumeSources.Sfx.cs` extends the original native source collection. EXE and final-ZIP entry points acquire `.exe/.z02…/.zip` before decoder admission, reread the own descriptor under leases, check marker/end counts and reject surplus numbered members. The original identity, requested-member binding, verification and Dispose owners remain in force. Ordinary single-file ZIP keeps its original path.
- The original C# output bridge admits ordinary EXE or exact split-family candidate names, keeps the unpublished encoder token and all-candidate-before-publication gate, and extends fresh-family checks to EXE/final ZIP/numbered targets. Ordinary and split result locations are EXE and final ZIP respectively. Actual creation/publication and partial-journal verification pass in the [creation evidence](ref20-creation-publication.md).

## Behavioral evidence

Every selection uses nonzero LIST followed by RUN with the same filter. No ignored/SKIP/not_run result counts as PASS.

| Boundary | RED authority | GREEN authority |
| --- | --- | --- |
| Public Rust intent | `b7-ref20-rust-intent-red-*`: unknown strict `self_extracting` field | Public function in `b7-ref20-rust-intent-green-*`:1PASS |
| Private Rust intent | `b7-ref20-rust-private-intent-red2-*`: original worker bytes, corrected final bulk-enabled request,1FAIL | Same final test bytes in `b7-ref20-rust-private-intent-green2-*`:1PASS |
| C# public intent | Original nine product RED selections include the wire contract | `b7-ref20-product-intent-green.*`:LIST1/RUN1/PASS1; unchanged test function |
| Product physical layout and original reader | `b7-ref20-rust-layout-red-*`:LIST14/RUN14,1PASS/13FAIL |11PASS in `b7-ref20-rust-layout-green-*`, plus three distinct ordinary coordinate selections in `b7-ref20-rust-layout-ordinary-{store,deflate,zip64}-green2-*`;14unique PASS |
| Native complete source family | `b7-ref20-product-sources-red2.*`:four individual LIST1/RUN1 selections,1PASS/3FAIL | `b7-ref20-product-sources-green.*`:four individual LIST1/RUN1/PASS1 selections; same final test/registration bytes |

The coordinate tests preserve the independently qualified42data-only physical inputs and manifest. They cover six original-reader profiles, exact8230decoded bytes per profile, physical candidate offsets, UTF-8/directory/empty members, stream descriptors, ZIP64, original full payload verification, incomplete held authority, corruption, cancellation and prefix/loader limits. Layout input encodes the same qualified literal contents through the original ZIP encoder before exercising the new coordinate adapter. No EXE is executed by these tests.

Native source tests use GUID-owned copies only. All members deny writers while leased; the parent directory denies rename. Identity remains stable, the requested entry retains its original identity, and every existing source exclusively reopens after success or typed failure. Three ordinary profiles and three split profiles from both supported entry points are covered. Missing first/middle/final, descriptor version/count, surplus `.z01`, bounded admission and an unrelated unleased file are checked. C# builds pass with zero warnings/errors.

Raw commands, required originating environment, source witnesses, LIST/RUN logs, counts and exits are frozen under `evidence/ref20/implementation/raw/`. Layout test bytes match retained RED/GREEN copies directly. Prerequisite records remain distinct from behavioral authority.

## Integrated delivery

The original worker_zip_create/worker_volume_output chain, explicit capability, independent final product outputs and reviewed generated-output recipient execution pass. Official Create UI and original completion lifecycle, converged preservation/quality, bounded review and evidence freeze are complete. The [REF-20 handoff](ref20-outcomes.md) records exact evidence and limitations; independent acceptance remains open.

The separately completed recipient22 tests, reviewed native18 commands and loader4 observations remain the authority in [recipient native execution](ref20-native-execution.md); they are not rerun or substituted for product creation. See the [capability matrix](ref20-capabilities.md) for evidence layers.
