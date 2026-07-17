# Phase 1: platform proof

Phase 1 answers the risky architecture questions before product features are migrated.

## Delivered in this checkpoint

- independent dual-licensed repository;
- Rust workspace and .NET PlatformHost project;
- protocol v1 with compatibility validation and round-trip tests;
- bounded frame readers and protocol wire tests in both Rust and C#;
- duplex Windows named-pipe handshake between Rust and C#;
- runtime directories, rotating log writer and single-instance guard;
- background GPUI application controller with no startup main window;
- `Alt+C` Windows global hotkey and cursor/target-window context transfer;
- independent borderless GPUI clipboard popup with search and virtual history list;
- Rust-owned SQLite text history, adjacent deduplication and restart persistence;
- correlated click-to-paste command restoring the original focused child window through C#;
- separate Rust indexer process placeholder;
- repeatable development build and launch scripts.
- packaged-desktop supervision and restart of the C# PlatformHost adapter.

## Manual verification

1. Run `scripts/build.ps1`.
2. Run `scripts/run-dev.ps1`.
3. Confirm no main window appears on startup.
4. Copy text in another application, press `Alt+C`, and confirm the popup appears near the mouse.
5. Confirm search filters persisted real history and no demonstration rows appear.
6. Click a row and confirm it pastes into the control that had focus before the popup opened.
7. Confirm Escape and clicking outside close the popup while background capture continues.
8. Confirm Chinese input and labels at 100%, 125% and 150% scaling.
9. Launch a second desktop instance and confirm it exits immediately.

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
