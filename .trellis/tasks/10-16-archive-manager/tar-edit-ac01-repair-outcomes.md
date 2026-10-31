# TAR-AC01 — minimal raw-header fidelity repair outcomes

2026-10-29. **Repair implementation delivered; pending coordinator independent re-acceptance.** Still **Phase E — Format expansion and ZIP editing**, Step3 only. No Step4/HZ3/HZ4/HZ5. Original [acceptance](tar-edit-acceptance.md), [implementation outcomes](tar-edit-outcomes.md) and [matrix](tar-edit-matrix.md) remain byte-identical; their failures/history are not rewritten. Sole status authority remains [checkpoint](phase-a-checkpoint.md).

Evidence root: `C:\Users\Administrator\.pi-desktop\scratch\0c0e5320-11c4-40dd-bb50-f6a437188fc1\tar-ac01`.

## 1. Confirmed gap and minimal boundary

Read the independent acceptance, latest checkpoint, full42-line `tar-header-probe.py` and its actual failed `header-probe-posix/result.json` before changes. The prior frozen worker preserved decoded name/content and source, but unnecessarily changed the untouched main header checksum from `3031303134370020` to `3030313031343700` on PAX-only rename. This is an exact raw-fidelity defect, not payload loss. Existing self-plan readback comparison could not detect it because the plan was already normalized.

Only one product file changes: `rust/crates/nxtools-archive/src/tar_edit.rs`. Preserve the original ordinary structure admission, metadata refusal, finite budgets, outer codec, ZIP/7z editors, control/queue and native FILE-only handoff/publication. New regression seam is the actual worker -> held candidate -> independent original-byte comparison, as requested. No added reviewer, subagent/session or broad reference/backend research.

The original Ark reference remains clean at `f7ea89e828925244d0a6b76cbfb4ac5b9fafe2e5`, read/write plugin SHA256 `16D219AC0955DB5712AAD8D7C25C8108CF5509062417B16407B7727CD1447B5C`; existing streaming/filter/license observations unchanged. No source copied from Ark and no new reference capability claimed.

## 2. Repair behavior

- `Member::change` now compares the complete main header before recalculating checksum. An overriding PAX/GNU-only path change that does not alter main bytes retains the entire original main header, including legal numeric/checksum representation.
- `change_pax` validates the original bounded PAX payload, replaces **only** the affected path/size record, and leaves every unrelated record byte-for-byte intact, including legal padded length prefixes. An already matching value does not rewrite its record. When a new path record is required, append it without reconstructing old records.
- `Extension::change_data` retains the complete extension header when payload length is unchanged. TAR header checksum covers only header, not extension payload. Only a changed payload length updates the size field and dependent header checksum. Unaffected PAX/GNU extensions therefore keep their original header/payload representation.
- A real replacement size change still updates main size/checksum and any PAX size override. New member headers receive a valid initial checksum, including long names represented only through a new PAX extension. No blanket refusal of legal noncanonical ordinary headers, no deleted fidelity promise, no archive-wide canonicalization.

Exact specification increment: [TAR edit contract](../../spec/nxtools-archive/backend/tar-edit.md), raw-byte dependency rule and required independent regression. No safety/metadata policy relaxed.

## 3. Red -> green at the real worker boundary

New files:
- `rust/crates/nxtools-archive/tests/tar_edit_raw_header_contract.rs` (four Rust integration tests).
- `rust/crates/nxtools-archive/tests/fixtures/tar-edit-raw-headers.py` (independent tiny CPython fixture/raw-byte/content oracle and actual worker protocol driver, exact child30s watchdog).

The oracle generates valid six-octal+NUL+space and seven-octal+NUL checksum forms; space-padded octal and GNU base256 numeric fields. Original values/record bytes are the expected data, **not** editor-generated plan bytes. Every source and candidate is independently parsed with CPython and bsdtar, with exact names/content/uid/gid/mode/owner/group/PAX time precision. An untouched second member is compared as a complete original raw header/extensions/content record. All source archives are GUID-owned test data, unchanged after worker requests; candidate publication is protocol fixture only, actual native publication is covered separately below.

| Exact Rust test | Final cases | Observed |
| --- | --- | --- |
| tar_ac01_pax_only_rename_preserves_untouched_main_and_extension_fields |2representations x same/longer pathname; same/changed extension payload lengths |RED before fix; GREEN |
| tar_ac01_gnu_only_rename_preserves_unrelated_pax_and_main_header |2representations x same/longer GNU name, unrelated PAX extension |RED before fix; GREEN |
| tar_ac01_replacement_preserves_unrelated_extension_for_same_and_changed_size |PAX-only and PAX+GNU2shapes x2representations x same/different replacement size |RED before fix; GREEN |
| tar_ac01_replacement_updates_only_affected_pax_size_record_and_dependent_header |2representations x same/different payload size; unaffected PAX path/time records retained |RED before fix; GREEN |

