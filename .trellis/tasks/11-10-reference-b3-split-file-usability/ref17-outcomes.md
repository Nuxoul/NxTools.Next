# REF-17 — Exact split sizes

Status: **accepted at source, production TestPlatform and owned-native fixture boundaries**. [Coordinator acceptance](coordinator-acceptance.md) closes B3-01/E1/E2 with independent REF17 8/8, forms48/draft12/Job129 and retained native split3/3 evidence. [Repair evidence](acceptance-repair.md) and initial source/native results remain below. Archive remains **Phase E — Format expansion and ZIP editing / in_progress**; B4–B7 remain paused.

## Production chain and decisions

`OperationForm::create/new` → stable NumberInput/Select states → deferred local draft synchronization → `CreateDraft::options` checked byte snapshot → original ArchiveJob → C# ArchiveJobSession/worker runner → original distinct ZIP-coordinate / continuous7z writers → original all-candidate/source/readback checks → per-volume native publication/journal/cleanup.

The original680×600/min480×360 window, source viewport, options scrolling and fixed footer remain. Official Kit0.8 Form/Field/NumberInput/Select use Medium32 controls and public slots. Presets are 不分卷、自定义、100MiB、700MiB、2GiB、4GiB. Unit changes preserve original parsing/minimum/range errors; exact-divisibility guidance applies only to valid bytes with a remainder. Rejection retains old text/unit/entity; exact switching keeps bytes. Unsupported formats retain split intention and block Submit until explicit correction. The byte value is checked once per request snapshot.

Optional Core `create_volume_size { value, unit }` treats missing unit as MiB and missing preference as no split. Opening does not write config. Original section transaction merges only changed Create leaves against its baseline; size/unit is one leaf. Quick ZIP/7z explicitly keeps original one-click no-volume behavior despite saved form defaults. ZIP/7z writers and native publication code are unchanged.

## Regression evidence

Log root: `D:\.pi-desktop\data\scratch\0ffe6ffa-9e52-420b-af61-a29fdd0f37b4`.

| Boundary / exact selection | Executed evidence |
| --- | --- |
| Production format retention and changed-leaf save | `b3-ref17-red-list.log` LIST2; `b3-ref17-red.log` RUN2/FAIL2: unsupported format erased intended size; stale untouched Create preference overwrote latest disk state. Both pass in subsequent original-boundary tests. |
| Core `cargo test -p nxtools-core ref17_` | `b3-ref17-core-list-final.log` LIST2; `b3-ref17-core-green.log` PASS2: missing-unit MiB/load-without-write, exact unit persistence/reopen, integer bounds and concurrent changed-leaf save. |
| Desktop `cargo test -p nxtools-desktop --bin nxtools-desktop ref17_ -- --test-threads=1` | `b3-list-ref17_.log` LIST5; `b3-final3-ref17_.log` PASS5, including real pointer/typed2048MiB→keyboard2GiB, unchanged2147483648bytes and no outer Enter-submit, at default/minimum viewport with real scroll and32DIP footer/control bounds. |
| Original form/draft/quick paths | LIST logs `b3-list-archive__forms__tests.log`46, `b3-list-archive__draft__tests.log`12, `b3-list-archive_polish_quick.log`3; identical RUN filters in `b3-final3-archive__forms__tests.log` PASS46, `b3-final3-archive__draft__tests.log` PASS12, `b3-final3-archive_polish_quick.log` PASS3. Counts overlap the REF-17 selection. |
| Matching isolated-codecs worker | `b3-worker-build.log`: `cargo build -p nxtools-archive --features isolated-codecs --bin nxtools-archive-worker` passed. |
| Original native fixtures | `b3-split-native-list-final.log` LIST3 → `b3-split-native-run.log` RUN3/PASS3/SKIP0: `archive Phase E delivery ZIP seven volumes old family`, `archive Phase E delivery partial volume race facts`, `archive B3 explicit split sizes count cancellation`. |
| Exact count-limit result | `b3-split-limit-build-list.log` LIST1 → `b3-split-limit-final.log` RUN1/PASS1/SKIP0, overlapping the third native fixture. |

Native effects are GUID-owned scratch sources/outputs, original Job/worker/native handles and the existing approved stock7Zip oracle. Both formats produce real66560byte/65KiB volumes and byte-exact independent readback. Existing-family refusal retains old output; foreign-target race preserves truthful per-volume publication facts. Before-publication cancellation removes owned incomplete output; late cancellation retains committed facts.64MiB Store ZIP at64KiB split size returns exact `resource_limit` for>1024volumes before publication. Sources remain. No whole-family atomicity is claimed.

Independent reader: existing `D:\Tool\Directory Opus\Directory Opus\_Green\7z极限压缩\bin\x64\7z.exe`, used only as oracle. Matching worker: `D:\Code\NxTools.Next-main\target\debug\nxtools-archive-worker.exe`. Existing fixture provenance remains in the archive owner records; no tool installation or reference GUI launch occurred.

## Changed paths

Core `archive.rs`, `archive_tests.rs`; Desktop `archive/{draft.rs,draft_tests.rs,forms.rs,forms_volume.rs,forms_layout.rs,forms_tests.rs,jobs.rs,job_tests.rs,launch_entry.rs}`; test host `ArchivePhaseEDeliveryTests.cs` and its original Program catalog. B3 design/reference/owner-plan records preserve the current phase and accepted B1/B2.

## Remaining acceptance levels

Physical provider, IME/DPI, human visual, installed/Release and full-workspace acceptance are not established by these checks. [Final B3 results](outcomes.md) record scoped lint and affected cross-layer verification, including retained unrelated strict-lint failures. No product/reference launch, packaging/install, registry/service, commit or push was performed.
