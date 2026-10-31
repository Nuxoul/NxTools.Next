# Step3 — TAR-family editing outcomes

2026-10-28. **Phase E — Format expansion and ZIP editing**; only the third item of the user's seven-step list. Plain TAR then TAR.GZ/TGZ then TAR.XZ/TXZ implemented. Original A–F headings/order, accepted common foundation/ZIP/7z/HZ2 and old failures remain. **Implementation and scoped automated/owned-native delivery; pending coordinator independent acceptance. Overall PARTIAL**, not PhaseE/archive manager/first release completion. Stop here; no Step4/HZ3/HZ4/HZ5.

[Before-change route](tar-edit-route.md), [exact matrix/test names](tar-edit-matrix.md), [owner contract](../../spec/nxtools-archive/backend/tar-edit.md). Evidence root:
`C:\Users\Administrator\.pi-desktop\scratch\0c0e5320-11c4-40dd-bb50-f6a437188fc1\tar-edit`.

## 1. References, scope and owners

Actual clean Ark checkout `D:\Code\archive-edit-references\ark`, commit `f7ea89e828925244d0a6b76cbfb4ac5b9fafe2e5`; inspected AddJob/MoveJob/DeleteJob -> interface -> read/write plugin -> base reader -> filters/processOldEntries/copyData/finish. Actual inspected plugin/jobs headers BSD-2-Clause. Streaming old-to-new and filter separation are design references; Qt/UI/unchecked write_close/QSaveFile commit and read-error-as-EOF are **not copied**. Additional three entry hashes added to [manifest](../../../docs/reference-code/archive-editing-manifest.json); all22reference files rechecked (`reference-final-recheck.json`, `ark-final.json`). No Ark build/runtime claim, no external product codec/backend/native ABI/dependency upgrade.

New Rust owners: `tar_edit.rs` strict bounded raw header/extensions/catalog/plan; `tar_edit_outer.rs` separate signature/full outer decode/encoder; `worker_tar_edit.rs` streaming candidate/verification/original handoff. Existing Protocol/C# Modify request and runner/job routing gained Tar/TarGz/TarXz; optional typed `tar_edit` facts travel only on actual worker browser context, missing legacy facts fail closed. Final edit revalidates source/native/revision/structure regardless of caller facts. Desktop consumes completed isolated-worker facts, does not predecode TAR itself. Existing Browser/ModifyDraft/Form/queue/selection/pickers/progress/result/lease owners remain.

The original `ArchiveSourceLease.cs`, `ArchiveOutputSession.cs`, `ArchiveVolumeSources.cs`, `ArchiveCreationSources.cs`, ZIP editor and7z editor retain exact opening SHA256 (`protected-owner-hashes.json`). Only internal progress-stage API cfg was widened so TAR also compiles under default features; no global sharing/native publish change. Root Cargo files were not edited by this task; opening owner-hash inventory did not include them, so no unsupported opening-hash equality claim. Locked GPUI0.8.0/pre0.3.9 release/editor gates pass.

## 2. Actual supported/refused and cost/fidelity

| Area | Delivered behavior | Refusal / caveat |
| --- | --- | --- |
| Add | protected source-ID files/directories/zero file/empty directory; one candidate | new records uid/gid/mtime0, file0644/dir0755, no ACL/ADS/source-time conversion |
| Delete | original ordinals, subtree/overlap/all, a/ not ab/; legal independently read empty archive | never deletes original source; ordinary TAR payload has no checksum |
| ReplaceContent | one old regular ordinal plus one protected ordinary input; old name/metadata retained | directory/multiple/stale/EOF/extra refuses; ZIP ReplaceContent remains unavailable |
| RenameFile / DirectorySubtree | explicit/implicit/empty directories, Unicode, original descendant ordinal+prefix | duplicate/case/file-prefix/implicit-parent aliases, occupied target subtree/self-descendant/file-kind promotion refuse before native candidate |
| ustar/GNU ordinary/PAX | retained raw member headers/extensions; uid/gid/mode/owner strings/time values; GNU long paths; local PAX path,size,mtime,atime,ctime,uid,gid,uname,gname | global PAX/unknown semantic keys/links/devices/sparse/special/vendor structures refuse; not complete POSIX/NTFS fidelity |
| Gzip | exactly1member; preserve mtime/OS/name/comment; XFL0/2/4->level6/9/1 | FTEXT/FHCRC/FEXTRA/unknownXFL/multimember edit refuses; normal reader still validates all members/trailers; compressed bytes/layout may differ |
| XZ | exactly1stream, LZMA2-only filter, preserve check0/1/4/10; preset1 encode | filters/multistream edit refuse; normal multistream read unchanged; parameters/block layout change disclosed |
| UI/task | existing Modify commands/menu/replace picker, actual eligibility, five real phases | fourth-step interaction unification not implemented; processed!=committed; unknown totals/space stay unknown |