- `worker-raw-red.log/json`: compiled actual worker regression **4RUN/0PASS/4FAIL**, exit101,1.66s tests. Independent content/name/metadata/source checks passed; raw main/extension/PAX-record equality failed. No compile/selector failure counted as behaviorRED.
- `worker-raw-green.log/json`: same four names **4PASS/0FAIL**, exit0. Final fixture refinement makes PAX same-length rename a genuine unchanged extension length and adds GNU replacement cases.
- Final-shape differential against **prior** frozen worker SHA256 `00FAFA51E9CDE82E3F468829F061CF950234D60D233F6C7F0BB11E7ADA1A8191`: all5scenario drivers exit1 with the fidelity defect (`old-final-shape-*.log/json`,20cases; normal numeric/canonical-positive subcases retained). This was not a production mutation or reverting the fix.
- Final source-built/frozen worker: all5scenario drivers exit0, **20/20raw cases**, no failures, source unchanged and complete terminal; `frozen-final-*/results.json`, per-case `owned.tar`, `candidate.tar`, `owned.wire.json`, `result.json`, and `frozen-raw-results.json` / `final-counts.json`.
- The **unchanged original independent acceptance script** was executed with only its work-pointer/identities fixture selecting this repair's frozen worker. `original-probe-green.log/json` and `original-probe/header-probe-posix/result.json`: original and candidate checksum both **`3031303134370020`**, `original_main_unchanged=true`, `source_unchanged=true`, independent content/name PASS. Script remains read-only and hash recorded in `original-probe-identity.json`.

Small intermediate editing/fixture bookkeeping mistakes were corrected before relevant execution (helper brace placement/import line, test length setup; summary counted arrays as groups, then matched log files). They are not productRED or suppressed failures; raw behaviorRED and successful20case evidence remain separately saved. Old final-shape per-child exit1 is expected sensitivity evidence, not a green wrapper exit claim.

## 4. Relevant verification and exact counts

Every Cargo invocation checked idle shared cargo/rustc lane immediately before start; same process `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`; `--locked --offline`, serial test threads, full timeouts. No timeout/OOM/process kill/duplicate Cargo this repair. Test-only watchdog may terminate only its exact owned worker child after30s; none triggered.

### Rust focused regression

Exact args saved in `regressions-command.json`:
`cargo test -p nxtools-archive --features isolated-codecs --locked --offline --lib --test tar_edit_raw_header_contract --test tar_edit_eligibility_contract --test tar_edit_matrix_contract --test tar_edit_worker_contract --test zip_edit_contract --test zip_edit_worker_contract --test sevenz_edit_safety_contract --test sevenz_edit_worker_contract --test stream_contract --test tar_time_contract -- --test-threads=1`

**91PASS/0FAIL/0ignored/0filtered**, exit0 (`regressions-final.log/json`): lib42 (includes existing7TAR finish/flush/outer/plan cases), TAR eligibility1/matrix7/raw4/worker3/time6, original stream12, ZIP edit10+worker1,7z safety4+worker1. All exact names are in raw log; no full42target or321Desktop rerun claimed.

- `cargo clippy -p nxtools-archive --features isolated-codecs --tests --locked --offline -- -D warnings`: PASS exit0 (`clippy-final.log/json`), the only edited product owner. No other Rust product owners edited; five-owner full lint already recorded by prior independent acceptance, not reattributed to this repair.
- `cargo check --workspace --tests --locked --offline`: PASS exit0 (`workspace-check.log/json`),42.16s.
- scoped `rustfmt --edition 2024 --config skip_children=true --check` for2changed Rust files: PASS; global/scoped `git diff --check`: PASS, inherited CRLF notices retained, not edits.
- `dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore -m:1`: incremental PASS **0warnings/0errors**, exit0 (`dotnet-build.log/json`). No C# source change/clean-build claim.

### Matching worker and actual native TAR5

`cargo build -p nxtools-archive --features isolated-codecs --bin nxtools-archive-worker --locked --offline`: PASS exit0 (`worker-build.log/json`). Frozen in this repair's scratch:
`C:\Users\Administrator\.pi-desktop\scratch\0c0e5320-11c4-40dd-bb50-f6a437188fc1\tar-ac01\frozen\nxtools-archive-worker.exe`
SHA256 **`14089B5024EDFD53E7E07619C06231D27536441C20E3180F65CEE555CCE04384`** (`worker-identity.json`). Current source-built worker copied after this round's build, not a prior binary reused. Original old frozen worker retained for differential RED.

