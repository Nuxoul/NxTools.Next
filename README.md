# NxTools Next

NxTools Next is a Windows productivity suite rebuilt around four focused capabilities:

- accelerated downloads;
- clipboard history and paste workflows;
- a lightweight floating assistant;
- instant file and application finding.

The product is a clean-room rewrite. Rust owns the GPUI desktop experience, domain logic,
storage, search, download engine and orchestration. C# is a deliberately narrow Windows
adapter for APIs that are significantly safer or more maintainable through .NET and COM.

## Repository status

Phase 1 is the architecture and platform proof. It establishes the process boundary,
versioned named-pipe protocol, custom GPUI shell, runtime paths, logs and build scripts.
Product features will be delivered as vertical slices after these foundations are verified.

## Quick start

Requirements: Windows 10/11, Rust 1.96 and .NET SDK 8.0.421 or a compatible patch.
Release builds also require the Windows 10/11 SDK shader compiler (`fxc.exe`); set
`GPUI_FXC_PATH` when it is installed outside the standard SDK directory.

```powershell
.\scripts\build.ps1
.\scripts\run-dev.ps1
```

See [docs/architecture.md](docs/architecture.md) for ownership rules and
[docs/phase-1.md](docs/phase-1.md) for the verification checklist, and
[docs/building.md](docs/building.md) for toolchain details.

## License

Licensed under either Apache License, Version 2.0 or MIT License at your option.
