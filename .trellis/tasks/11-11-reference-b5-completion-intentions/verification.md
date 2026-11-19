# B5 verification

Acceptance: `not_accepted`.

## Execution boundary

Evidence root: `D:\.pi-desktop\data\scratch\0ffe6ffa-9e52-420b-af61-a29fdd0f37b4`. The task's `verification.json` indexes the full `b5-verification.json` there: exact selections, commands, LIST/RUN exits, counts, original logs, behavioral RED and prerequisite records. Overlapping selections are not added into a total.

Each Rust execution initialized jobs1/incremental0/codegen-units256 and scratch TEMP/TMP in that same PowerShell process after the global compiler-lane check. Every test uses `cargo test --locked --offline -p <package> <target> <filter>`; target `lib` maps to `--lib`, `bin:name` to `--bin name`, other targets to `--test name`. `(all)` omits the filter. LIST appends `-- --list`; RUN appends `-- --nocapture --test-threads=1` to the identical selection. Temporary runners are `b5-run-rust.ps1`, `b5-preservation.ps1`, `b5-quality.ps1` and `b5-quality-retest.ps1` under the evidence root.

## Current-source preservation

| Log stem | Package / target / filter | LIST/RUN/PASS/FAIL/SKIP/not_run | Exits |
| --- | --- | --- | --- |
| `b5-final-archive-draft` | `nxtools-desktop / bin:nxtools-desktop / archive::draft::tests::` | 12/12/12/0/0/0 | 0/0 |
| `b5-final-archive-job` | `nxtools-desktop / bin:nxtools-desktop / archive::application::jobs::tests::` | 129/129/122/7/0/0 | 0/101 |
| `b5-final-b4-ui` | `nxtools-desktop / bin:nxtools-desktop / b4_` | 30/30/30/0/0/0 | 0/0 |
| `b5-final-b5-ui` | `nxtools-desktop / bin:nxtools-desktop / b5_` | 22/22/22/0/0/0 | 0/0 |
| `b5-final-browser-host` | `nxtools-browser-host / lib / (all)` | 48/48/48/0/0/0 | 0/0 |
| `b5-final-cli` | `nxtools-cli / lib / (all)` | 15/15/15/0/0/0 | 0/0 |
| `b5-final-completion-original` | `nxtools-desktop / bin:nxtools-desktop / shutdown` | 12/12/12/0/0/0 | 0/0 |
| `b5-final-completion` | `nxtools-download / b5_completion_contract / (all)` | 9/9/9/0/0/0 | 0/0 |
| `b5-final-coordinator-original` | `nxtools-desktop / bin:nxtools-desktop / completion_actions::tests::` | 5/5/5/0/0/0 | 0/0 |
| `b5-final-core-lib` | `nxtools-core / lib / (all)` | 98/98/98/0/0/0 | 0/0 |
| `b5-final-core-settings` | `nxtools-core / b4_download_settings_contract / b4_` | 2/2/2/0/0/0 | 0/0 |
| `b5-final-coverage-http` | `nxtools-download / b4_coverage_contract / b4_` | 1/1/1/0/0/0 | 0/0 |
| `b5-final-detail` | `nxtools-desktop / bin:nxtools-desktop / live_progress_updates_the_cached_selected_segment_without_requerying_sqlite` | 1/1/1/0/0/0 | 0/0 |
| `b5-final-domain` | `nxtools-download / domain_contract / (all)` | 12/12/12/0/0/0 | 0/0 |
| `b5-final-download-ipc` | `nxtools-download-ipc / lib / (all)` | 9/9/9/0/0/0 | 0/0 |
| `b5-final-download-lib` | `nxtools-download / lib / (all)` | 128/128/128/0/0/0 | 0/0 |
| `b5-final-engine` | `nxtools-download / engine_contract / (all)` | 105/105/105/0/0/0 | 0/0 |
| `b5-final-handoff` | `nxtools-desktop / bin:nxtools-desktop / task_window_handoff_` | 2/2/2/0/0/0 | 0/0 |
| `b5-final-headers-http` | `nxtools-download / b4_request_policy_contract / b4_` | 3/3/3/0/0/0 | 0/0 |
| `b5-final-http-client` | `nxtools-download / http_client_contract / (all)` | 26/26/26/0/0/0 | 0/0 |
| `b5-final-ipc` | `nxtools-ipc / lib / (all)` | 62/62/62/0/0/0 | 0/0 |
| `b5-final-messages` | `nxtools-download / message_contract / (all)` | 16/16/16/0/0/0 | 0/0 |
| `b5-final-paused` | `nxtools-download / b4_paused_url_contract / (all)` | 11/11/11/0/0/0 | 0/0 |
| `b5-final-phase-size` | `nxtools-desktop / bin:nxtools-desktop / task_windows_use_compact_content_driven_phase_sizes` | 1/1/1/0/0/0 | 0/0 |
| `b5-final-protocol` | `nxtools-protocol / lib / (all)` | 116/116/116/0/0/0 | 0/0 |
| `b5-final-rate-limit` | `nxtools-download / rate_limit_contract / (all)` | 2/2/2/0/0/0 | 0/0 |
| `b5-final-ref06` | `nxtools-desktop / bin:nxtools-desktop / ref06_` | 11/11/11/0/0/0 | 0/0 |
| `b5-final-render-owner` | `nxtools-desktop / bin:nxtools-desktop / task_window_render_never_reads_the_parent_download_application` | 1/1/1/0/0/0 | 0/0 |
| `b5-final-runtime-speed` | `nxtools-download / runtime_speed_cap_contract / (all)` | 8/8/8/0/0/0 | 0/0 |
| `b5-final-server` | `nxtools-desktop / bin:nxtools-desktop / server_info_` | 7/7/7/0/0/0 | 0/0 |
| `b5-final-settings-failure` | `nxtools-desktop / bin:nxtools-desktop / download_settings_save_failure_uses_center_and_keeps_original_draft` | 1/1/1/0/0/0 | 0/0 |
| `b5-final-settings-rollback` | `nxtools-desktop / bin:nxtools-desktop / settings_persistence_failure_rolls_back_engine_values` | 1/1/1/0/0/0 | 0/0 |
| `b5-final-theme` | `nxtools-desktop / bin:nxtools-desktop / auxiliary_theme_tests::` | 10/10/10/0/0/0 | 0/0 |

