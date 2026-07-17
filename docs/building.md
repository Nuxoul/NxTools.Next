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