Existing effects audit reread; no new native registrations. Exact `NxTools.PlatformHost.Tests.exe --fixtures --list 'archive tar edit'` -> **identical** RUN filter:
**5selected/5RUN/5PASS/0SKIP/0FAIL/0not_run**, exit0 (`native-list.log`, `native-run.log/json`). Explicit worker/hash/env required and used. Names:
1. archive tar edit wire shapes capabilities
2. archive tar edit native actions metadata empty
3. archive tar edit native source cancel handoff
4. archive tar edit native failure unknown late cancel
5. archive tar edit native conflict injected lowspace

These reuse real original Job/browser/native same-path publication, held-source/parent cancellation/native5/64/latecancel/conflict and injected112;112 remains injected before_flush, **not real disk-fill evidence**. Original ordinal/version/attempt/native transaction untouched. Owned fixture roots are in `native-fixtures/roots.txt`.

### Tool provenance / static

Same CPython3.13.2 SHA256 `7D96A4ED35D6E596CD9DD8933FEAFC66349CC21F75BE3B15C89FD336E50140C1`; System32 bsdtar3.7.7/libarchive3.7.7 SHA256 `92FAFED485AF0FE4C291457DC3C00D0241F6A136731BAB63EF18A3F0C949D37A` (`raw-oracle-tools.json`). Independent tools are test-only, no backend/ABI/dependency or tool install. Spec116documents/14owners, GPUI0.8.0/pre0.3.9 provenance, pinned57file Markdown editor and PlatformHost effects/test-selection static gates all **PASS**, exit0 (`static-gates.json` and gate logs). Static passes do not prove native/user acceptance.

## 5. Source hashes and history preservation

| Repair product/test source | Final SHA256 |
| --- | --- |
| rust/crates/nxtools-archive/src/tar_edit.rs |30F80D56C7921C6F3B5B9F332E90DFD28B0ED34DED00A5EF7BDC9AC90B4D3E6A |
| rust/crates/nxtools-archive/tests/tar_edit_raw_header_contract.rs |D76944C9277727C0C9A90D52D04BC71A7EB461149E8720BD6D1E301A97305595 |
| rust/crates/nxtools-archive/tests/fixtures/tar-edit-raw-headers.py |6F41464AAAE62C20360CDF0F01634D9E8CDE155A181F6AFD0570736DB72D154D |

`opening-hashes.json`, `repair-source-hashes.json`, `final-scope-hashes.json` and `protected-hash-check.json` record exact files/bytes. The original worker_tar_edit, outer codec, ZIP/7z editors, SourceLease/OutputSession, root Cargo.toml/Cargo.lock and original tar-edit-outcomes/matrix/acceptance are unchanged from this repair's opening hashes. Existing unrelated dirty work remains; no reset/stage/commit/push. Documentation changes are only this repair report, additive TAR precise contract and sole checkpoint prefix.
Prior52delivery-source comparison confirms only tar_edit.rs changed; the other51files remain exact (`prior-delivery-drift.json`). Current source-built worker and repair-frozen worker have the same SHA256. No verified-source drift after focused tests.

Current checkpoint saved before any edit: **175179bytes**, SHA256 **`89632E6602670D14747F76A7C681435C2E2846F121F0A92821507F919E1A576F`** (`checkpoint-before.bin`). Top-prepend only; entire saved content must remain an exact suffix, including coordinator acceptance prefix and all older failures/A–F headings. Final suffix result lives in `checkpoint-prepend-verification.json`; historical reports never rewritten.

## 6. Remaining and stop boundary

- **TAR-AC01 repair is ready for independent recheck, not self-accepted.** Ordinary supported/refused structure scope and all original fidelity/space/transaction limitations remain.
- Did not rerun full42Archive targets,321Desktop, broader42/56native compatibility, full5owner lint or all resource/codec combinations; the concrete product change is limited to TAR header/extension mutation. Focused91tests, archive-owner strict lint, workspace types and actual nativeTAR5 are this round's evidence.
- Prior known C# readmission `source_unavailable`, historical CLI `helper_untrusted`, FB-AC05, and unresolved original ZIP-chain Cancelled-vs-SourceChanged instability remain as documented; not rerun or declared fixed. Independent acceptance's later254PASS/0FAIL does not erase old instability logs.
- No physical Explorer/IME/DPI/UIA/visual-user/installed-release acceptance, process crash/restart/HZ4, source reclamation/HZ5, huge archives/real disk fill. No package/sign/install/product/Explorer/system/Registry/service/ACL/UserChoice/Recycle/user-archive/commit/push operation; no self-app GUI refusal bypass.

**Stop after this repair report and wait for coordinator independent re-acceptance. No Step4/HZ3–HZ5.**
