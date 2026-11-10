# Bandizip nested entry and archive replacement

## Connected entry-to-parent workflow

The inspected Bandizip7.32.0.1 path opens an inner archive as an extracted temporary member in another Bandizip process. The parent retains the source/member/temp relationship and the child process handle. After child exit, it compares the temporary file's last-write time and asks whether to apply a changed file to the originating archive. Confirmed writeback enters the original ZIP edit operation and completion path. This is a sequence of parent/child operations and explicit decisions, not one recursive transaction across all ancestor archives.

These are read-only Ghidra12.1.4/static PE observations of `D:\Tool\Bandizip\Bandizip.exe`; no nested archive, child executable or modification was run. Complete target-bound records are in [the local evidence archive](evidence/README.md).

## Open with Bandizip is a separate action

The list popup menu resource173 binds `POPUPMENU_LISTCONTROL_OPENWITHBANDIZIP` to command `0x81ef`. Main command handler `0x14013fc40` uses its command table to select case `0x20`, which invokes `0x140156ce0(browser,3)`. The table byte at `0x1401415c4` confirms this case mapping. The compiled UI action is distinct from the modes used by associated Open or external Edit.

`0x140156ce0` validates the open catalog and selected list row, resolves that row to its original member index, handles folder navigation separately, then calls `0x1401499f0` with the requested open mode. That routine extracts the selected regular member to a temporary filesystem location through the archive interface/progress owner before calling `0x14014a3c0`. Extraction failure does not launch the child.

`0x14014a3c0` supplies the current archive path, member/temp context, password context when relevant and mode to the retained launch record through `0x140161bd0 → 0x140161eb0`. For mode3, `0x140161eb0` obtains the running application's own executable path using `0x140022cc0 → GetModuleFileNameW`, builds the executable/temp-file command and calls `CreateProcessW`. On success it retains the process/thread handles and source/member/temp fields; failure returns an execution error instead of inventing an opened entry. The mode2 configured-editor and ordinary Shell-associated branches are separate.

The record retains path/context relationships; these observations do not establish a native held-original-object identity capability or a generation-scoped content lease equivalent to NxTools'.

## Child completion and explicit writeback

Process polling `0x140163120` checks `GetExitCodeProcess`, leaves a still-active child alone, and closes completed process/thread handles. It opens the tracked temporary file and compares its last-write `FILETIME` with the recorded value. A changed file enters the pending-modification queue; unchanged-file cleanup follows the configured temp-lifetime path. This is exit-time modification detection, not continuous filesystem watching or an unconditional writeback on every save.

The owner path `0x14013ae70 → 0x14013b470` checks the pending record against the active archive context, restores/raises the owner when required and asks `TEXT_ASK_APPLY_MODIFIED_FILE`. A non-Yes answer clears the pending record without starting the edit. Yes constructs the changed-file edit context and calls `0x14014f8d0`.

`0x14014f8d0` captures the browser location, checks update admission, derives `.temp` output, imports the current ZIP into the compressor plan, adds the changed member data, and runs the existing compression/edit coordinator `0x14013be50`. It releases the worker interfaces before completion. The admitted successful path enters `0x140154650`; an earlier error path deletes its temporary archive output. General edit restrictions remain in `0x140154360`: an opened writable ordinary ZIP is different from a readable nested archive, split archive, SFX or broken input.

Each parent can observe its own tracked child and ask to update its own member. The connected mechanism does not automatically commit all outer levels, turn a read-only outer format writable, or make a multi-level edit atomic. A modified inner archive must pass the applicable parent admission and confirmation at that boundary.

## Completion before replacement

`0x140154650` freezes the current source/browser context, closes prior reader state and opens the generated temporary archive through the original open-worker boundary. Its branches distinguish cancellation, unsuccessful open and the configured edit-test result. A canceled temporary open deletes that temporary output. An unsuccessful open reports an archive error and reopens the original source; the inspected path does not enter replacement. If post-edit testing is enabled and fails, it deletes the temporary output and reports modification failure before moving the original.

Opening a new archive and, when configured, testing it are therefore separate from publishing it at the original path. The compression worker's success alone is not the replacement result.

## Actual replacement and recovery attempt

After preparation succeeds, `0x140154650` closes the temporary reader and derives an unused backup name from the original path, starting with `.old` and checking numbered alternatives. It then performs:

1. Original path → backup path through `0x140122080`.
2. Temporary archive path → original path through the same helper.
3. Reopen the published original path, refresh browser/location/row context and restore list focus when possible.
4. Request deletion/recycling of the backup through `0x140121f60`, then settle source-delete lists where applicable.

`0x140122080` is a direct `MoveFileW` wrapper with an application activity flag around the call. It does not perform an identity-conditioned replacement, a durable journal commit or a single atomic swap of all three names. There is a visible intermediate state after the first move and before the second.

