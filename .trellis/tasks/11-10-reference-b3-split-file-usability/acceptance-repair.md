# B3 acceptance repair

Status: **accepted at source, production TestPlatform and owned-native fixture boundaries**. [Coordinator acceptance](coordinator-acceptance.md) closes B3-01, B3-02, B3-E1 and B3-E2; the implementation evidence below retains its original executed commands/results. Canonical **Phase E — Format expansion and ZIP editing / in_progress** and accepted B1/B2 remain unchanged. B4–B7 remain paused.

## Delivered behavior and assertions

### B3-01 — Original size-validation reasons

`OperationForm::select_volume_unit` preserves `CreateDraft::volume_size` / `ArchiveVolumeSize::bytes` errors. Only successfully normalized bytes with a remainder in the requested unit receive exact-divisibility guidance. A rejected switch keeps numeric text, previous unit and InputState entity; there is no rounding, truncation or byte reinterpretation.

Production TestPlatform regression `ref17_b3_01_actual_invalid_unit_switch_preserves_reason_and_correction` types into the original NumberInput, activates the real unit Select, then checks:

| Initial KiB text | Required reason |
| --- | --- |
| `0`, `63` | `分卷大小至少为 64 KiB，且必须是正整数。` |
| `1.5` | `分卷大小必须是正整数；请选择 KiB、MiB 或 GiB。` |
| `18446744073709551615`, `18446744073709551616` | `分卷大小超出可表示范围。` |
| `65` | `当前大小不能用所选单位的正整数精确表示。已保留原值；请保留单位或明确修改大小。` |

Every case retains text/KiB/entity and produces no outer Submit. Actual correction to2048KiB followed by real MiB selection gives2MiB, exactly2097152bytes and no error.

Genuine RED: LIST1/RUN1/FAIL1 in `b3-repair01-red-list.log` / `b3-repair01-red.log`: input0 received exact-divisibility guidance instead of the minimum-size reason. GREEN: LIST1/RUN1/PASS1 in `b3-repair01-green2-list.log` / `b3-repair01-green2.log`; the final REF17 and form selections below include it.

### B3-02 — First keyboard confirmation

The actual official Medium32 primary Button dispatches public `dialog::Confirm { secondary:false }` only for `ClickEvent::is_keyboard()`. DialogAction retains pointer Confirm ownership. Original deferred generation-scoped `on_ok` remains the sole Start continuation; no product key handler, vendor change or second submit route exists.

`ref18_b3_02_first_focused_enter_submits_exactly_one_start` and `ref18_b3_02_first_focused_space_submits_exactly_one_start` observe semantic `Role::Button`, exact label `确认合并` and `focused == true`, rather than guessing Tab counts. Escape and keyboard 返回核对 return to Preview with no Start. First Confirm activation produces exactly one original outbound Start with matching operation/session/generation/frozen preview/output; repeated activation produces no second Start. Original reply delivery reaches Terminal. The pre-existing REF18 selection preserves Cancel ACK≠terminal, actual cleanup-bound close, cleanup failure and late-success display.

Genuine RED: `b3-repair02-red2-list.log` LIST2; `b3-repair02-red2.log` RUN2/PASS1/FAIL1. First Enter passed; first Space failed at the original operation queue with `receiving from an empty channel`. GREEN: `b3-repair02-green-list.log` LIST2 / `b3-repair02-green.log` RUN2/PASS2, including first activation and repeat/no-duplicate assertions for both keys.

### B3-E1 — Actual preset and +/- controls

`ref17_b3_e1_actual_preset_and_step_controls_preserve_bytes_and_focus` operates original controls at680×600 and480×360. Actual preset Select activation chooses100MiB; actual official `increment` / `decrement` Button hitboxes inside the original numeric wrapper reach the original deferred NumberInputEvent::Step subscriber:

- 100MiB =104857600bytes →101MiB =105906176bytes →100MiB =104857600bytes.
- Stable original InputState entity; after each step, focus remains on that input.
- No outer Submit or submitted state; real scrolling reveals the row and fixed Medium32 Submit/Cancel footer remains reachable.

No direct helper invocation or manually emitted control event is used as evidence. First actual execution passed; this is coverage evidence, not a fabricated RED.

### B3-E2 — Original Submit to immutable Job and wire

`ref17_b3_e2_actual_create_submit_freezes_volume_in_job_and_wire` opens the original Create form through ArchiveApplication, types65KiB into its actual input and clicks its actual Submit. Original deferred admission → JobQueue → JobRecord → PlatformBridge dispatches one ArchiveJob Start. The accepted form slot retires; original Job state is `正在准入`.

`JobRecord.command.request.create.volume_bytes == Some(66560)`; the actual outbound command equals the entire accepted command, including job identity and the exact volume bytes. A narrow existing-impl `cfg(test)` method then changes only the retained already-submitted CreateDraft to128MiB and checks that draft value. Accepted JobRecord and already-outbound command stay unchanged at66560bytes; no new Start appears. The helper cannot create admission evidence and is absent from production. No native task runs in this TestPlatform fixture. First actual execution passed.

