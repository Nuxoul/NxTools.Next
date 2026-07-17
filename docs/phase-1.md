# Phase 1: platform proof

Phase 1 answers the risky architecture questions before product features are migrated.

## Delivered in this checkpoint

- independent dual-licensed repository;
- Rust workspace and .NET PlatformHost project;
- protocol v1 with compatibility validation and round-trip tests;
- bounded frame readers and protocol wire tests in both Rust and C#;
- duplex Windows named-pipe handshake between Rust and C#;
- runtime directories, rotating log writer and single-instance guard;
- GPUI shell with client decorations, draggable custom title bar and design tokens;
- representative virtual list, image, scrolling and popover states;
- separate Rust indexer process placeholder;
- repeatable development build and launch scripts.
- packaged-desktop supervision and restart of the C# PlatformHost adapter.

## Manual verification

1. Run `scripts/build.ps1`.
2. Run `scripts/run-dev.ps1`.
3. Confirm the custom window is centered and can be dragged, minimized, maximized and closed.
4. Confirm Chinese labels are crisp at 100%, 125% and 150% scaling.
5. Toggle the preview and scroll the history proof independently.
6. Confirm the desktop log records a `platform_host` handshake and foreground-window event.
7. Launch a second desktop instance and confirm it exits immediately.

## Exit criteria before clipboard migration

- idle CPU remains below 1% after settling on the target machine;
- no timer-driven UI refresh loop is present;
- named-pipe reconnect survives either adapter process restarting;
- per-monitor DPI and Chinese IME input are manually verified on Windows 10 and 11;
- debug and release builds are reproducible from a clean checkout.

Measured results and remaining environment gates are recorded in
[verification.md](verification.md). The current development machine does not have `fxc.exe`, so
the Debug pipeline is verified but the formal Release/package exit criterion remains open until
the Windows SDK is installed.
