# Archive compatibility and preview budgets — 2026-11-06

## Delivered behavior

- Selected previews keep text64KiB/image8MiB output limits independent from encoded-source, directory admission, solid decode work and intermediate ZIP/TAR spools. Real 80-byte members in larger 7z, split ZIP and CAB sources preview successfully; oversized selected members remain refused.
- Continuous ZIP `.zip.001` and signature-bound `.001` families enter the original native all-member source lease, retain global ZIP coordinates and use only the exact admitted paths. Missing members retain `missing_volume` through actual cleanup and the public terminal. Existing genuine `.zNN/.zip`, 7z and RAR ownership stays intact.
- py7zr solid archives with interspersed empty records use a bounded decode layout with original-ordinal mapping. The independent 38-record fixture is completely visited. PPMd/BZip2/Deflate/BCJ2 7z methods are enabled with working-memory admission.
- Isolated readers cover CAB MSZIP/LZX, ISO9660/Joliet/Rock Ridge, UDF, WIM, AR, CPIO and standalone LZMA/LZ4/Brotli. Multi-image WIM preserves image-local identity under numeric image roots. CPIO supports binary byte orders and retains link/special metadata without publishing it as regular content. LZ4 block buffers are working-memory charged before allocation.

## Verification

| Check | Result |
| --- | --- |
| Archive library | 98/98 |
| Additional-reader worker contracts | 5/5, including 14 supplied format cases, binary CPIO, two-image WIM, corruption and failed-write boundaries |
| ZIP volume contracts | 6/6 |
| Existing ZIP boundary / crypto contracts | 2/2 and 5/5 |
| Original C# Read/Entry/Job/native fixtures | LIST10, RUN10, PASS10, SKIP0, FAIL0 |
| Archive protocol / IPC regressions | 54/54 and 17/17 |
| Clipboard portable archive / extension import preservation | 5/5 and 8/8 |
| Current-source Quick Launch first-batch visibility/action regression | 1/1 |
| Archive all-target isolated-codecs Clippy | Passed with `-D warnings` |
| C# build | Zero warnings/errors |
| Scoped Rust formatting / diff whitespace | Passed |
| Spec navigation, Quick Launch lifecycle, pinned GPUI/editor source gates | Passed |

The native corpus comparison uses original source bytes, committed-file counts and exclusive source reopen. Thirteen of the fourteen additional-format samples complete full extraction. The bsdtar odc CPIO instead verifies listing/Test, zero-publication UnsafePath refusal and exact safe-member preview because the producer has already substituted `?` in names. An additional system-makecab fixture verifies the small-member/large-peer preview boundary.

Controlled worker output replies are separate from C# native evidence. These results do not certify physical GUI/DPI interaction, every format variant, broad resource performance or the overall archive-manager phase. Existing component/layout/variant and GPUI lifecycle owners remain unchanged.

## Remaining limits

- Quick Launch's production provider now publishes file-kind candidates before directory-kind search finishes. Actual production-worker/framed-IPC and Desktop consumer tests pass; installed input-to-first-paint latency remains unverified. [Measured delivery and limits](2026-11-06-large-count-and-launcher.md).
- The supplied70,000-entry ZIP completes through the original production Job/native pipeline, with every filename/payload verified, bounded summary/frame/logs and postcommit cancellation/late-corruption cleanup. Debug elapsed373,444.161ms is capacity evidence, not Release throughput. [Reference and runtime comparison](../reference-code/archive-large-count.md).
- Irreversibly substituted or unsafe Windows names are not silently renamed. Damaged archives, conflicting internal names and invalid paths retain typed refusal.
- The supplied file named `wim-lzms.wim` is actually an uncompressed single-image WIM (flags0x00000080, chunk_size0). Its successful native comparison is not LZMS runtime acceptance. The new WIM dependency's LGPL distribution obligations are recorded in the reader reference and must be satisfied before distribution.
- Workspace-wide rustfmt reports existing differences in `archive/application_tests.rs`, `archive/browser_drag.rs`, `archive/browser_drag_tests.rs` and protocol `archive_virtual_drag.rs`. Those unrelated edits were preserved; this slice's Rust files pass scoped formatting.
- No Release package, installation, service replacement or commit was performed. Changes remain in the working tree.

The separately requested large resident-memory investigation is recorded in [PlatformHost memory observation](2026-11-06-platformhost-memory.md). Its ten-minute sample did not reproduce the original19GiB private-commit incident; allocation ownership remains unconfirmed.

## Reproduction inputs

Use the supplied `D:\Temp\archive-tests` through `NXTOOLS_ARCHIVE_COMPAT_CORPUS`, the current isolated worker through `NXTOOLS_ARCHIVE_WORKER_FIXTURE`, and GUID-owned temporary roots under PI_SCRATCH_DIR. TEMP/TMP point at that scratch directory. Every Cargo command uses the workstation's required single-job/nonincremental/codegen settings. The compatibility tests never mutate supplied sources.
