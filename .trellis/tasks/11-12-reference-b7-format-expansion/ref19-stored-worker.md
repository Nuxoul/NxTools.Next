# REF-19 — ordinary ALZ Stored Worker slice

Status: first vertical slice GREEN; REF-19 and B7 remain in progress and not accepted.

## Result

The original isolated worker now reads ordinary version10 ALZ Stored archives through `worker_formats` → `worker_alzeg::Alz`. The real CC0 sample exposes one original `text.txt`; Test verifies3600bytes and the format CRC, and extraction returns all3600literal bytes through the existing output capability. Each of the three worker processes is actually waited; source removal succeeds after all readers exit.

The adapter keeps bounded record/name admission, raw names and immutable ordinals, original metadata/path policy, checked packed extents, full archive-end validation, source checkpoints and first-error/incomplete-output cleanup. Unsupported methods remain typed. No dependency, Windows authority, Job, IPC/runtime or replacement output framework is added.

## Evidence

Frozen archive: [manifest](evidence/ref19/stored/files.txt),56files excluding the manifest. [Result](evidence/ref19/stored/result.json) retains exact selection, exits/counts and test/sample byte equality; [scoped source attribution](evidence/ref19/stored/changes.diff) compares against B7 pre-edit bytes, not the whole HEAD diff.

| Check | LIST / RUN | Result |
| --- | --- | --- |
| Independent CPython struct/literal/zlib CRC prerequisite | One real sample | Exact name,3600bytes,CRC and end structure PASS |
| Original worker behavior RED | 1 / 1 | Cargo101; UnsupportedFormat at Browse;0pass/1fail/0ignored |
| Same test/sample GREEN | 1 / 1 | Cargo0;1pass/0fail/0ignored/not_run0; Browse/Test/extract |
| Spec contracts |129documents,14owners | Exit0; source/navigation only |
| Scoped rustfmt |lib.rs,worker_formats.rs,worker_alzeg.rs,ref19_alzeg_contract.rs | Exit0 |

Authoritative selection:

```text
cargo test --locked --offline -p nxtools-archive --features isolated-codecs
  --test ref19_alzeg_contract ref19_alz_stored_browse_test_extract_exact_bytes
  -- --list --exact
cargo test --locked --offline -p nxtools-archive --features isolated-codecs
  --test ref19_alzeg_contract ref19_alz_stored_browse_test_extract_exact_bytes
  -- --nocapture --test-threads=1 --exact
```

The archive retains raw commands, environment/commit observations, RED input/current source, prerequisite records and logs. Earlier prerequisite records are not successful product verification or additional accepted test counts.

## Remaining scope

The worker's native peer is controlled test infrastructure. Production Browse/Read/Job protocol admission, real C# source/volume/output handles, other methods/passwords, EGG, genuine split inputs and failure/cancellation matrices remain required REF-19 work. The [capability matrix](ref19-capabilities.md) records each row separately. No physical UI, installed or Release acceptance is claimed. REF-20 remains a separate subsequent implementation.