Every edit is complete rewrite; compressed edits fully decode/reencode. No path-tree extraction or whole-archive content Vec; finite anonymous decoded TAR/inner candidate/outer candidate/readback spools. Original encoded/decoded/payload/output/time/control limits, bounded entries/extensions/metadata/parent indexes and working-memory qualification remain. Extension <=64KiB/at most2pending distinct records. Original+candidate/spools coexist; no source deletion credit. Replacements/new records follow retained originals deterministically; original byte order/layout/compression ratio is not claimed identical. UI explicitly discloses this rewrite/parameter/space/new-metadata/checksum scope using existing Form/Field + official Small default-nonclosable warning Alert. Original outlined buttons/Root/titlebar/scroll/focus/deferred cross-entity ownership stays; compiled render bounds are not visual/user acceptance.

TAR header checks, full content comparison and SHA256 are transactional verification, **not an intrinsic TAR payload checksum**.

## 3. Transaction and diagnostics

Complete inner candidate/end -> checked finish/flush -> full exact names/kinds/sizes/raw headers/extensions/SHA256 verification -> checked outer finish/flush -> native held partial write/readback encoded hash -> complete outer decode and identical TAR/content checks -> actually close all old readers -> original acknowledged edit_reads_complete -> C# still-held source/target/candidate full checks -> final cancel -> release only original source FILE -> one original native publication. Parents/candidate remain protected. Approved final-check/close/path-publish TOCTOU is not ID-CAS. No delete-old-first/backup move/truncate/punch/global source sharing/recursive path cleanup/unknown retry.

Original queue/same-archive serialization/pause/cancel/active preview/drag/nested/read/edit lease policies retained. Correlation accepted/start/fixed planning/reading/reencoding/verifying/committing/terminal/elapsed/native5/64/112 facts cross original Rust/IPC/C# path. No names/paths/content/password/chunk diagnostics; byte samples coalesced, processed not committed. Native success outranks late cancel; unknown retains evidence and no automatic retry.

## 4. Red→green and failure history (never erased)

