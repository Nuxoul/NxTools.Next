# REF-19 — source verification stages and output readmission

Status: slice GREEN; REF-19 and B7 remain in progress / not accepted.

## Original ownership

`SourceSet` retains the ordered all-source revisions. `worker_formats::run` verifies ALZ/EGG bytes at operation stages; `ArchiveVolumeSources` retains native object/namespace leases and verifies every source at each original `source_precommit`. `ArchiveWorkerRunner` holds these leases through actual child, pump and Job cleanup. `PrivateOutput::preflight_output` and `worker_browser::selected` remain the output-shortfall and selected-member policy owners.

The existing fingerprint now records its actual reads in `ArchiveWorkObservation.revision_passes/revision_bytes/revision_us`. No new digest, identity owner or per-member diagnostic is introduced.

Extraction scans occur at initial admission, after split normalization, before the first actual file commit and before successful completion. Ordinary sources have no normalization scan. Native protection remains per commit. Final source failure denies success and source-deletion eligibility without erasing already completed outputs.

Encoded readmission borrows the original finite default source ceiling only for extraction output-readmission or selected-member requests. Native content inspection occurs under the original held lease. Target-output, entry, metadata, working-memory and Job limits are unchanged; an encoded reservation grants no output or decoder credit.

## Behavioral RED → GREEN

| Boundary | Identical nonzero LIST / RUN | RED | GREEN |
| --- | ---: | --- | --- |
| Real isolated worker `ref19_alzeg_contract` | 30 / 30 | 27 PASS, 3 FAIL; Cargo101 | 30 PASS, 0 FAIL; Cargo0 |
| Production Job readmission and Browser selected-member limit | 2 individually selected and run | 1 PASS, 1 FAIL | 2 PASS, 0 FAIL |

All ignored/SKIP/not_run counts are zero. Rust and C# test bytes are identical across each pair. The worker RED observes18 source scans rather than12 for the three-part/four-member ALZ family, accepts a source changed after the final commit, and cannot return the original declared-output shortfall under low output admission. The native Job RED reaches resource_limit without the required source-bound shortfall. The selected-member ceiling is preserved throughout.

The GREEN work regression checks actual terminal observations and independent exact output bytes:

| Input | Physical sources | Members | Fingerprint passes |
| --- | ---: | ---: | ---: |
| ALZ Stored split | 3 | 4 | 12 |
| ALZ Deflate split | 8 | 10 | 32 |
| EGG Stored split | 3 | 4 | 12 |
| EGG Deflate split | 8 | 1 | 32 |
| Ordinary ALZ Stored | 1 | 1 | 3 |
| Ordinary EGG Stored | 1 | 5 | 3 |

Revision bytes equal complete physical-source bytes multiplied by the stage count. This bounds scans independently of member count; it is not a throughput or timing claim. The original changed-continuation-before-first-commit regression remains GREEN. Changing a continuation after the last controlled commit returns SourceChanged while all independently verified completed bytes remain factual.

## Low-output native lifecycle

Both genuine three-part Stored families use metadata8192bytes and entry/output allowances1byte. The original Job returns resource_limit with current1/1 and required40,000/160,000bytes, a complete-source binding, zero committed files, complete cleanup and no destination creation. Sources then reopen exclusively.

A separately submitted output-only approval retains the exact source binding and metadata/working limits. Both retries succeed: eight files,320,000native bytes, every byte equal to the independent CC0 oracle contents and no extra files. Original Browser-entry materialization narrowed to1byte returns selected_output_limit, no cache lease/path and actual worker/source cleanup for both families.

## Preservation and quality

Fresh same-filter LIST/RUN selections on this source:

- Eight plain production Read/Browser/Test/extract, missing/corrupt, stale Browser, cancellation and sticky native-write lifecycle tests:8PASS.
- Six ALZ/EGG native source-family protection/admission/release tests:6PASS.
- Two original ZIP/7z protected-source tests and the original ZIP/TAR/7z/RAR output-readmission Job test:3PASS.
- Rust `zip_volume_contract`:6PASS; `sevenz_readmission_contract`:1PASS; `rar_readmission_contract`:1PASS.

Together with the30-test worker target and2production selections, this slice has57successful GREEN executions, zero failures/ignored/SKIP/not_run. Five actual C# test-host builds succeed with zero warnings/errors. Both explicit matching-worker builds succeed.

Archive all-target check and strict Clippy with isolated-codecs, default-library strict Clippy, four-file rustfmt check and the specification source/link/structure gate all exit0. The specification gate covers129documents/14workspace owners, not native/physical/Release acceptance. No historical unrelated lint is changed or suppressed.

## Evidence and remaining scope

Frozen [evidence](evidence/ref19/source-stages-readmission/files.txt):374files excluding the manifest. It retains raw commands, logs, counts, environments, exact test/source snapshots, slice-before copies, scoped diffs and results. Shared Program snapshots are complete, while ownership is restricted to this slice's two reviewed REF19 TestCase registrations; concurrent registrations/functions are not B7 changes. Earlier frozen slices are unchanged.

Remaining REF-19 work is method/solid/password/crypto/name-profile integration and its separate evidence. REF-20 remains pending. No independent acceptance, physical UI, installed/Release, packaging, deployment, reference-application/member execution or system change is claimed.
