# OPT-03 — CREATE-01/02 option contract

## Status and boundary

CREATE-01/02/03 and the touched CREATE-04/06 boundaries pass the authorized automated/owned-native checks under **Phase E — Format expansion and ZIP editing**. Original form/draft owners enforce capability-correct options; correlated destination observation precedes complete output planning and original batch admission. [OPT-03 results](opt03-intake-results.md) records exact evidence and limitations.

Reviewed references: `docs/reference-code/archive.md`, `docs/reference-code/archive-editing.md`, the archive backend specs, the OPT-03 task PRD/design/implementation/acceptance, saved Bandizip create preflight `opt03-bandizip-create-p-0x1400f6d10.txt`, and saved Ark option normalization `opt03-ark-create-p-0x18001fd80.txt`. The Bandizip material confirms no-source rejection, GZ/XZ folder and multi-input refusal, bounded split parsing and final option transfer; the Ark material confirms normalization of password/header/method combinations before writer execution. These are static reconstructed observations, not proprietary source reuse or runtime acceptance.
## Implementation

- `forms_layout.rs` now disables 7z file-name encryption until the password and confirmation are both nonempty and equal, with an explicit reason. `forms.rs` clears stale header intent when the secret becomes absent/mismatched.
- The volume field is visible only for ZIP/7z. `CreateDraft::set_format` and the stable Input entity both retire a prior volume value when switching to TAR/TGZ/TXZ/GZ/XZ, so a hidden stale option cannot block capture.
- `draft.rs` no longer rejects same-basename each-input outputs before the complete destination plan. Ordinary single-archive member collision rules remain in `CreationPlan`; this change is only to separate output archive names.
- `archive_create_destination.rs` matches C# `ArchiveCreateDestinationCommand/Result`: version/operation/session/generation/destination, `destination_missing`, `existing_names`, optional code/native error, strict unknown-field rejection, 16,384-name and 262,144-UTF-8-byte bounds, and private Debug.
- `create_output_plan.rs` registers pending before the correlated command, freezes connection/form/generation/requests/completion, handles timeout/disconnect/retirement, rechecks the submitted form and connection after defer, then returns the renamed immutable batch to the original `submit_form_admitted`/`validate_batch`/FIFO path. Diagnostics contain IDs/count-free state only, never destination paths or names.
- Planning reserves the observed disk namespace, every original requested output and every assigned generated output before submission. Matching is Windows-style case-insensitive; suffixes start at `(2)` and preserve compound extensions. A generated name cannot consume a later original request. ZIP `.zNN` and 7z `.NNN` families are collision reservations only; native per-volume publication and late collision checks remain authoritative.

`application.rs` registers the output-planning module; `Jobs.create_plans` holds bounded pending observations. Original submission, command results, disconnect and form retirement route through that module. No child job is admitted before the complete plan; `submit_form_admitted` retains original validation, FIFO and explicit refusal behavior.

## Existing control → typed request → writer mapping