Each stem identifies `<stem>-list.log`, `<stem>.log` and `<stem>-results.json` under the evidence root. The Archive129 group remains failed because seven original tests require absent native wire exports; exact names and attribution are in `limitations.md`. SKIP/not_run do not count as PASS.

## Behavioral RED and GREEN

| RED log stem | LIST/RUN/PASS/FAIL/SKIP/not_run | Exits | GREEN owner |
| --- | --- | --- | --- |
| `b5-ref07-persistence-red` | 1/1/0/1/0/0 | 0/101 | task persistence1, included in Desktop22 |
| `b5-queue-invalidation-red` | 1/1/0/1/0/0 | 0/101 | Download completion9 |
| `b5-worker-drain-red` | 1/1/0/1/0/0 | 0/101 | worker drain1, included in Download lib128 |
| `b5-round-native-probe-red` | 3/3/0/3/0/0 | 0/101 | Desktop22 |
| `b5-suspend-deadline-red` | 1/1/0/1/0/0 | 0/101 | Desktop22 |
| `b5-disconnect-red` | 1/1/0/1/0/0 | 0/101 | Desktop22 |

Original assertions, owner chains and GREEN logs are in `ref07-outcomes.md`, `backend-outcomes.md`, `lifecycle-outcomes.md`, `ui-outcomes.md` and `exit-outcomes.md`. C# production dispatch has its separate genuine RED1 → GREEN2 in `native-outcomes.md`.

The machine ledger separates compiler/API prerequisites, fixture corrections and zero selection from behavioral RED. Actual Desktop counts include LIST0/RUN0/not_run14 for the compile prerequisite, 14/14/PASS11/FAIL3 for focus selectors, 20/20/PASS19/FAIL1 for Escape KeyUp lifetime, and original shutdown12/PASS10/FAIL2 for pre-click feedback isolation. Their raw records remain in the evidence root. Real SQLite/HTTP/runtime cases without an observed pre-fix failure are first-pass supplementary coverage.

