# Building on Windows

## Debug

Debug builds require Rust 1.96 and .NET SDK 8.0.421. Run:

```powershell
.\scripts\build.ps1
```

## Release

GPUI 0.2.2 compiles its DirectX shaders during non-debug builds and therefore requires the
legacy HLSL compiler `fxc.exe`. Install a Windows 10 or Windows 11 SDK that includes the shader
tools. The build script searches:

1. `GPUI_FXC_PATH`;
2. the current `PATH`;
3. versioned `C:\Program Files (x86)\Windows Kits\10\bin\*\x64` directories.

For trusted packaging, first read [the persistent packaging handoff](../PACKAGING.md).
It includes the existing local PEM/ledger paths; plain `package.ps1` without signing arguments is
not a complete trusted-release command. On the configured workstation use:

```powershell
.\scripts\package-local.ps1 -CheckOnly
.\scripts\package-local.ps1
```

The first command discovers prerequisites without reserving a sequence or building; the second
automatically selects the next sequence and delegates to the original verified packaging pipeline.
The preflight fails before Cargo when required tools or publisher material are absent.

The Windows wrappers also own the repository's low-peak Rust policy. They force one Cargo build job,
disable incremental compilation for verification/package runs and apply the required 256 codegen
units even when the caller did not set environment variables. Release verification runs formatter,
Clippy and tests without optimizing every test target, then builds only the packaged binaries with
the Release profile. The wrapper also disables development/test debug information and Release LTO
for these verification processes, avoiding large GPUI test PDB/link peaks without changing product
source behavior.

Before a Release build starts Cargo, the wrapper reads Windows committed-memory counters and requires
6 GiB of free commit headroom. This prevents the known OS 1455/LNK1102/`0xc0000409` failure mode from
destabilizing unrelated applications. The threshold can be changed explicitly when diagnosing a
controlled machine:

```powershell
.\scripts\package.ps1 -MinimumCommitHeadroomGiB 8
```

Lowering the threshold accepts the system-stability risk; it is not the normal packaging path.

If headroom is initially below the threshold, `package.ps1` may reclaim memory only from running
Desktop, CLI or PlatformHost processes whose executable paths exactly match the selected package
directory. Those processes must be stopped before their files can be replaced anyway. The wrapper
then samples commit headroom again and still refuses to start Cargo if the threshold is unmet. It
never terminates an editor, an installed copy of NxTools or another process by name alone.

### Memory-cleaner reference boundary

[Mem Reduct](https://github.com/henrypp/memreduct) was inspected as a Windows memory-management
reference on 2026-08-08. Its repository is GPL-3.0, so its source is not copied into this
MIT/Apache-licensed package. The verified upstream cleanup path uses native system-information calls
to empty process working sets, trim the system file cache, flush modified pages and purge standby
page lists. Those operations can increase available physical RAM, but working-set residency is not
Windows committed-memory headroom. They do not release a live process's committed private
allocations and therefore do not solve this build's measured commit-limit failure mode. System-wide
working-set or page-list purges are intentionally excluded from the build wrappers because they can
increase page faults and disrupt unrelated applications while making the visible RAM number look
better. The safe remedies remain bounded build concurrency, exact-package process shutdown, commit
re-sampling, a sufficient paging-file limit and refusal before Cargo when headroom is still unsafe.

The checked-in `.zed/settings.json` keeps Rust `target` trees, C# `bin`/`obj`, diagnostics and packaged
artifacts out of Zed's file scanner. This prevents a Release build's high-volume file changes from
adding editor indexing pressure on top of Cargo's commit usage.

## Temporary .NET build trees

`build.ps1` owns its unique `vh-*` directory, Release fixture verification owns `vf-*`, and
`package.ps1` owns `package-build-*`. The creating wrapper removes only that exact directory in
its outermost `finally`, on success or failure. `PI_SCRATCH_DIR` selects the temporary parent;
otherwise Windows Temp is used. Package ownership includes setup failures before verification
and failures after package copying during installer preparation. No whole-Temp cleanup is performed.

`publish-release-hosts.ps1` borrows the caller's build/generated directories. It leaves generated
identity inputs and published hosts available until the caller's test builds or package copies
finish. Packaged binaries, manifests, installer/source-distribution outputs and packaging logs are
retained outside the temporary build tree. Publisher PEM/sequence ledger and service recovery
remain governed by [the original handoff](../PACKAGING.md).

Default C# fixture evidence is retained in `.diagnostics/verification/fixtures-<unique-id>/`, not
inside `vh-*`. Explicit lifecycle/volume evidence roots remain in effect, and the wrapper reports
the actual evidence paths and restores the caller's fixture environment on exit.

IsolatedStaging retains the four public generated C# inputs in `<output>.generated-inputs/`.
The current managed Host assembly is retained as `<output>.TEST-PlatformHost.dll` for the original
`ArchiveReleaseTests.StagedActualIdentities` metadata/identity checks. `<output>.build-inputs.json`
references the retained `generated_directory` and `host_assembly`, never a temporary `build_directory`.
The existing `<output>.TEST-public-trust.g.cs` remains available; no private PEM is copied. These
sidecars are test evidence outside the installable payload, not production-trusted installation inputs.

`python tests/architecture/build-tree-cleanup.tests.py` runs unchanged wrapper copies against
tool stubs and self-owned temporary roots. It covers both parent locations, success, setup and
downstream failures, native nonzero exits, live-tree assertions at ISCC/final distribution, retained
evidence/managed-input references, consumer source bindings and borrowed outputs.
It does not run Cargo, .NET builds, real signing, installers or service/process operations.