| Evidence | Actual result | Meaning |
| --- | --- | --- |
| `worker-red.log/json` |1RUN/1behaviorFAIL EOF, exit101 | plain TAR edit unavailable before implementation |
| `worker-plain-first.log/json` |compile E0689, exit101 | not behaviorRED; fixed explicit u64 |
| `worker-plain-second.log/json` |1PASS | plain vertical green |
| `compressed-red.log/json` |3RUN/1PASS/2behaviorFAIL, exit101 | compressed routes unavailable |
| `compressed-first.log/json` |20PASS |3new worker+stream12+xz5 initial green |
| `production-wire-red.log/json` |3RUN/3FAIL EOF, exit101 | real production password_interaction=true rejected; corrected whitelist |
| `matrix-first.log/json` |7RUN/4PASS/3FAIL |2fixture mistakes (invalid protected component, accidental //); real implicit target-directory merge defect fixed before write |
| `matrix-second.log/json` |13PASS (5unit+7matrix+1plain worker),2compressed names filtered | not claimed all3worker execution; later complete3PASS |
| `prefix-red.log/json` |1behaviorFAIL | forged directory-prefix could promote file; fixed and green in final lib |
| `edit-flush-mutation-red.log/json` |2RUN/2FAIL, exit101 | explicit sensitivity mutation ignoring final gzip/xz flush made actual edit-specific tests fail; mutation restored exact SHA `CDAE5BCD6E75ABC4231B50942F5EA659B6FC5240621B63EC791F4C18E98F916C` before final worker; `mutation-restored.json` |
| `integration-rust-first.log`, `dotnet-first.log` |compile errors | missing type qualification and nullable test cleanup; not behavioralRED |
| `desktop-archive-first/second.log` |default-feature compile cfg errors | internal stage setters now compile for TAR/default; no codec feature expansion |
| `desktop-archive-third/lowdebug.log`, `archive-final-configured.log` |LLVM OOM/0xc0000409 | system commit pressure; no test pass or code defect claim; no process killed. After memory naturally recovered, original Desktop command321PASS |
| `dotnet-final-build.log` |tool timeout600s | no duplicate build; survivors inspected, no matching build survived; recovered build0warnings/errors |
| `native-run-first.log/json` |4selected/3RUN/2PASS/1FAIL/1not_run | fixture after-hash read before releasing own writer; fixed cleanup order, final5PASS |
| `archive-final.log` |old stream fixture2FAIL missing env | fixture config failure, not productRED; correct hash-verified data later2PASS |
| `archive-final-recovered.log` |old output_readmission_chain1FAIL Cancelled vs SourceChanged | same source_changed scenario bounded targeted rerun1PASS; keep unified failure as unresolved instability, no assertion/budget change |
| `compat-native-final` |35PASS/6SKIP/1FAIL |6old7z native oracle path encoding configuration skips; corrected JSON-path exact6rerun6PASS. Known source_unavailable remains FAIL |
| first `gate-spec-contracts.log` |8documentation violations |outcomes not yet written + new scenario section headings; fixed docs, final gate separately recorded |

## 5. Final commands / exact counts / worker / oracles

Every Cargo invocation checked live shared cargo/rustc lane immediately before start; same process jobs1/incremental0/RUSTFLAGS=-C codegen-units=256; `--locked --offline`, serial tests, full-operation timeout. No Cargo timeout/duplicate build/name-kill. No delegation/new session.

- Final build: `cargo build -p nxtools-archive --features isolated-codecs --bin nxtools-archive-worker --locked --offline`, exit0 (`worker-final-build.log/json`). Frozen file **`tar-edit\frozen\nxtools-archive-worker-final.exe`**, SHA256 **`00FAFA51E9CDE82E3F468829F061CF950234D60D233F6C7F0BB11E7ADA1A8191`**. Source-built current target hash matched frozen; final C#5 and compatibility use explicit file/hash, not shared target assumptions.
- Archive lib+41integration targets selected via exact `archive-final-command.json`: consolidated **253PASS/1FAIL**,0ignored, one100kperformance filter. `archive-target-counts.json` records all42targets, exact names/logs/counts; `archive-final-recovered.log` + `archive-final-remainder.log`. One bounded `cargo test ... --test output_readmission_chain -- --test-threads=1 --nocapture`1PASS does **not** relabel unified failure green. Three separate rar/sevenz/tar-stream full coordinator chains not run.
- New TAR tests: **7unit +11integration** (eligibility1+matrix7+worker3), plus **2Desktop** and **5C# registrations**. Original ZIP record/unknown-delete/AES/ZipCrypto/ZIP64/method/volume and7z tests passed in the recorded targets; not a broad resource certification.
- `cargo test -p nxtools-desktop --locked --offline --bin nxtools-desktop archive -- --test-threads=1`: **321PASS/0FAIL/0ignored**, exit0 (`desktop-archive-recovered.log/json`). Original HZ1 18/icon-layout8/WAC12, preview/drag/control/result protections remain. No wholeDesktop/full-app test claim.
- `cargo test -p nxtools-core -p nxtools-protocol -p nxtools-ipc --locked --offline --lib --tests -- --test-threads=1`: Core93+7, IPC43, Protocol89PASS (`layers-final.log/json`).
- Strict `cargo clippy -p nxtools-archive -p nxtools-protocol -p nxtools-ipc -p nxtools-core -p nxtools-desktop --features nxtools-archive/isolated-codecs --tests --locked --offline -- -D warnings`: PASS exit0 (`strict-final.log/json`). `cargo check --workspace --tests --locked --offline`: PASS exit0 (`workspace-complete.log/json`). Scoped rustfmt42Rust paths PASS, `git diff --check` PASS (unrelated inherited CRLF notices are not changed files).
- `dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore -m:1`: final0warnings/0errors (`dotnet-complete.log`). Effects audit -> exact `--fixtures --list 'archive tar edit'` -> identicalRUN: **5selected/5RUN/5PASS/0SKIP/0FAIL/0not_run**, exit0 (`native-complete-*`). Native actions are real same-path publications;5/64/112 are explicitly injected existing-boundary failures, not system disk-fill/network outage tests.
- C# preserved42names individually exactLIST/RUN: consolidated latest **41PASS/1FAIL/0SKIP**, original6skip logs retained and fixed-path rerun6PASS (`compat-native-final/results.json`, `compat-sevenz-final/results.json`). Includes SevenZ9, ZIP5, space11, preview2, retained OLE/async/tree/read leases and directory-only result/control. The historical readmission source_unavailable1FAIL remains.
- Static source/docs/provenance: GPUIcatalog/release, Markdown57file provenance, archive-tree-root-drag3tests, PlatformHost test-selection PASS; final spec gate **116documents/14owners PASS**, exit0 (`gate-spec-contracts-final.log/json`).22reference-file hash check PASS. These are not native/user acceptance.

Independent tools: CPython **3.13.2** (`python.exe` SHA256 `7D96A4ED35D6E596CD9DD8933FEAFC66349CC21F75BE3B15C89FD336E50140C1`) generates/decodes TAR+gzip+xz and verifies exact contents/metadata. System **bsdtar3.7.7/libarchive3.7.7** SHA256 `92FAFED485AF0FE4C291457DC3C00D0241F6A136731BAB63EF18A3F0C949D37A` independently checks exact final member lists for native fixtures. Existing stock **7-Zip26.00** SHA256 `4BCC005758C5E604C0AAA257B6097846A4A708E62A9C0C7471FA360FAD863F1F` only retained7z compatibility oracle. No tool installs; no product use of these executables. Old stream compatibility fixtures were copied into this scratch only after manifest hash verification; producer/provenance retained (`compat-fixtures.json`). Final TAR fixtures are newly generated this round. Oracle secrets never logged/argv; TAR no password.

## 6. Source scope, drift and append-only checkpoint

Opening HEAD `97d9a9dd21c2fd7563af218d29fa002730cc2520`; saved full status and892owner hashes before changes. Detailed52product/test paths and final/opening hashes in `scope-paths.json` / `delivery-source-hashes.json`; many test files only add `tar_edit:None` compatibility, preserving preview/drag/WAC/HZ1 semantics. Docs: route/outcomes/matrix/new TAR spec, narrow archive/Desktop/platform indexes, reference note/manifest and sole checkpoint prefix. Existing dirty repository is not this task's attribution; never reset unrelated changes. Opening source/lease/ZIP/7z hashes match as above. Outside listed product/test paths, the5observed changed opening files were this task's documented owner/reference updates; no independent product drift observed in that sampled892file inventory. Unlisted/new parallel files are not fully covered by this comparison.

Checkpoint raw backup `checkpoint-before.bin`: **169655bytes**, SHA256 **`CB635E63E79A8E16702586943AF697E05856C4F5C34EB4B1CFC7E61B37415DE7`**. Only prepend current summary; verify entire saved byte sequence is an exact suffix. Original failures/A–F headings unchanged. Final prepend verification JSON is separate evidence, not rewriting any historical report.

## 7. Remaining / stop boundary

- Historical C# readmission `source_unavailable` independently reproduced again. Rust ZIP native-coordinator source_changed scenario had1unified failure (Cancelled) and1targeted pass; keep unresolved instability, not a new TAR pass or a hidden all-green claim. Original native source/volume/creation/output owners unchanged this task; no broad unrelated fix.
- Historical CLI `helper_untrusted` and unidentified FB-AC05 remain OPEN; no product/Explorer/IME/DPI/UIA/physical-pointer/visual-user/installed acceptance. Product self-app GUI refusal not bypassed.
- No exhaustive format/extension/filter/coder/resource cross-product, huge archives, physical disk-full, hard-kill/restart/power-loss/HZ4 or reclaim/HZ5. Bounded common supported structures plus explicit refusals only.
- No package/sign/install/product/Explorer launch/Registry/service/ACL/UserChoice/Recycle/user-archive mutation/commit/push/delegation. Test fixtures are self-owned GUID archives and exact original native boundary probes only.

**Step3 implementation handoff stops here for coordinator independent acceptance. Do not start Step4/HZ3/HZ4/HZ5.**