## Safe PlatformHost fixtures

`csharp/NxTools.PlatformHost.Tests/bin/Debug/net8.0-windows/NxTools.PlatformHost.Tests.exe --fixtures --list "B5 completion"`, then identical filter without `--list`: LIST2/RUN2/PASS2/FAIL0/SKIP0/not_run0, exits0/0. Bodies and TestEffects were reviewed. Typed fake covers Sleep/Hibernate,50/5/1314/87, correlation/elapsed/native code and held-delegate dispatch/drain. Debug builds have zero warnings/errors. No capability, process-token or power API was invoked. Exact commands and logs are in `native-outcomes.md`.

## Formatting, strict lint and check

| Check | Exit | Log |
| --- | --- | --- |
| `format` | 0 | `b5-final-format.log` |
| `clippy-nxtools-core` | 0 | `b5-final-clippy-nxtools-core.log` |
| `clippy-nxtools-download` | 0 | `b5-final-clippy-nxtools-download.log` |
| `clippy-nxtools-download-ipc` | 0 | `b5-final-clippy-nxtools-download-ipc.log` |
| `clippy-nxtools-ipc` | 0 | `b5-final-clippy-nxtools-ipc.log` |
| `clippy-nxtools-protocol` | 101 | `b5-final-clippy-nxtools-protocol.log` |
| `clippy-nxtools-desktop` | 101 | `b5-final-clippy-nxtools-desktop.log` |
| `clippy-nxtools-cli` | 0 | `b5-final-clippy-nxtools-cli.log` |
| `clippy-nxtools-browser-host` | 0 | `b5-final-clippy-nxtools-browser-host.log` |
| `check` | 0 | `b5-final-check.log` |

Commands: `rustfmt --check --edition 2024 --config skip_children=true <B5 Rust paths>` and per package `cargo clippy --locked --offline --keep-going -p <package> --all-targets -- -D warnings`. Exact Rust path arguments and final quality commands are in the full machine ledger. Protocol/Desktop failures are the three unchanged expressions in `limitations.md`; no suppression is used.

Combined check:

```text
cargo check --locked --offline --keep-going -p nxtools-core -p nxtools-download -p nxtools-download-ipc -p nxtools-ipc -p nxtools-protocol -p nxtools-desktop -p nxtools-cli -p nxtools-browser-host --all-targets
```

## Documentation and source gates

- `python tests/architecture/spec-contracts.py`: exit0,129 documents/14 owners; `b5-final-gate-spec-contracts.log`.
- `python tests/architecture/gpui-component-catalog.py`: exit0; `b5-final-gate-gpui-component-catalog.log`.
- `python tests/architecture/gpui-release-contracts.py`: exit0, Kit0.8.0/pre0.3.9/exactly two approved patches; `b5-final-gate-gpui-release-contracts.log`.
- `python tests/architecture/markdown-editor-contracts.py`: exit0; `b5-final-gate-markdown-editor-contracts.log`.
- `tests/architecture/download-ui-lifecycle.ps1`: passed; `b5-final-download-ui-lifecycle.log`.

These establish documentation/source/provenance consistency, not visual/native/installed acceptance.

## Resource and handoff evidence

`b5-final-resources.json` records no remaining workspace tests, B5 wrappers, archive workers (both executable names), test TCP or B5 completion pipes. Production fixtures separately assert actual Download futures, HTTP listeners/sockets, IPC callers/clients/host joins, clients0/Stopped and post-stop refusal. A TestPlatform quit request is not physical process exit.

`changed-paths.txt` is relative to the accepted dirty B1–B4 starting state and original B5 before-copies, not a HEAD-wide attribution. `b5-path-audit-results.json` verifies protected coordinator bytes and unchanged branch/HEAD. `b5-final-record-audit.json` verifies JSON/JSONL, exact manifest and scoped diff-check.

Independent coordinator, physical power/current-user Desktop Exit, native OS-close/IME/DPI/Shell, human visual and installed/Release/whole-workspace acceptance remain unperformed. Stop at B5.
