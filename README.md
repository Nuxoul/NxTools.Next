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

The first product vertical slice is a pure-text clipboard workflow. NxTools stays in the
background, captures text through the Windows adapter, and opens a focused GPUI clipboard
panel only when `Alt+C` is pressed. The former always-visible demo shell has been removed.

Images, files, categories, favorites, downloads, Finder/Listary behavior and the floating
assistant are intentionally not part of this slice.

## Quick start

Requirements: Windows 10/11, Rust 1.96 and .NET SDK 8.0.421 or a compatible patch.
Release builds also require the Windows 10/11 SDK shader compiler (`fxc.exe`); set
`GPUI_FXC_PATH` when it is installed outside the standard SDK directory.

```powershell
.\scripts\build.ps1
.\scripts\run-dev.ps1
```

After the development launcher starts, copy text in another application and press `Alt+C`.
Search the real persisted history or click an item to paste it back into the application that
was active when the panel opened.

See [docs/architecture.md](docs/architecture.md) for ownership rules and
[docs/phase-1.md](docs/phase-1.md) for the verification checklist, and
[docs/building.md](docs/building.md) for toolchain details.

## License

Licensed under either Apache License, Version 2.0 or MIT License at your option.
