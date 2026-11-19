# REF-20 — ZIP-SFX delivery

Acceptance: **accepted for the declared Windows x64 ZIP-SFX profile at the coordinator's recorded levels**. [Coordinator acceptance](coordinator-acceptance.md) records independent creation/readback, reader/native/TestPlatform boundaries and audited recipient receipts.

The original Create → immutable Job → worker → native publication chain creates ordinary and genuine split clear Store/Deflate ZIP-SFX. The official Create form exposes **创建自解压 ZIP（EXE）**; intent is default false, captured once, capability-negotiated and never silently converted to ordinary ZIP. Parent **Phase E — Format expansion and ZIP editing** remains open.

## Recipient and output form

The835,584-byte project-owned static Windows x64 recipient uses existing locked Deflate/CRC/tempfile primitives, embeds project/dependency/Rust notices and imports only Windows system DLLs. It requires no installed NxTools, PlatformHost, .NET, dynamic CRT or external unzip. Exact provenance is in [source review](ref20-source-review.md), [design](ref20-design.md) and [native execution](ref20-native-execution.md).

An exact48-byte descriptor follows the bounded PE raw image. Physical ZIP local/central/end/ZIP64 offsets include the prefix. Ordinary output is one `a.exe`; split output is `a.exe`, `a.z02…`, final `a.zip`. The complete prefix precedes the disk0 span marker; records remain intact and payloads may cross volumes. The first EXE is limited to4,294,967,295physical bytes by reviewed own-loader evidence. Split size must contain at least835,636bytes plus actual intact-record admission. ZIP64 coordinate evidence uses small forced fixtures, not a large-payload claim.

The recipient accepts clear Store/Deflate, UTF-8/ASCII names, regular files, directories and empty files. Read-only Test, explicit absent-root extraction and licenses are supported. It never launches archive members or system actions. Fresh-root/catalog preflight, complete consumption/size/CRC, fixed buffers and no-replace file publication retain truthful completed/incomplete output facts.

## Verified boundaries

| Boundary | Authority |
| --- | --- |
| Recipient parser/publication |22unchanged final unit PASS; strict bin lint, scoped format, static build, import and notice evidence |
| Immutable intent |2Rust intent selections and1C# wire selection PASS |
| Physical layout/original reader |14unique PASS, plus1same-test directory-attribute RED→GREEN regression |
| Native protected family |4same-test selections PASS: ordinary/split entries, missing/surplus/corrupt and finite-admission cleanup |
| Actual Create/publication |8unique PASS: ordinary/split, fresh-family conflict, cancel, native112, partial journal, missing/corrupt and complete-prefix limit |
| Exact selected output | Positive34-byte extraction and isolated34-byte member/33-byte total refusal PASS on all four actual forms; encoded prefix preparation does not grant output credit |
| Original Create/completion |7unique TestWindow/TestPlatform PASS: real pointer/Tab/Space/Enter, immutable naming/format/password/busy/close and original post-create Test publication/cleanup |
| Independent actual-output oracle |4profiles,8physical files,20entries,3,623,188decoded bytes; CPython physical parsing/raw zlib/CRC/exact bytes, plus stock zipfile ordinary |
| Controlled recipient execution |18specification-built and12actual-product reviewed command outcomes, including required existing-root refusals;4separate loader observations |
| Converged preservation |215fresh PASS; exact counts, quality and attribution in [verification](verification.md) |

Creation successes are composed5+1+1+1; UI/completion successes are5+1+1. Failed-only resumptions preserve earlier success rather than asserting a single final all-PASS run. Expected existing-root refusal is exit1, not a successful extraction. RED/fixture/compiler/lane records remain distinct in raw evidence.

## Ownership and lifecycle

The original CreationPlan/native ID reads/source digests/encoder finish/full held-token readback remain authoritative. `zip_container` performs coordinate-aware prefix layout; `worker_volume_output` stages and verifies all held candidates before publication. Native fresh-family admission, CREATE_NEW/no-replace, source_precommit and sequential journal facts are preserved. Later failure or cancel retains real commits and removes only owned unpublished candidates; no family-atomicity or blanket rollback is claimed.

Small official Form/Field/Checkbox composes with original InputState/Select/Root and fixed footer. Checkbox callbacks update local draft state and defer name InputState synchronization. Original Job completion tests use the committed EXE or final ZIP as Test data, retain creation publication through post-Test failure/cancel, and keep the running slot for incomplete cleanup. They never execute the created EXE.

## Frozen evidence

[Implementation audit](evidence/ref20/implementation/audit.json) and [manifest](evidence/ref20/implementation/files.txt) cover **4070files excluding the manifest**, including3634raw files. Sources, qualified fixtures, actual native outputs, expected bytes, build/command/environment/LIST/RUN/exits, independently reviewed native effects, point-in-time tested-source copies, original B7 attribution and one bounded no-findings source review are retained. The exact implementation path inventory is [owned paths](evidence/ref20/implementation/ownership/b7-final-owned-paths.txt); shared-file membership does not claim unrelated lines.

The three final task records `ref19-outcomes.md`, `ref20-outcomes.md` and `verification.md` are separate task-created handoff documents. Live task/status/reference summaries supersede point-in-time status text inside immutable evidence; frozen behavioral receipts remain unchanged.

## Limitations

SFX passwords, other ZIP methods/special types, RAR/ZIPX creation and split-set editing are not supported by this profile. No ordinary ZIP or7z behavior is replaced. Loader-size observations do not prove multi-gigabyte decoded correctness; small forced ZIP64 establishes coordinates only. Abrupt interruption may leave owned partial recipient output; handled failure cannot promise set-wide rollback.

Three original Protocol/Desktop strict-lint diagnostics remain separately attributed, with no suppression or unrelated edit. Physical UIA/HWND/IME/DPI/user visual approval and installed/Release acceptance remain separate. Independent acceptance is limited to the declared profile and recorded evidence levels. No product package, installed replacement or system configuration is part of this delivery.
