# Release 45 — Root wrapper name units (R45-N01)

## Contract

Tree Root uses the archive display basename without its archive extension as a descriptor-only directory. Desktop `model_drag.rs::archive_drag_stem` checks its Windows component; `freeze_drag_roots` prefixes every descriptor before IPC while leaving original member ordinals and cache paths untouched. A component fits at most 255 UTF-16 code units; the complete descriptor-relative path must use fewer than 260 UTF-16 code units, leaving a terminating WCHAR in the SDK `FILEDESCRIPTORW.cFileName[MAX_PATH]`. The `FILEGROUPDESCRIPTORW` HGLOBAL stores a count followed by 592-byte descriptors with the UTF-16 name at offset 72. Transport separately limits the **entire encoded envelope** to 1 MiB; it does not impose a 255-byte UTF-8 component cap.

The native command parser now applies the same UTF-16 component limit as the Rust validator. Its existing `ArchiveVirtualPlan.Admit` → `ArchiveVirtualFileDataObject.ValidTree` → `ArchiveOutputSession.Validate` chain still checks Windows-invalid names and full descriptor length before writing the HGLOBAL. A noncolliding 86-CJK-character wrapper (86 WCHAR / 258 UTF-8 bytes) remains verbatim; a colliding presentation may still use the existing bounded suffix logic. The original entry operation, ordinal, source and member content are unchanged.

## Regression verification

- `model_drag::tree_tests::tree_root_long_unicode_wrapper_preserves_name_and_member_ordinal`: frozen Root tree for `界` repeated 86 plus `.zip`, asserts the unshortened wrapper, prefixed file descriptor and original ordinal 7.
- `archive_virtual_drag::tests::r45_n01_long_root_serialized_wire_fixture`: validates a complete Root command and serializes the **real Rust envelope** into `$PI_SCRATCH_DIR/archive-r45-n01-long-root-wire.json` when `PI_SCRATCH_DIR` is set. This generated fixture is an intermediate, not a checked-in wire imitation.
- Existing C# `ArchiveVirtualTreeTests.WireShapesConflictsAndBounds` now admits 85/86/255 CJK code units and rejects 256; `ArchiveVirtualTreeTests.SdkDescriptorsAndDirectoryRefusal` reads the Rust wire, passes it through `ArchiveVirtualDragWire.Parse` and `ArchiveVirtualPlan.Admit`, then calls actual `IDataObject.GetData(CFSTR_FILEDESCRIPTORW)` and checks all three names, NUL termination, flags/attributes and original ordinals. This test fails on the former UTF-8-byte rejection. No OLE loop, worker, user files or system configuration is involved.

The Rust protocol fixture, Desktop Root-freeze test and both exact C# wire/HGLOBAL descriptor fixtures pass. Rust serializes the real command into session scratch; C# consumes that file through original admission and native descriptor construction. The regression detects the former UTF-8 veto; no executed pre-fix failure is claimed for this name case. [Executed verification](release45-repair.md).

## Inspected reference and limits

`docs/reference-code/archive-virtual-drag.md:130-142,213-249` records original Root and native flow. Installed Windows SDK 10.0.26100.0 `um/ShlObj_core.h:1865-1879,1897-1900` defines the actual descriptor/group arrays; legacy `D:\Code\NxTools\Services\WindowsFileDragService.cs:29-61` uses real-file CF_HDROP and does not set this virtual descriptor policy. Current source ownership was verified at `rust/apps/nxtools-desktop/src/archive/model_drag.rs:26-52,408-439`, `rust/crates/nxtools-protocol/src/archive_virtual_drag.rs:11-39,232-334`, `csharp/NxTools.PlatformHost/Protocol/ArchiveVirtualDragContract.cs:51-99`, `csharp/NxTools.PlatformHost/Windows/Archive/ArchiveVirtualPlan.cs:18-26` and `ArchiveVirtualFileDataObject.cs:60-109`. No real receiver, held pointer, OLE or installed Release acceptance is established here.