## Exact repair paths

Product behavior:

- `rust/apps/nxtools-desktop/src/archive/forms_volume.rs`
- `rust/apps/nxtools-desktop/src/tools/raw_join.rs`

Original test modules / test-only seam:

- `rust/apps/nxtools-desktop/src/archive/forms_tests.rs`
- `rust/apps/nxtools-desktop/src/archive/application_tests.rs`
- `rust/apps/nxtools-desktop/src/archive/forms.rs` — existing `cfg(test)` impl only
- `rust/apps/nxtools-desktop/src/tools/raw_join/tests.rs`

Contracts / navigation:

- `.trellis/spec/nxtools-core/backend/database-guidelines.md`
- `.trellis/spec/platformhost/raw-numbered-join.md`
- `docs/reference-code/archive-volumes.md`
- `docs/reference-improvement-taskbook.md`
- `.trellis/tasks/11-02-archive-workflow-optimization/implement.md`
- This task's `acceptance-repair.md`, `task.json`, `design.md`, `implement.md`, `outcomes.md`, `ref17-outcomes.md`, `ref18-outcomes.md`, `changed-paths.txt`.

The [complete B3 path list](changed-paths.txt) retains initial delivery paths and adds the repair evidence paths. Repair baseline: `b3-repair-initial-status.txt`, `b3-repair-initial.diff` and six relative-path source copies under `b3-repair-source-baseline`. ZIP/7z writers, native source/output, router/transport/queue implementations, dependencies/vendor, geometry/palette and unrelated dirty changes are retained. Coordinator-owned records are unchanged.

## Executed verification

Absolute log root: `D:\.pi-desktop\data\scratch\0ffe6ffa-9e52-420b-af61-a29fdd0f37b4`.

Each Rust selection uses `cargo test --locked --offline -p PACKAGE TARGET FILTER -- --list`, then identical PACKAGE/TARGET/FILTER with `-- --nocapture --test-threads=1`. LIST and RUN commands are recorded in `b3-repair-final-list-results.json` / `b3-repair-final-run-results.json`; final Job prerequisites and results are recorded separately. Counts overlap, not a distinct-test total. Every Rust execution sets jobs1/incremental0/codegen-units256/scratch TEMP/TMP in the same PowerShell process and checks the single Cargo lane.

| Package / target / exact filter | Observed result | Log pair under the absolute root |
| --- | --- | --- |
| Desktop `--bin nxtools-desktop ref17_b3_` | LIST3/RUN3/PASS3/FAIL0/ignored0 | `b3-repair-ref17-new-list.log`, `b3-repair-ref17-new.log` |
| Desktop `--bin nxtools-desktop ref18_b3_02_` | LIST2/RUN2/PASS2/FAIL0/ignored0 | `b3-repair02-green-list.log`, `b3-repair02-green.log` |
| Desktop `--bin nxtools-desktop ref17_` | LIST8/RUN8/PASS8/FAIL0/ignored0 | `b3-repair-final-ref17-list.log`, `b3-repair-final-ref17.log` |
| Desktop `--bin nxtools-desktop ref18_` | LIST11/RUN11/PASS11/FAIL0/ignored0 | `b3-repair-final-ref18-list.log`, `b3-repair-final-ref18.log` |
| Desktop `--bin nxtools-desktop archive::forms::tests` | LIST48/RUN48/PASS48/FAIL0/ignored0 | `b3-repair-final-forms-list.log`, `b3-repair-final-forms.log` |
| Desktop `--bin nxtools-desktop archive::draft::tests` | LIST12/RUN12/PASS12/FAIL0/ignored0 | `b3-repair-final-draft-list.log`, `b3-repair-final-draft.log` |
| Desktop `--bin nxtools-desktop archive::application::jobs::tests` | LIST129/RUN129/PASS129/FAIL0/ignored0 with actual native wire exports | `b3-repair-final-jobs-with-wire-list.log`, `b3-repair-final-jobs-with-wire.log` |
| Desktop `--bin nxtools-desktop archive_polish_quick` | LIST3/RUN3/PASS3/FAIL0/ignored0 | `b3-repair-final-quick-list.log`, `b3-repair-final-quick.log` |
| Desktop `--bin nxtools-desktop tools::polish_tests` | LIST11/RUN11/PASS11/FAIL0/ignored0 | `b3-repair-final-tools-list.log`, `b3-repair-final-tools.log` |
| Core `--lib ref17_` | LIST2/RUN2/PASS2/FAIL0/ignored0 | `b3-repair-final-core-ref17-list.log`, `b3-repair-final-core-ref17.log` |
| Protocol `--lib ref18_` | LIST1/RUN1/PASS1/FAIL0/ignored0 | `b3-repair-final-protocol-ref18-list.log`, `b3-repair-final-protocol-ref18.log` |
| IPC `--lib` | LIST62/RUN62/PASS62/FAIL0/ignored0 | `b3-repair-final-ipc-list.log`, `b3-repair-final-ipc.log` |

