# Recipient distribution notices

The private recipient binary embeds these notices for `--licenses`; it does not extract them to disk or invoke another process.

- `project-MIT.txt`, `project-APACHE.txt`: workspace license notices, unchanged.
- `Apache-2.0.txt`: full Apache-2.0 license from locked flate2 1.1.9.
- `flate2-1.1.9-MIT.txt`: Alex Crichton 2014–2026 MIT notice.
- `crc32fast-1.5.0-MIT.txt`: Sam Rijs, Alex Crichton and contributors 2018 MIT notice.
- `cfg-if-1.0.4-MIT.txt`: Alex Crichton 2014 MIT notice.
- `zlib-rs-0.6.6.txt`: Trifecta Tech Foundation 2024 zlib notice.
- `tempfile-3.27.0-MIT.txt`: Steven Allen 2015 MIT notice; safe native no-replace publication.
- `fastrand-2.4.1-MIT.txt`, `once_cell-1.21.4-MIT.txt`, `getrandom-0.4.3-MIT.txt`: exact locked tempfile Windows dependency notices.
- `windows-sys-0.61.2-MIT.txt`, `windows-link-0.2.1-MIT.txt`: Microsoft Corporation MIT notices for tempfile's locked Windows bindings.
- `rust-1.96.0-std.txt`: complete text content of the installed Rust 1.96.0 x86_64-pc-windows-msvc distribution's `share/doc/rust/COPYRIGHT-library.html`, with HTML entities decoded and blank formatting lines removed. It includes the standard-library distribution's in-tree and out-of-tree notices.

Dependency notices come from their exact locked crates.io source distributions; no dependency source, tests or resources are adopted into the recipient. A different Rust toolchain requires its matching standard-library notices before rebuilding a distributable stub.

The locked Windows tempfile graph is tempfile 3.27.0 → fastrand 2.4.1, once_cell 1.21.4, getrandom 0.4.3 → cfg-if 1.0.4, and windows-sys 0.61.2 → windows-link 0.2.1. Unix rustix and optional non-Windows getrandom dependencies are not part of this Windows target. These MIT notices are retained even where release dead-code elimination removes unused dependency code.