| Concern | Existing route and observed contract |
| --- | --- |
| Seven formats | `forms.rs:1873-1893` maps the released format selector exactly to ZIP, 7z, TAR, TGZ, TXZ, GZ and XZ. `draft.rs:242-290` maps the frozen draft to `ArchiveJobCreate`; `ArchiveJobSession.cs:84-102` maps it to `create_zip/create_7z/create_tar/create_tgz/create_txz/create_gzip/create_xz`; `worker.rs:1195-1211` dispatches the actual writer. |
| Semantic levels and ZIP method | `forms.rs:318-339,405-459` keeps ZIP Store synchronized with the Store semantic level and exposes Fast/Normal/Maximum elsewhere. `draft.rs:139-169,261-273` omits level for TAR, rejects Store for non-ZIP/non-TAR and emits ZIP Store/Deflate. Writers map ZIP to 0/1/6/9 (`worker_zip_create.rs:92-113`), 7z to LZMA2 presets 1/6/7 (`worker_sevenz_create.rs:46-56,92-106`), TGZ to 1/6/9 and TXZ to 1/6/7 (`worker_tar_create.rs:92-103,125-139`), and GZ/XZ to 1/6/9 and 1/6/7 (`worker_create.rs:88-109,139-145`). |
| Encryption, solid and encrypted names | `draft.rs:116-129,274-278` requires password confirmation, restricts secrets to ZIP/7z and requires a password for 7z header encryption. `ArchiveJobSession.cs:86-104` converts the private UTF-8 secret and zeroes the protocol byte buffer after admission. ZIP maps password to AES-256; 7z maps password to AES-256 plus explicit solid/header flags. Actual writers apply WinZip AES-256 (`worker_zip_create.rs:120-127`) and 7z AES/LZMA2, solid grouping and header encryption (`worker_sevenz_create.rs:91-106,136-157`). |
| Password display and retirement | Production fields use masked official `Input.mask_toggle()` controls and confirmation (`forms_layout.rs:161-208`), and the containing field is visible only for ZIP/7z (`forms_layout.rs:351-355`). Format-discard clears both inputs before applying an unsupported format (`forms.rs:1040-1057`); form release/retirement clears draft and Input state (`forms.rs:519-524,930-951`). `PrivatePassword` is Debug-redacted and zeroed on drop; `WorkerRequest` zeroizes its private password on drop. Existing Desktop/protocol tests retain these regressions. |
| UTF-8 ZIP names | Create source names remain Rust UTF-8 strings. ZIP writer `start_file` receives the planned Unicode name directly (`worker_zip_create.rs:129-145`); existing independent ZIP evidence records general-purpose UTF-8 bit 11. The new worker regression checks a real non-ASCII local header name and bit 11. The request codepage remains the existing read/display setting and does not downgrade writer names. |
| Exclusions and single-root stripping | `draft.rs:279-286` projects these only for tree formats. `creation.rs:127-217,295-341` applies one shared bounded plan, strips only one directory root and omits excluded members before payload reads. All ZIP/7z/TAR-family writers consume that plan. The new regression checks a stripped Unicode directory and proves `Thumbs.db` never reaches `source_read`. |
| Source kind and output self-inclusion | Intake reads root metadata only, rejects reparse points/links and non-file/non-directory kinds (`forms.rs:1841-1871`). The bounded creation plan separately validates every relative name and duplicate type (`creation.rs:142-217,264-293`). Native `ArchiveCreationSources.RejectOutputCollisions` rejects source/final identity aliases and output directories at or below an input directory (`ArchiveCreationSources.cs:643-690`), and the runner repeats the held-output check immediately before execution (`ArchiveWorkerRunner.cs:754-771`). Existing source-binding/native creation fixtures own those real-handle regressions. |
| GZ/XZ cardinality | `draft.rs:131-137,238-290` rejects directories and multi-input single-output requests; separately-compressed regular files become one typed request each. Protocol validation independently requires one path and no tree options (`archive_job.rs:601-606`); the actual stream writer independently requires a regular file (`worker_create.rs:113-126`). |
| Split options | `draft.rs:171-190,290` admits bounded ZIP/7z sizes only. Rust protocol (`archive_job.rs:571-575`), C# contract (`ArchiveJobContract.cs:203-209`), C# admission (`ArchiveWorkerRunner.cs:164-205`) and worker validation (`worker.rs:226-232`) repeat that gate. ZIP uses genuine disk-relative splitting; 7z emits numbered encoded chunks; both stage/read back every part before publication (`worker_volume_output.rs:11-112`). The new worker regression combines AES ZIP split and solid AES/header-encrypted 7z split. |
| Frozen submission | `OperationForm::capture` synchronizes stable Inputs once, creates owned typed requests, retires draft secrets after successful capture and creates a separate remembered-settings value (`forms.rs:619-720`). `submit` accepts exactly that captured request and moves additional each-input requests without rereading controls (`forms.rs:751-820`). Existing Desktop tests cover once-only capture, rejection retaining the draft and saved defaults without secrets. |
| Illegal combinations | Rust protocol `ArchiveJobRequest::validate` (`archive_job.rs:557-608`), C# `ArchiveJobWire.Valid` (`ArchiveJobContract.cs:195-210`) and private worker `WorkerRequest::validate` (`worker.rs:409-501`) independently reject method/level mismatch, unsupported secrets/options, header encryption without password, tree options on streams, stream cardinality violations and split on non-ZIP/7z. |

No writer-chain defect was found in the admitted seven-format mappings above. Existing writer verification remains mandatory: every candidate is decoded/read back before source revalidation and commit; optional post-create Test is separate and cannot replace writer verification.

## New regression coverage

`rust/crates/nxtools-archive/tests/opt03_create_options_contract.rs` adds two independent tests:

1. `opt03_all_admitted_create_formats_reach_actual_writers_with_option_snapshot`
   - validates public typed requests and private worker requests;
   - starts the actual `nxtools-archive-worker` for ZIP, 7z, TAR, TGZ, TXZ, GZ and XZ;
   - covers ZIP Store/UTF-8/root stripping/exclusion, ZIP Deflate/AES/split, 7z Fast non-solid, 7z Maximum solid/AES/header/split, plain TAR, TGZ Fast, TXZ Maximum, GZ Normal and XZ Maximum;
   - answers the existing bounded native source/output rendezvous, requires complete writer readback and records only committed artifacts;
   - checks format signatures, split part bounds/names, ZIP UTF-8/encryption/method fields, excluded-source non-read, and—under `isolated-codecs`—7z header-password and solid facts.
2. `opt03_invalid_create_combinations_fail_at_typed_and_writer_boundaries`
   - rejects ZIP level/method mismatch, ZIP undersized volume, 7z ZIP-method leakage, 7z header encryption without password, TAR level/password/split, TGZ password/missing tree options, GZ multiple inputs/tree options and XZ tree options.

The fixture is generated inside the test's owned temporary directory. It does not use system clipboard, GUI, picker, Explorer, user archives, services, registry or installed state.

## UI and planning regressions

