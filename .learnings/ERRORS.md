# Errors

Command failures and integration errors.

---

## [ERR-20260718-003] clipboard-category-regression-test

**Logged**: 2026-07-18T11:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

A regression test assumed `ClipboardHistory::open_in_memory`, but this project only exposes `ClipboardHistory::open`.

### Error

```text
no associated function or constant named `open_in_memory` found for struct `ClipboardHistory`
```

### Context

- Command: `cargo test -p nxtools-clipboard invalid_category_kind_is_rejected_instead_of_becoming_all`
- The unverified helper assumption prevented the new corruption regression test from compiling.

### Suggested Fix

Use the same `tempfile::tempdir` plus file-backed SQLite setup as the crate's existing tests, and verify available test helpers before calling them.

### Metadata

- Reproducible: yes
- Related Files: rust/crates/nxtools-clipboard/src/lib.rs

### Resolution

- **Resolved**: 2026-07-18T11:00:00+08:00
- **Notes**: Replaced the nonexistent helper with the established temporary database fixture.

---

## [ERR-20260718-003] cargo-test-multiple-positional-filters

**Logged**: 2026-07-18T10:40:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tooling

### Summary

`cargo test` accepts only one positional test-name filter; passing two test names made that step fail while a later PowerShell command masked the aggregate exit code.

### Resolution

Use one shared module/name substring or run focused tests as separate commands, and do not infer a chained native-command result only from the final process exit code.

---

## [ERR-20260718-002] stale-platform-protocol-file-names

**Logged**: 2026-07-18T10:05:00+08:00
**Priority**: low
**Status**: resolved
**Area**: backend

### Summary

Repository inspection referenced guessed protocol file names that do not exist.

### Error

```text
Cannot find path '...Protocol\PlatformCommandProcessor.cs'
Cannot find path '...Protocol\PlatformCommand.cs'
```

### Context

- The actual command parsing and dispatch live in `PlatformCommandParser.cs` and `PlatformPipeClient.cs`.

### Suggested Fix

Run `rg --files` for the target directory before opening architecture-sensitive files.

### Metadata

- Reproducible: yes
- Related Files: csharp/NxTools.PlatformHost/Protocol

### Resolution

- **Resolved**: 2026-07-18T10:06:00+08:00
- **Notes**: Located the real parser and dispatcher from the repository file list.

---

## [ERR-20260718-001] product-design-user-context-preflight-path

**Logged**: 2026-07-18T09:35:00+08:00
**Priority**: low
**Status**: resolved
**Area**: config

### Summary

The Product Design user-context preflight script was resolved from the plugin root instead of the skill-local scripts directory.

### Error

```text
can't open file '...product-design\\0.1.50\\scripts\\user_context_preflight.py': No such file or directory
```

### Context

- The script referenced by `skills/user-context/SKILL.md` lives under `skills/user-context/scripts/`.

### Suggested Fix

Resolve relative script references against the directory containing the selected `SKILL.md`.

### Metadata

- Reproducible: yes
- Related Files: product-design/skills/user-context/SKILL.md

### Resolution

- **Resolved**: 2026-07-18T09:36:00+08:00
- **Notes**: Re-ran the preflight from the skill-local scripts directory successfully.

---

## [ERR-20260717-002] debug-package-missing-binaries

**Logged**: 2026-07-17T18:10:00+08:00
**Priority**: medium
**Status**: resolved
**Area**: infra

### Summary

The Debug package script copied top-level Rust executables after tests without explicitly building binary targets.

### Error

```text
Cannot find path 'target\debug\nxtools-indexer.exe' because it does not exist.
```

### Context

- Command: `scripts/package.ps1 -Configuration Debug`
- `cargo test` produced hashed test executables but not `target\debug\nxtools-indexer.exe`.

### Suggested Fix

Run `cargo build -p nxtools-desktop -p nxtools-indexer` for the selected profile before copying package files.

### Metadata

- Reproducible: yes
- Related Files: scripts/package.ps1

### Resolution

- **Resolved**: 2026-07-17T18:10:00+08:00
- **Notes**: The packaging script now explicitly builds both Rust binary targets before publishing and copying artifacts.

---

## [ERR-20260717-001] cargo-release-build

**Logged**: 2026-07-17T15:43:00+08:00
**Last Seen**: 2026-07-18T00:00:00+08:00
**Occurrences**: 2
**Priority**: high
**Status**: resolved
**Area**: infra

### Summary

GPUI 0.2.2 Debug builds succeed without `fxc.exe`, but non-debug Windows builds require it.

### Error

```text
Failed to find fxc.exe
```

### Context

- Command: `cargo build --release -p nxtools-desktop -p nxtools-indexer`
- Environment: Windows 10 with Rust 1.96; Windows SDK was rooted at `D:\Windows Kits\10` while the preflight only checked C:.
- GPUI's build script only compiles HLSL ahead of time when debug assertions are disabled.

### Suggested Fix

Install the Windows 10/11 SDK shader tools or set `GPUI_FXC_PATH`. Keep the build-script
preflight so missing toolchains fail before the expensive Release compilation begins.

### Metadata

- Reproducible: yes
- Related Files: scripts/build.ps1, docs/building.md

### Resolution

- **Resolved**: 2026-07-18T10:45:00+08:00
- **Notes**: Installed the Microsoft Windows SDK package and changed `Find-Fxc` to discover `KitsRoot10` through both Windows SDK registry views. The preflight now finds SDK tools regardless of drive.

---
