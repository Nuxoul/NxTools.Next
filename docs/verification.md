# Phase 1 verification record

Date: 2026-07-18

Machine: Windows 10.0.19045, AMD Radeon RX 580 2048SP

## Automated

- `cargo fmt --all --check`: pass
- `cargo clippy --workspace --all-targets -- -D warnings`: pass
- `cargo test --workspace`: pass, including clipboard favorite/pin persistence, pinned-first
  ordering, soft-delete/restore, plain-text editing, category persistence/source filtering and
  category wheel/drop behavior, plus embedded Material Symbol availability
- Rust protocol/runtime tests: pass
- .NET build with warnings as errors: pass
- C# protocol wire and bounded-reader tests: pass
- Rust/C# named-pipe handshake: pass
- foreground context event received by Rust: pass
- packaged desktop launches PlatformHost with the Rust process as parent: pass
- graceful desktop close stops the supervised PlatformHost: pass
- forced/crashed parent termination is detected and the PlatformHost exits: pass
- second desktop instance rejected with non-zero exit: pass
- Release/package build with GPUI shaders: pass; package produced at `artifacts/win-x64`

## Performance baseline

Debug build, window open and idle after a five-second settling period, sampled for five seconds:

| Process | CPU | Working set |
| --- | ---: | ---: |
| `nxtools-desktop` | 0.130% | 54.4 MiB |
| `NxTools.PlatformHost` | 0.000% | 27.1 MiB |

This is a diagnostic baseline, not the `<1%` Release acceptance result.

## Open environment gates

- Windows 10 and 11 per-monitor DPI: requires manual testing on both operating systems.
- Chinese IME: the proof now contains a real GPUI input control; composition still requires
  manual verification with the target IMEs.
