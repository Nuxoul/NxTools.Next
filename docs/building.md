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

Then run:

```powershell
.\scripts\package.ps1
```

The preflight check fails immediately with a clear message when the compiler is absent, before
Cargo spends time optimizing the GPUI dependency graph.

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