If moving the temporary archive to the original name fails, completion captures the native error, calls `MoveFileW(backup,original)` to attempt restoration, reports modification failure and tries to reopen the original path. The restoration call's return is not checked in this body. The original may remain at the backup name if restoration itself fails; this is not a verified rollback guarantee. Failure of the first move does not authorize deleting the old archive merely to make publication succeed.

On successful replacement, `0x140121f60` uses the configured recycle path or `DeleteFileW` fallback for backup cleanup. Its return is not consumed by the successful completion branch. Backup cleanup and successful new-archive publication are therefore different outcomes; a leftover backup is not evidence that encoding failed.

The inspected completion also has distinct handling for edit actions with optional source deletion. It does not make destructive follow-up actions part of the two-move publication primitive. No crash/restart discovery or durable recovery contract was established by these functions.

## Implications for NxTools

- Preserve the original member/source identity and explicit parent-child context when opening or editing an inner archive. A temporary path alone is not authority to update whichever archive is currently visible.
- Separate child lifetime, pending-change presentation, edit execution, verification and final publication. Declining writeback must not silently apply it later.
- Represent inner-to-outer updates as their actual per-parent operations and confirmations, not a fictitious all-level transaction.
- Retain NxTools' stronger native held-source/output identity, finish/verification, publication and cleanup boundaries. The useful reference is the user flow, not the direct path-based two-move implementation.
- Report successful encoding, successful replacement, restoration failure and backup/source cleanup separately. Do not describe the best-effort move-back branch as crash-safe rollback.

## Evidence map

Records are relative to `evidence/raw/`; exact target/provider/query identities are in [the index](evidence/index.json).

| Boundary | Canonical file | Evidence ID |
| --- | --- | --- |
| Popup command/dispatch | `bandizip-create-p-0x1401401fb.json` | `ev_62909cf3d746bcd5b408319b252fc9d385facb256bfa349b3e90ce32ef35288e` |
| Dispatch byte for command0x81ef | `bandizip-create-bytes-0x1401415c4-1.json` | `ev_fc4cb7faaa3a8d87d28e76c18f768122d5c9e603544f07f0b068b4f8ba318b41` |
| Selected row/member mode | `bandizip-create-p-0x140156ea1.json` | `ev_a3f8fedddb98a2e20bda53693cd1b096f84ae2e94ab791b291e92c1b20a7acb1` |
| Selected temporary extraction | `bandizip-create-p-0x14014a2cf.json` | `ev_382eee26123567a381d91ae7b20d8b7b055f381d8d8ed5f7d7fa5bfcee2c785f` |
| Launch-record context | `bandizip-create-p-0x14014a562.json` | `ev_7d794a0043c78c00872ce7e813e4c0d7b79b19ccd6e81b889928a41a499da120` |
| Launch wrapper/mode propagation | `bandizip-create-p-0x140161dbf.json` | `ev_57dbe198e255a01beb7d149dc8fc38fa3f75aafd1598afab7c361aac28a3466a` |
| Self-executable path | `bandizip-create-p-0x140022cc0.json` | `ev_1e4b0293930585b598203293d5a547fb6281e00d0e0da3457b40e73b1f7fd56c` |
| Child launch and tracked fields | `bandizip-browser-code-from-0x140161f3e.json` | `ev_19bb8f824c88232b31e22426d59a58980ca6c3df4ccbb1a04f4253b3dfdf4864` |
| Child exit/temp change | `bandizip-browser-code-0x140163120.json` | `ev_9f9eb83e282cbdde936df6c138748771e435544761a1b4414e3791d70f2d25f6` |
| Yes to original edit request | `bandizip-browser-cont-analysis-0x14014f8d0.json` | `ev_7ecccc40046c4d6f05c2e1c9d5c497ebc63a8a1f4a2543612dac212f0d1925b1` |
| Test/replacement/restore/refresh | `bandizip-browser-code-from-0x140154b1f.json` | `ev_23defda9d8520169930e8fdcdf50b333d77edf140f471e6aa2c306ac3dd6e915` |
| Actual native move | `bandizip-create-p-0x140122080.json` | `ev_507e63045906d576223a09178e5c9fd5cfd2e2aa1421d1253ff7b77d8658c40a` |
| Backup cleanup | `bandizip-create-p-0x140121f60.json` | `ev_71f6c58f4d9c34deb96aae42b18ed01c004ac23c7abfc950fc73ecf9d94ff8db` |

`bandizip-writeback-menu-metadata.json` retains the standard-menu observation for resource173, command0x81ef and its dispatch byte; it is static PE metadata, not a new REA record. The earlier [browser report](bandizip-browser-integration.md) retains the prompt/child owners, and [creation/edit report](bandizip-create-edit-repair.md) retains compressor/member-update behavior.

## Limitations

All paths are reconstructed static behavior, not a performed edit, native failure reproduction or crash test. Format-specific writeback acceptance, every external application's process lifetime, simultaneous nested edits and temporary-file cleanup after every failure were not exercised. The source/member context and native path moves do not certify original-object identity, transaction durability or complete rollback. No product logic, archives, credentials, system settings or installed files were modified.
