# NxTools Next

NxTools Next is a Windows productivity suite rebuilt around four focused capabilities:

- accelerated downloads;
- clipboard history and paste workflows;
- a lightweight floating assistant;
- instant file and application finding.

The product is a clean-room rewrite. Rust owns the GPUI desktop experience, domain logic,
storage, search, download engine and orchestration. C# is a deliberately narrow Windows
adapter for APIs that are significantly safer or more maintainable through .NET and COM.

## Engineering and UI standards

Start at [.trellis/spec/index.md](.trellis/spec/index.md) for ownership, coding contracts,
package checks and the approved UI target/current matrix. Standards are source-backed; gaps and
unverified native/release acceptance remain explicit. Validate document navigation and contract
structure with `python tests/architecture/spec-contracts.py` (Python 3.11+); product verification
uses the owner-specific checks and workstation rules there.

The repository-status section below is a historical phase-1 summary, not the current completion
ledger. See [the dated remediation report](docs/audits/2026-10-08-sol-review-remediation.md) and
active Trellis task artifacts for current implementation evidence and pending acceptance.

## Repository status

The active product slice is the background clipboard workflow. NxTools captures text, images
and file drops through the Windows adapter, persists typed history in Rust, and opens a focused
GPUI panel only when `Alt+C` is pressed. The panel supports search, type filters, native/plain
paste actions, favorites, pinning, text editing, soft deletion, immediate undo and persisted
built-in/custom source categories with drag ordering.

Clipboard controls use self-hosted Google Material Symbols SVG assets embedded in the desktop
executable, so icons remain available offline and do not depend on a font installed on Windows.

Downloads, Finder/Listary behavior and the floating assistant remain later vertical slices.

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
[docs/building.md](docs/building.md) for toolchain details. Previously inspected behavior and source
entry points are cataloged in [docs/reference-code](docs/reference-code/README.md).

## License

Licensed under either Apache License, Version 2.0 or MIT License at your option.
Third-party asset attribution is recorded in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
