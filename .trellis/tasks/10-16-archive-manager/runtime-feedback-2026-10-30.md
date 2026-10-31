# Runtime feedback — 2026-10-30

## Current behavior

| Surface | Cause | Implemented behavior |
| --- | --- | --- |
| Quick Launch | Current nonterminal indexed rows were accepted but withheld from the panel; row availability was coupled to all-provider completion. Extension publication inherited the same pending state. | Nonempty current local/index/extension rows publish and execute before terminal completion. Previous-query retained rows remain disabled. Original ranking, final merge,65ms input coalescer, generation, provider validation and close-on-acceptance remain. |
| Clipboard | The legacy empty-query branch treated unmodified digits as indexed paste. | Ctrl is required on digit down; plain digits enter search even when empty. Original native reveal/activation/search-focus chain remains. Initial-focus coverage uses no fixture focus repair. |
| Archive preview layout | Automatic preview still allocated the right pane before a previewable selection. | No initial preview slot, handle or reserved width. Supported single-file selection adds the original official resizable panel; empty/unsupported/multiple/directory selection removes it. Original controller, passive focus and cancellation remain. |
| Archive preview read | Product PasswordInteraction=true made no-output whole-archive TEST preparation inherit the selected text member's64KiB output quota. | Preparation uses existing general decode-work allowances; selected output/decode quotas remain unchanged. Browser TAR/non-password codecs use the original attempt. Encoded source, preparation, intermediate spool, selected output and decoder have separate resource owners. |
| Archive drag-out | Root, expanded-descriptor, file, cumulative content-request, cache-lease and512MiB drag gates rejected ordinary complete directories. | Remove those business gates across Desktop/protocol/native/cache/worker. Share frozen source context once, preserve complete original ordinals, and validate the real1MiB envelope, native representation and existing general extraction resources. |

The128-item refusal was a product rule, not a Windows OLE requirement. No replacement small item cap is introduced. Active workers remain2; logical descriptors, live leases and repeat content requests are not worker concurrency.

## Ownership and representation

- Rust freezes original source/selection and owns current-query/current-selection state; GPUI renders released components; C# owns HWND, OLE STA, streams, source leases, worker Job and native cleanup.
- Full and compact formal/alias wire shapes preserve the original source contract. A member cannot override shared source context. No selected root or descendant is silently omitted.
- Presentation-only collision naming keeps original paths and ordinals. Suffixes shorten the presentation stem at character boundaries to fit existing native component/path representation.
- Known file sizes preserve64-bit values; unknown sizes and overflowing aggregate estimates stay unknown, not zero. Actual output is charged independently.
- Materialize uses existing general32GiB-entry/100GiB-output allowances. Preview8MiB/text64KiB limits remain. Source/path/integrity checks, encrypted authorization, cancellation and held-stream final release remain.
- The preview panel is conditionally added as the last child of the existing official split. This keeps hidden slot sizes out of drag calculations and retains the original Tree/details handles in both list modes.
- Existing correlated lifecycle diagnostics remain; provider completion is distinct from current row availability. No query, archive contents, passwords or Clipboard payloads are logged.

## Reference comparison

[Listary/SwiftList research](../../../docs/reference-code/quick-launch-research.md) records the actual inspected entry points. SwiftList publishes local/network callbacks before terminal completion. Listary's inspected managed default launcher waits on Task.WhenAll before ranking/replacing results; its native search engine source and ordinary query latency were not available. Neither observation justifies withholding already accepted current rows or assuming seconds-long ordinary latency is normal.

The archive reference records the actual [preview preparation and split](../../../docs/reference-code/archive.md) and [native drag/wire/content lifetime](../../../docs/reference-code/archive-virtual-drag.md). Clipboard's [input reference](../../../docs/reference-code/clipboard.md) retains the original native show/activation/focus chain.

## Verification

| Check | Result |
| --- | --- |
| Related Desktop Quick Launch/Clipboard/Archive tests |546 passed,0 failed,0 ignored |
| Protocol virtual drag |11 passed |
| IPC connection/cancel virtual drag |3 passed |
| Browser-worker admission |1 passed |
| General engine/extraction-plan protections |15+5 passed |
| Controlled native virtual tree/async/password/cache/preview fixtures |26 selected,26 run,26 passed,0 skipped,0 failed,0 not_run |
| Matching isolated-codecs Debug worker |Build passed |
| C# PlatformHost test build |Passed,0 warnings,0 errors |
| Strict affected Protocol/IPC/Archive/Desktop Clippy |Passed with warnings denied |
| Scoped Rust formatting |17 files passed |
| Specification navigation/evidence/contracts |119 documents,14 owners passed |
| GPUI release and pinned-editor provenance |Both source gates passed |
| Scoped tracked-file diff whitespace |Passed |

Coverage includes:

- Actual GPUI Input/coalescer/command/current nonterminal batch/list bounds/action/Enter and real local-row pointer dispatch before index completion; stale/empty/late/closed-window/extension ownership remains tested.
- Initial Clipboard search focus without test-side repair; empty/nonempty plain-digit text; exact Ctrl-digit paste, Ctrl-first release, IME/editor/menu and immutable item/generation checks.
- Initially absent preview, supported/unsupported selection transitions, real list viewport widths, both Tree splitter regressions, tab/close/cancel and stale-result isolation.
- Real80BZIP/TAR preview through product PasswordInteraction=true, exact output, genuine oversized selected refusal/no published path, source failure preservation and lease release.
- Complete256-file directory/257-descriptor wire and held gesture,256 empty directories,64 selected roots, escaped shared comments, native256 indexed memory streams and a legal257th repeated request.
- Real256-file ZIP with34 distinct first/middle/last worker demands across the old32-lease boundary; repeated reads and transferred streams survive retirement. Duplicate ZIP/7z ordinals and unknown-size Zstd retain real worker/native-stream coverage.
- 8GiB+17 descriptor metadata verifies native high/low size fields; it is not a GiB payload test.

## Test and record maintenance

Replace obsolete expectations for empty-query plain-digit paste, all-provider-pending current rows and tiny drag-count/size gates. Keep actual source/path/integrity/native-name/frame/resource/password/stale-generation and transferred-stream protections. No valid failing lifecycle test is deleted.

[Release38 repair](release38-drag-preview-repair.md) remains the historical earlier slice; its counts are not this task's verification. Installer records, historical evidence and unrelated working-tree changes remain untouched. No package, installation, installed-product launch or HZ5 operation is part of this change.

## Limitations

- Quick Launch Desktop availability is verified. The ordinary indexer service still computes a complete result set before transport batching; live end-to-end service query latency and the user's entire seconds-long delay are not measured or certified resolved.
- Controlled GPUI and native COM/STA/worker fixtures do not certify physical Explorer/foreign-target drag, real HWND capture/focus, mixed DPI, installed behavior or user visual acceptance.
- The exact archive/member behind the latest reported failure is not established. Existing recent-state ZIP metadata confirms realistic ordinary inputs; private paths and contents are not published.