`forms_tests.rs::create_invalid_options_are_disabled_and_format_switch_retires_volume` drives production Select/Input/Checkbox controls. `draft_tests.rs` covers unsupported-volume retirement and duplicate each-input names reaching planning. Original forms41/41 and drafts12/12 pass. A bounded old-behavior comparison makes the real header control, volume retirement and same-name form submission regressions fail; restored final behavior passes.

`create_output_plan_tests.rs` passes15 tests, including original rendered Create submission, zero Job before destination observation, `report (3).gz`/`report (2).gz`/`report (4).gz`, case/family/compound/long names, malformed/overflow/native failure/timeout/close/late/deferred disconnect and full/near-full queue refusal. The supported8193-name collision regression passes its5-second bound with preindexed normalized occupancy.

## Replay commands

Every Cargo invocation sets the required memory environment in the same PowerShell call; use one Cargo lane:

```powershell
$env:CARGO_BUILD_JOBS='1'
$env:CARGO_INCREMENTAL='0'
$env:RUSTFLAGS='-C codegen-units=256'
$env:TEMP=$env:PI_SCRATCH_DIR
$env:TMP=$env:PI_SCRATCH_DIR
cargo test -p nxtools-archive --features isolated-codecs --test opt03_create_options_contract -- --test-threads=1
cargo test -p nxtools-protocol archive_job::tests:: -- --test-threads=1
cargo test -p nxtools-desktop archive::draft::tests:: -- --test-threads=1
cargo test -p nxtools-desktop archive::forms::tests:: -- --test-threads=1
cargo test -p nxtools-protocol archive_create_destination::tests:: -- --test-threads=1
cargo test -p nxtools-desktop create_output_plan_tests -- --test-threads=1
cargo clippy -p nxtools-archive --features isolated-codecs --test opt03_create_options_contract -- -D warnings
cargo fmt --all -- --check
python tests/architecture/spec-contracts.py
```

The focused archive tests passed2/2:9 actual writer configurations across7 formats plus invalid combinations. Their native replies are controlled; independent native interoperability is separately verified by the8 exact fixture selections below, all passed without skips (43 created outputs).

```powershell
dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore -m:1 -p:UseSharedCompilation=false
$hostExe = Resolve-Path 'csharp/NxTools.PlatformHost.Tests/bin/Debug/net8.0-windows/NxTools.PlatformHost.Tests.exe'
& $hostExe --fixtures --list 'archive ZIP creation fixture directory levels roots'
& $hostExe --fixtures 'archive ZIP creation fixture directory levels roots'
& $hostExe --fixtures --list 'archive ZIP creation fixture AES independent interoperability'
& $hostExe --fixtures 'archive ZIP creation fixture AES independent interoperability'
& $hostExe --fixtures --list 'archive tree creation fixture 7z options'
& $hostExe --fixtures 'archive tree creation fixture 7z options'
& $hostExe --fixtures --list 'archive tree creation fixture TAR plain'
& $hostExe --fixtures 'archive tree creation fixture TAR plain'
& $hostExe --fixtures --list 'archive tree creation fixture TGZ levels'
& $hostExe --fixtures 'archive tree creation fixture TGZ levels'
& $hostExe --fixtures --list 'archive tree creation fixture TXZ levels'
& $hostExe --fixtures 'archive tree creation fixture TXZ levels'
& $hostExe --fixtures --list 'archive stream fixture actual worker gzip creation'
& $hostExe --fixtures 'archive stream fixture actual worker gzip creation'
& $hostExe --fixtures --list 'archive stream fixture actual worker XZ creation'
& $hostExe --fixtures 'archive stream fixture actual worker XZ creation'
```

Required environment for those fixture selections:

- `PI_SCRATCH_DIR`: this session's owned scratch root.
- `NXTOOLS_ARCHIVE_WORKER_FIXTURE`: absolute path to the freshly built `target\debug\nxtools-archive-worker.exe` from the same source/feature state.
- `NXTOOLS_ARCHIVE_PYTHON_ORACLE`: absolute path to the approved CPython executable.
- `NXTOOLS_ARCHIVE_ZIP_FIXTURES`: absolute path to `rust\crates\nxtools-archive\tests\fixtures`.
- `NXTOOLS_ARCHIVE_7ZIP_ORACLE`: absolute path to the pinned stock 7-Zip executable used by `verify-created-tree.py` for 7z listing/extraction.

LIST and RUN filters must be identical. Any skip, zero selection or missing oracle is not a pass.

## Acceptance boundary

C# build and native LIST/RUN pass with the frozen matching worker; original source/output owners publish and clean up, and .NET/Python/stock-7-Zip independently decode the seven-format outputs. Native late-target appearance after observation reaches the original overwrite confirmation and cancellation preserves the target and source. Strict five-package all-target Clippy, relevant Desktop/protocol/IPC/CLI regressions and documentation/dependency gates pass. Physical GUI/pointer/focus/DPI/accessibility, real picker/clipboard/Explorer, user archives and Release acceptance remain separately authorized.