### Native preservation and Job prerequisites

Existing executable: `D:\Code\NxTools.Next-main\csharp\NxTools.PlatformHost.Tests\bin\Debug\net8.0-windows\NxTools.PlatformHost.Tests.exe`. Every fixture is LISTed with its effects before identical `--fixtures "EXACT FILTER"` RUN; LIST adds `--list`.

- Native split LIST3/RUN3/PASS3/SKIP0/FAIL0/not_run0: exact filters `archive Phase E delivery ZIP seven volumes old family`, `archive Phase E delivery partial volume race facts`, `archive B3 explicit split sizes count cancellation`. Logs `b3-repair-native-split-list.log`, `b3-repair-native-split.log`. Original worker/native leases/publication, independent approved stock7Zip readback,1024-volume bound and pre/late-cancel facts remain.
- Raw join exact filter `raw numbered join`: LIST4/RUN4/PASS4/SKIP0/FAIL0/not_run0. Logs `b3-repair-native-join-list.log`, `b3-repair-native-join.log`. GUID-owned source/output/pipe, single64KiB one-pass transfer, writer refusal, source/family preflight, write112, cancellation/actual cleanup/exclusive reopen and original reader physical drain remain.
- Job prerequisite exact filters `archive OPT01 AC repair production progress wire` and `OPT03 create actual native result locations`: each LIST1/RUN1/PASS1/SKIP0/FAIL0/not_run0. OPT01 uses original in-memory Operation/OnFrame/two channels/serializer only. OPT03 uses GUID-owned160000byte source, copied current worker, original ZIP/7z/native publication/location/late-cancel/foreign-target refusal and awaited cleanup. Logs `b3-repair-job-progress-native-list.log`, `b3-repair-job-progress-native.log`, `b3-repair-job-create-native-list.log`, `b3-repair-job-create-native.log`.

For Job LIST/RUN, `NXTOOLS_ARCHIVE_AC_PROGRESS_WIRE=D:\.pi-desktop\data\scratch\0ffe6ffa-9e52-420b-af61-a29fdd0f37b4\b3-repair-job-progress-wire.json` and `NXTOOLS_OPT03_CREATE_WIRE=D:\.pi-desktop\data\scratch\0ffe6ffa-9e52-420b-af61-a29fdd0f37b4` supply those actual C# exports, including `ordinary.zip.json`, `split.zip.json` and `split.7z.json`.

Worker: `D:\Code\NxTools.Next-main\target\debug\nxtools-archive-worker.exe`. Independent reader: `D:\Tool\Directory Opus\Directory Opus\_Green\7z极限压缩\bin\x64\7z.exe`. No fixture output is executed; no product/reference/browser GUI, clipboard/Shell/system mutation or deployment is involved.

### Quality and source gates

- Six repair Rust files: `rustfmt --edition 2024 --check --config skip_children=true`, exit0; `b3-repair-final-rustfmt.log`.
- `cargo clippy --locked --offline -p nxtools-desktop --bin nxtools-desktop --tests -- -D warnings`: exit101, only the two retained unrelated download lint errors listed below; `b3-repair-final-desktop-clippy.log`. No warning suppression or unrelated repair.
- `python tests/architecture/spec-contracts.py`: PASS129documents/14owners; `b3-repair-final-spec-contracts.log`.
- `python tests/architecture/gpui-component-catalog.py`: PASS79historical pages/78mappings/current0.8authority; `b3-repair-final-gpui-component-catalog.log`.
- `python tests/architecture/gpui-release-contracts.py`: PASS Kit0.8.0/defaultpre0.3.9, exactly two carried narrow patches; `b3-repair-final-gpui-release-contracts.log`.
- `python tests/architecture/markdown-editor-contracts.py`: PASS57original editor files; `b3-repair-final-markdown-editor-contracts.log`.
- Resource audit2026-11-10T14:17:22.1873068+08:00:0Cargo/rustc/clippy/linker/Desktop-test/native-fixture/worker processes and0repair wrappers; `b3-repair-final-resources.json`. Owned native fixtures await actual terminal/DisposeAsync/worker drain and exclusive source reopen before returning. All current repair background commands are settled.

## Limitations

- Strict Desktop lint remains blocked by existing `downloads/browser_handoff_impl.rs:436 nonminimal_bool` and `downloads/settings_task_rem_tests.rs:612 unwrap_used`. The repair does not change either file. Other historical full-workspace failures are not recertified or silently cleared.
- Generic raw families cannot prove an unknown final part is present; original32GiB/1024part/native path limits remain.
- Source judgment, production TestPlatform and GUID-owned native/readback boundaries are distinct. Physical provider, actual Windows IME/DPI/OS-close/hotkey/Shell, human visual, installed/Release and full-workspace acceptance remain unperformed.
- No package/install/deploy replacement, registration/association, registry/service/power action, dependency/vendor upgrade, commit or push. Coordinator owns acceptance; stop at B3.
