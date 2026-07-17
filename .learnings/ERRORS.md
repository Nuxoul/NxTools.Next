# Errors

Command failures and integration errors.

---

## [ERR-20260717-001] cargo-release-build

**Logged**: 2026-07-17T15:43:00+08:00
**Priority**: high
**Status**: in_progress
**Area**: infra

### Summary

GPUI 0.2.2 Debug builds succeed without `fxc.exe`, but non-debug Windows builds require it.

### Error

```text
Failed to find fxc.exe
```

### Context

- Command: `cargo build --release -p nxtools-desktop -p nxtools-indexer`
- Environment: Windows 10 with Rust 1.96; no Windows SDK shader compiler installed.
- GPUI's build script only compiles HLSL ahead of time when debug assertions are disabled.

### Suggested Fix

Install the Windows 10/11 SDK shader tools or set `GPUI_FXC_PATH`. Keep the build-script
preflight so missing toolchains fail before the expensive Release compilation begins.

### Metadata

- Reproducible: yes
- Related Files: scripts/build.ps1, docs/building.md

---
