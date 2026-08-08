# Errors

Command failures and integration errors.

---

## [ERR-20260808-MISSINGDESKTOPTESTPATH] assumed_desktop_test_directory

**Logged**: 2026-08-08T19:10:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

A read-only search assumed that `rust/apps/nxtools-desktop/tests` existed instead of discovering
the available test paths first.

### Error

```text
rg: rust/apps/nxtools-desktop/tests: 系统找不到指定的文件
```

### Context

- The repository keeps these desktop tests in `src/main.rs` and architecture gates under
  `tests/architecture`.
- No files were changed by the failed search.

### Suggested Fix

Use `rg --files` or `Test-Path -LiteralPath` before passing an assumed test directory to `rg`.

### Metadata

- Reproducible: yes
- Related Files: `rust/apps/nxtools-desktop/src/main.rs`, `tests/architecture`

### Resolution

- **Resolved**: 2026-08-08T19:10:00+08:00
- **Notes**: Subsequent discovery checked each candidate path before searching it.

---

## [ERR-20260808-DESKTOPTESTTIMEOUT] desktop_test_first_compile_timeout

**Logged**: 2026-08-08T19:18:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

The first desktop red-test invocation used a 120-second outer timeout, which expired during the
initial native dependency build before Rust emitted the intended compile failure.

### Error

```text
command timed out after 124041 milliseconds
```

### Context

- The build already used one Cargo job, disabled incremental compilation and 256 codegen units.
- No Cargo, rustc or linker process remained afterward, and 6.30 GiB physical memory was free.

### Suggested Fix

Allow up to five minutes for the first desktop test build while retaining the low-memory build
environment.

### Metadata

- Reproducible: yes
- Related Files: `rust/apps/nxtools-desktop/src/main.rs`

### Resolution

- **Resolved**: 2026-08-08T19:18:00+08:00
- **Notes**: The repeated command completed in 12 seconds from the warmed build tree and produced
  the intended missing-type red failure.

---

## [ERR-20260808-CLIPPYINSPECTERR] browser_host_clippy_manual_inspect

**Logged**: 2026-08-08T17:42:45+08:00
**Priority**: low
**Status**: resolved
**Area**: backend

### Summary

BrowserHost Clippy rejected `map_err` used only to log an unchanged process-spawn error.

### Error

```text
error: using `map_err` over `inspect_err`
```

### Context

- The desktop launch boundary logs the Windows native error code before adding `anyhow` context.
- The closure returned the original error unchanged, triggering `clippy::manual_inspect` under
  `-D warnings`.

### Suggested Fix

Use `Result::inspect_err` for logging-only side effects, then attach context without transforming
the original error.

### Metadata

- Reproducible: yes
- Related Files: `rust/apps/nxtools-browser-host/src/lib.rs`

### Resolution

- **Resolved**: 2026-08-08T17:42:45+08:00
- **Notes**: Replaced the logging-only `map_err` with `inspect_err` and reran Clippy.

---

## [ERR-20260808-HEADLESSSCREENSHOTRACE] screenshot_visible_after_process_return

**Logged**: 2026-08-08T17:29:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

The headless Chromium wrapper returned before PowerShell observed the screenshot file, although
Chromium wrote it moments later.

### Error

```text
Cannot find path '...\\target\\popup-width-check\\popup.png' because it does not exist.
47668 bytes written to file ...\\popup.png
```

### Context

- The render command and immediate `Get-Item` were in the same PowerShell invocation.
- The browser's output confirmed the screenshot write after the filesystem check had already run.

### Suggested Fix

Run screenshot generation and artifact inspection as separate bounded commands, or poll briefly for
the exact expected file before reading it.

### Metadata

- Reproducible: unknown
- Related Files: `target/popup-width-check/popup.png`

### Resolution

- **Resolved**: 2026-08-08T17:29:00+08:00
- **Notes**: Verified the exact file in a follow-up command and inspected the 456×600 render.

---

## [ERR-20260808-BGPACKAGE] start_process_package_timeout

**Logged**: 2026-08-08T17:15:42+08:00
**Priority**: medium
**Status**: resolved
**Area**: infra

### Summary

Launching the Release package through `Start-Process` did not return within the command wrapper's
10-second limit, and no durable package process or log remained.

### Error

```text
command timed out after 10016 milliseconds
```

### Context

- The attempted command started a hidden child PowerShell and redirected its output to temporary logs.
- The command runner tracks the spawned process tree, so `Start-Process` was not a reliable detach
  boundary in this environment.
- Read-only checks confirmed there was no package process before retrying.

### Suggested Fix

Run `package.ps1` directly through a yielded tool cell, then resume that same cell with the wait
mechanism. This preserves one process tree, streams completion, and avoids duplicate builds.

### Metadata

- Reproducible: unknown
- Related Files: `scripts/package.ps1`

### Resolution

- **Resolved**: 2026-08-08T17:15:42+08:00
- **Notes**: Switched the pending package run to a direct yielded command with the required low-memory environment.

---

## [ERR-20260808-DOWNLOADFIXTURETIMING] download_engine_fixture_parallel_timing

**Logged**: 2026-08-08T00:00:00+08:00
**Priority**: medium
**Status**: resolved
**Area**: tests

### Summary

Default-parallel download-engine integration runs failed at different timing-sensitive assertions:
CDN attempt count, Range fallback connection setup, and pausing before a small transfer completed.

### Resolution

- **Resolved**: 2026-08-08T00:00:00+08:00
- **Notes**: Exact reruns separated fixture timing from one real product gap: retryable single-stream
  failures had no bounded retry. A deterministic disconnect regression now covers that gap and
  passes 20/20; the complete download suite passes with `--test-threads=1` under the required
  low-memory policy. Default-parallel fixture timing remains recorded rather than misreported as a
  clean pass.

---

## [ERR-20260808-AMBIGUOUSASSERTPATCH] ambiguous_repeated_assert_patch

**Logged**: 2026-08-08T00:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tooling

### Summary

A patch intended to add context to one repeated `assert_eq!(completed.task.status(), ...)` matched
an earlier test with the same line.

### Resolution

- **Resolved**: 2026-08-08T00:00:00+08:00
- **Notes**: Diff inspection caught it before testing or commit; the earlier assertion was restored
  and the target patch was anchored to the exact test function. Future patches to repeated test
  assertions must include the function context.

---

## [ERR-20260808-BROWSERHOSTTARGET] cargo_binary_target_product_filename

**Logged**: 2026-08-08T00:00:00+08:00
**Priority**: medium
**Status**: resolved
**Area**: build

### Summary

The browser native host used the final Windows product filename as its Cargo binary target name;
Cargo rejects `.` in crate target identifiers.

### Error

```text
error: invalid character '.' in crate name: `NxTools.BrowserHost`
```

### Suggested Fix

Keep the Cargo target as `nxtools-browser-host` and rename it to `NxTools.BrowserHost.exe` only at
the package boundary.

### Resolution

- **Resolved**: 2026-08-08T00:00:00+08:00
- **Notes**: Added an architecture regression gate, corrected the target/packaging paths, and
  verified all three native-host tests pass under the low-memory Cargo policy.

---

## [ERR-20260808-BROWSERHOSTCLIPPY] browser_host_test_unwrap

**Logged**: 2026-08-08T00:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

The first BrowserHost Clippy pass found a bare `unwrap()` in the Native Messaging round-trip test,
which violates the workspace `unwrap_used` lint.

### Resolution

- **Resolved**: 2026-08-08T00:00:00+08:00
- **Notes**: Replaced it with a contextual `expect()` and reran Clippy for BrowserHost and the
  download core with `-D warnings`; the retry passed.

---

## [ERR-20260808-RUSTFMTEDITION] rustfmt_wrong_edition

**Logged**: 2026-08-08T00:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tooling

### Summary

Direct `rustfmt` verification was first invoked with edition 2021 even though this workspace uses
Rust 2024 syntax.

### Error

```text
error: let chains are only allowed in Rust 2024 or later
```

### Suggested Fix

Read the workspace package edition before invoking standalone `rustfmt`; pass `--edition 2024` for
this repository.

### Resolution

- **Resolved**: 2026-08-08T00:00:00+08:00
- **Notes**: Re-ran the same formatter with edition 2024 successfully. No build or package command
  was involved.

---

## [ERR-20260808-PSVARIABLECOLON] powershell_variable_followed_by_colon

**Logged**: 2026-08-08T00:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tooling

### Summary

A one-off PowerShell validation command embedded `$script:` in an interpolated string, which the
parser treated as an invalid scoped variable reference.

### Suggested Fix

Delimit PowerShell variables followed immediately by punctuation, for example `${script}:`.

### Resolution

- **Resolved**: 2026-08-08T00:00:00+08:00
- **Notes**: Corrected the validation command; the static diff and PowerShell parser checks then
  passed.

---
## [ERR-20260808-EMPTYPROCESSMATCH] powershell_empty_process_id

**Logged**: 2026-08-08T14:03:00+08:00
**Priority**: low
**Status**: resolved
**Area**: infra

### Summary

An empty process match was wrapped as an array containing `$null`, which caused `Get-Process` to
interpret the requested ID as PID 0.

### Error

```text
Cannot stop process "Idle (0)" because access is denied.
```

### Context

- The command attempted to stop only Cargo/rustc processes for one exact regression-test command.
- The matching processes had already exited, so the property projection produced a null element.
- No system or user process was stopped.

### Suggested Fix

Filter projected IDs with `Where-Object { $_ -gt 0 }` and test the resulting non-null array before
calling `Get-Process` or `Stop-Process`.

### Metadata

- Reproducible: yes
- Related Files: `.learnings/ERRORS.md`

### Resolution

- **Resolved**: 2026-08-08T14:03:00+08:00
- **Notes**: Verified that no Cargo or rustc process remained; subsequent process checks are read-only.

---

## [ERR-20260808-PSLASTEXITCODE] powershell_script_last_exit_code_null

**Logged**: 2026-08-08T00:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

A combined verification command checked `$LASTEXITCODE` after invoking a PowerShell script, causing
the command to exit successfully before later native checks ran because the value was null.

### Error

```text
Only "Package memory policy gate passed." was printed; later checks were skipped.
```

### Context

- PowerShell scripts communicate failure through terminating errors/`$?`, not reliably through the
  native-process `$LASTEXITCODE` variable.
- The skipped checks were rerun immediately as separate statements with the correct semantics.

### Suggested Fix

Let terminating PowerShell errors propagate or test `$?`; reserve `$LASTEXITCODE` for native
executables such as Git and Cargo.

### Metadata

- Reproducible: yes
- Related Files: `tests/architecture/package-memory-policy.ps1`

### Resolution

- **Resolved**: 2026-08-08T00:00:00+08:00
- **Notes**: Final verification was rerun without reading `$LASTEXITCODE` after the `.ps1` gate.

---

## [ERR-20260808-PACKAGETIMEOUT] package_wrapper_outer_timeout

**Logged**: 2026-08-08T00:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: infra

### Summary

The first controlled Release package attempt was terminated by the command runner's short outer
timeout before Cargo could return output.

### Error

```text
command timed out after 14035 milliseconds (exit code 124)
```

### Context

- The package command used the repository's low-memory wrapper with 13.56 GiB commit headroom.
- No `cargo`, `rustc`, `link`, or `lld-link` process survived and packaged artifacts were unchanged.
- This is an orchestration timeout, not a compiler or source failure.

### Suggested Fix

Run long build/package commands with a long command timeout and use the yielded cell to poll output.

### Metadata

- Reproducible: yes
- Related Files: `scripts/package.ps1`
- See Also: `ERR-20260728-PACKAGETIMEOUT`

### Resolution

- **Resolved**: 2026-08-08T00:00:00+08:00
- **Notes**: The retry uses a long outer timeout while preserving frequent progress polling.

---

## [ERR-20260808-PROCESSPROBE] optional_process_probe_exit_code

**Logged**: 2026-08-08T00:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: infra

### Summary

A read-only memory/process probe returned exit code 1 when one or more optional named processes did
not exist, although the committed-memory counters were valid.

### Error

```text
Exit code: 1; commit headroom output remained available.
```

### Context

- The probe passed several optional names to `Get-Process` during an active Release build.
- It did not affect the package process or modify system state.

### Suggested Fix

Enumerate optional process names individually into a variable before formatting, or use a filtered
all-process snapshot so an absent name is not treated as a command failure.

### Metadata

- Reproducible: yes
- Related Files: `scripts/package.ps1`

### Resolution

- **Resolved**: 2026-08-08T00:00:00+08:00
- **Notes**: Subsequent probes enumerate each optional name separately before formatting.

---
## [ERR-20260730-PSRANGES] powershell_nested_range_array_type_mismatch

**Logged**: 2026-07-30T16:48:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

A PowerShell source-inspection helper passed nested arrays with an unstable runtime type to
`[Math]::Min`, so the display command reported `Argument types do not match`.

### Error

```text
Argument types do not match
```

### Context

- The command attempted to print several line ranges from multiple Rust files.
- It was read-only and did not modify product code.

### Suggested Fix

Read each source range independently or cast both range endpoints and line counts to integers
before calling `[Math]::Min`.

### Metadata

- Reproducible: yes
- Related Files: `rust/crates/nxtools-download/src/http.rs`,
  `rust/crates/nxtools-download/src/engine.rs`

### Resolution

- **Resolved**: 2026-07-30T16:48:00+08:00
- **Notes**: Switched subsequent inspection commands to explicit source ranges.

---

## [ERR-20260730-CARGOFILTERS] cargo_test_accepts_only_one_name_filter

**Logged**: 2026-07-30T16:31:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

`cargo test` accepts one positional test-name filter, not two independent filter arguments.

### Error

```text
error: unexpected argument 'all_material_symbols_are_embedded' found
```

### Context

- Attempted to run the schedule-layout and embedded-icon tests in one Cargo invocation by passing
  both names.
- Cargo rejected the command before compiling or running product tests.

### Suggested Fix

Run the two filtered tests as separate Cargo commands, or use a single common substring/full suite.

### Metadata

- Reproducible: yes
- Related Files: `rust/apps/nxtools-desktop/src/downloads.rs`,
  `rust/apps/nxtools-desktop/src/assets.rs`

### Resolution

- **Resolved**: 2026-07-30T16:31:00+08:00
- **Notes**: Replaced the invalid invocation with separate filtered test commands.

---

## [ERR-20260730-PLATFORMPATH] assumed_desktop_platform_module_path

**Logged**: 2026-07-30T11:20:00+08:00
**Priority**: low
**Status**: resolved
**Area**: backend

### Summary

Reference inspection assumed a nonexistent desktop-local `platform.rs` instead of locating the
shared IPC bridge by symbol.

### Error

```text
Cannot find path rust/apps/nxtools-desktop/src/platform.rs
```

### Context

- The bridge is defined in `rust/crates/nxtools-ipc/src/lib.rs`.
- The failed read did not modify files or interrupt implementation.

### Suggested Fix

Use `rg "struct PlatformBridge" rust` before opening an assumed module path.

### Metadata

- Reproducible: yes
- Related Files: `rust/crates/nxtools-ipc/src/lib.rs`, `csharp/NxTools.PlatformHost/Protocol`
- Pattern-Key: tooling.discover_path_before_read
- Recurrence-Count: 2
- First-Seen: 2026-07-30
- Last-Seen: 2026-08-08

### Resolution

- **Resolved**: 2026-07-30T11:21:00+08:00
- **Notes**: Located and reviewed the shared bridge by symbol. The pattern recurred during Phase 4
  when two guessed C# protocol filenames interrupted a parallel read; the follow-up enumerates the
  protocol directory before opening exact files.

---

## [ERR-20260730-RANGEFIXTURE] transient_range_fixture_probe_race

**Logged**: 2026-07-30T10:40:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

The combined engine suite let the probe consume the transient 503 intended for a range-transfer
retry.

### Error

```text
transient transfer failed after 0 attempts: HTTP probe returned status 503
```

### Context

- All 21 unit tests and the other 37 engine contracts passed.
- The exact range-retry contract passed immediately when rerun alone.
- The failure occurred in fixture request ordering, before the behavior asserted by the test.

### Suggested Fix

Make the fixture distinguish probe requests from transfer-range requests before injecting the 503.

### Metadata

- Reproducible: no
- Related Files: `rust/crates/nxtools-download/tests/engine_contract.rs`
- See Also: `ERR-20260728-HTTPFIXTURE`

### Resolution

- **Resolved**: 2026-07-30T10:41:00+08:00
- **Notes**: Immediate exact rerun passed; no production change was made.

---

## [ERR-20260730-IPCSOCKET] ipc_http_fixture_nonblocking_read_race

**Logged**: 2026-07-30T10:25:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

The IPC integration fixture treated a transient nonblocking `WouldBlock` read as a terminal server
failure during the combined package run.

### Error

```text
read HTTP request: OS 10035 (WouldBlock)
HTTP task failed: Network
```

### Context

- Two preceding IPC tests passed.
- The exact HTTP transport test passed immediately when rerun alone.
- The failure occurred in the local test server before exercising task-detail behavior.

### Suggested Fix

Make the fixture retry nonblocking reads until its bounded deadline; do not change production
transport behavior based on a single fixture race.

### Metadata

- Reproducible: no
- Related Files: `rust/crates/nxtools-download-ipc/src/lib.rs`

### Resolution

- **Resolved**: 2026-07-30T10:26:00+08:00
- **Notes**: Immediate exact rerun passed; no production change was made.

---

## [ERR-20260730-V1TIMELINE] timeline_migration_assumed_full_v1_task_columns

**Logged**: 2026-07-30T10:00:00+08:00
**Priority**: high
**Status**: resolved
**Area**: backend

### Summary

The schema-v4 timeline seed assumed every v1 boundary contained status and update-time columns.

### Error

```text
no such column: updated_at_unix_ms
```

### Context

- The complete download package suite exercised the supported minimal schema-v1 migration boundary.
- Queue migration had intentionally supported a table containing only task ID and creation time.
- The new timeline migration queried newer columns and blocked upgrade before the application opened.

### Suggested Fix

Seed one safe `created` entry from the creation timestamp shared by every supported schema boundary.

### Metadata

- Reproducible: yes
- Related Files: `rust/crates/nxtools-download/src/storage.rs`,
  `rust/crates/nxtools-download/tests/storage_contract.rs`

### Resolution

- **Resolved**: 2026-07-30T10:03:00+08:00
- **Notes**: The v4 seed now uses only stable v1 columns; schema-three and minimal-v1 upgrade
  regressions cover both paths.

---

## [ERR-20260729-DETAILRESULT] inferred_detail_result_error_type

**Logged**: 2026-07-29T14:03:00+08:00
**Priority**: low
**Status**: resolved
**Area**: backend

### Summary

The detail-query closure's error type became ambiguous after adding a terminal failure branch.

### Error

```text
E0282: type annotations needed for Result<DownloadTaskDetail, _>
```

### Context

- The closure combines several `?` conversions before its error is inspected with `fault()`.

### Suggested Fix

Annotate the local result as `Result<DownloadTaskDetail, DownloadEngineError>`.

### Metadata

- Reproducible: yes
- Related Files: `rust/crates/nxtools-download/src/engine.rs`

### Resolution

- **Resolved**: 2026-07-29T14:03:30+08:00
- **Notes**: Added the explicit result type.

---

## [ERR-20260729-CARGOFILTER] cargo_test_multiple_positional_filters

**Logged**: 2026-07-29T14:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

`cargo test` accepts one positional test-name filter, not two separate filters.

### Error

```text
error: unexpected argument 'download_detail_masks_credential_headers_but_keeps_ordinary_values' found
```

### Context

- Attempted to run two named desktop unit tests in one Cargo invocation.
- The failure happened during command parsing; no test or product code ran.

### Suggested Fix

Run each exact filter separately, or run the complete package test suite.

### Metadata

- Reproducible: yes
- Related Files: `rust/apps/nxtools-desktop/src/main_window.rs`

### Resolution

- **Resolved**: 2026-07-29T14:01:00+08:00
- **Notes**: Switched verification to one filter per invocation followed by the full package suite.

---

## [ERR-20260728-FILTERTESTTRAIT] gpui_test_missing_interactive_trait

**Logged**: 2026-07-28T16:27:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

The new GPUI interaction fixture used `.id(...)` without importing the trait that exposes it in the
nested test module.

### Error

`no method named 'id' found for struct 'gpui::Div'`

### Context

- Production already imported GPUI's prelude at module scope, but nested Rust modules do not inherit
  that trait import.

### Suggested Fix

Import `gpui::InteractiveElement`, `gpui::StatefulInteractiveElement` and `gpui::Styled`
explicitly in GPUI test modules. Add `debug_selector(...)` explicitly when a visual test needs to
look up rendered bounds; an element ID alone does not populate `VisualTestContext::debug_bounds`.
Import `gpui::AppContext` when the fixture creates entities through `App::new`, and initialize
`gpui_component` before rendering components that read its global theme.

### Metadata

- Reproducible: yes
- Related Files: `rust/apps/nxtools-desktop/src/main_window.rs`

### Resolution

- **Resolved**: 2026-07-28T16:27:00+08:00
- **Notes**: Added the explicit test-module imports, including the separate stateful interaction
  trait required by `.on_click(...)`, removed the unused `div` import, and added the explicit
  visual-test bounds selector. The final real-`MainWindow` fixture imports `AppContext` and runs
  `gpui_component::init` before constructing the window.

---

## [ERR-20260728-PACKAGETIMEOUT] release_package_timeout

**Logged**: 2026-07-28T16:04:00+08:00
**Priority**: low
**Status**: resolved
**Area**: infra

### Summary

The full Release package wrapper exceeded the foreground tool's 120-second wait limit.

### Error

```text
command timed out after 124032 milliseconds
```

### Context

- The package wrapper includes full verification, Rust Release linking and .NET publishing.
- Its Cargo/rustc child processes remained active after the foreground wait ended.
- Existing package timestamps had not yet changed when first inspected.

### Suggested Fix

Monitor the surviving build process to completion, then rerun only the package copy stage with
verification skipped if the wrapper itself did not reach artifact publication.

### Resolution

The foreground wait expired while Cargo was still linking. The surviving build was allowed to
finish, then the package wrapper was rerun with `-SkipVerification` because the full test and
Clippy checks had already passed. The Release artifacts were published successfully.

### Metadata

- Reproducible: environment-dependent
- Related Files: `scripts/package.ps1`, `AGENTS.md`

---

## [ERR-20260728-CARGOFILTERS] cargo_multiple_test_filters

**Logged**: 2026-07-28T15:12:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

`cargo test` was given four positional test-name filters even though it accepts only one.

### Error

```text
error: unexpected argument 'task_can_resume_after_network_disconnect_exhausts_the_current_attempt'
```

### Context

- Clippy and formatting still ran, but the intended recovery tests did not run in that command.

### Suggested Fix

Loop over exact test names and invoke one `cargo test ... <filter>` command per test.

### Metadata

- Reproducible: yes
- Related Files: `rust/crates/nxtools-download/tests/engine_contract.rs`
- Recurrence-Count: 3
- Last-Seen: 2026-07-28

### Resolution

- **Resolved**: 2026-07-28T15:12:00+08:00
- **Notes**: Re-ran the recovery tests sequentially; later used one shared module-name filter for
  the desktop filter tests. Cargo accepts only one positional test-name filter per invocation.
  The same mistake recurred while combining the download-layout source and production GPUI tests;
  subsequent runs use one shared prefix or invoke the two exact filters separately.

---

## [ERR-20260728-HTTPFIXTURE] download_http_fixture_flake

**Logged**: 2026-07-28T14:56:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

One local single-stream speed-limit fixture failed only during the first combined all-target run.

### Error

```text
a_single_stream_download_uses_the_same_task_speed_limit: expected Completed, observed Failed
```

### Context

- The same exact test passed five consecutive isolated runs.
- The complete 25-test engine contract suite then passed twice consecutively.
- New capacity-boundary, sparse-file and file-lock tests remained deterministic.

### Suggested Fix

Treat a lone combined-fixture failure as inconclusive until the exact test and full engine suite are
repeated. Do not change production transfer logic without a repeatable red signal.

### Metadata

- Reproducible: no
- Related Files: `rust/crates/nxtools-download/tests/engine_contract.rs`
- Recurrence-Count: 4
- Last-Seen: 2026-07-30

### Resolution

- **Resolved**: 2026-07-28T14:56:00+08:00
- **Notes**: Exact test passed 5/5 and complete engine suite passed 2/2; no product change made.
  The same combined-suite-only failure recurred on 2026-07-30 after every preceding download test
  group passed; its immediate exact rerun passed, so production transfer logic remained unchanged.
  A later July 30 full-workspace run hit two independent local-fixture races: one crash-recovery
  range request received fixture HTTP 409 and one dynamic split had not started before the bounded
  pause checkpoint. Both exact contracts are rerun before treating either as a product regression.
  Both exact contracts passed immediately. A following combined run still intermittently missed the
  dynamic-split checkpoint and the single-stream speed-limit contract failed only in the suite;
  even a single-thread suite retained the speed-limit flake, while three immediate exact reruns all
  passed. No download-engine production code was changed by the task-window slice.

---

## [ERR-20260728-VERIFYTIMEOUT] cargo_verification_timeout

**Logged**: 2026-07-28T14:52:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

The download full-suite verification was accidentally launched with a one-second command timeout.

### Error

```text
command timed out after 5057 milliseconds
```

### Context

- The timeout terminated compilation before any test result was produced.
- This was a command-parameter error, not a product-code failure.

### Suggested Fix

Use the normal 120-second verification timeout while retaining the documented single-job,
non-incremental, high-codegen-unit environment.

### Metadata

- Reproducible: yes
- Related Files: `AGENTS.md`

### Resolution

- **Resolved**: 2026-07-28T14:52:00+08:00
- **Notes**: Relaunched the same gates with the correct timeout.

---

## [ERR-20260728-REFPATH] reference_path_assumption

**Logged**: 2026-07-28T14:35:01.7067847+08:00
**Priority**: low
**Status**: resolved
**Area**: docs

### Summary

An ABDM reference read assumed an outdated Kotlin package directory and missed the requested source files.

### Error

```text
Cannot find path ...\com\abdownloadmanager\downloader\utils\EmptyFileCreator.kt
Cannot find path ...\com\abdownloadmanager\downloader\utils\SparseFile.desktop.kt
```

### Context

- The repository uses `ir\amirab\downloader\utils`, not the assumed package path.
- The failed read did not modify product code.

### Suggested Fix

Locate reference entries with `rg --files` before opening paths whose package layout has not already
been recorded in the reference index.

### Metadata

- Reproducible: yes
- Related Files: `docs/reference-code/download.md`

### Resolution

- **Resolved**: 2026-07-28T14:35:01.7067847+08:00
- **Notes**: Located both files by repository filename index and continued from their actual paths.

---

## [ERR-20260728-083] cargo-index-curl-null-handle

**Logged**: 2026-07-28T00:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: infra

### Summary

Cargo panicked in its curl multi backend while refreshing the crates.io index before an otherwise
local Clippy run.

### Error

`curl multi.rs: assertion failed: !ptr.is_null()` followed by Cargo `SendError`.

### Context

- All dependencies were already present in `Cargo.lock` and the local cache.
- The failure occurred before compiling `nxtools-download`.

### Suggested Fix

Run package verification with `--offline` when no dependency resolution is required.

### Metadata

- Reproducible: yes
- Related Files: `Cargo.lock`

### Resolution

- **Resolved**: 2026-07-28T00:00:00+08:00
- **Notes**: Verification was rerun against the locked local dependency cache.

---

## [ERR-20260728-082] download-segment-test-context-not-declared

**Logged**: 2026-07-28T00:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

An HTTP segment contract test referenced a task ID that was not declared after the production API
was extended with an explicit diagnostic context.

### Error

`cannot find value task_id in this scope`

### Context

- The segment API gained a caller-supplied task ID and correlation ID for structured lifecycle logs.
- One test call reused a variable name from a neighboring test instead of constructing its own ID.

### Suggested Fix

Construct the complete diagnostic context locally at every public-boundary test call, then rerun the
same targeted test before broader verification.

### Metadata

- Reproducible: yes
- Related Files: `rust/crates/nxtools-download/tests/http_client_contract.rs`

### Resolution

- **Resolved**: 2026-07-28T00:00:00+08:00
- **Notes**: The test now creates its own stable task ID at the call site.

---

## [ERR-20260728-083] powershell-scriptblock-on-raw-sta-thread

**Logged**: 2026-07-28T12:35:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

A PowerShell script block used as a raw `System.Threading.Thread` callback crashed because the new
thread had no PowerShell Runspace.

### Error

`PSInvalidOperationException: There is no Runspace available to run scripts in this thread.`

### Context

- A read-only COM diagnostic attempted to compare `ITaskbarList3` activation on the current thread
  and a dedicated STA thread.
- The native probe was valid, but the thread callback itself was still a PowerShell ScriptBlock.

### Suggested Fix

Create and join the STA thread entirely inside compiled C# (`Add-Type`), and expose only the final
probe result back to PowerShell.

### Metadata

- Reproducible: yes
- Related Files: `csharp/NxTools.PlatformHost/Windows/TaskbarProgress.cs`

### Resolution

- **Resolved**: 2026-07-28T12:36:00+08:00
- **Notes**: The replacement probe owns its STA callback in compiled C# and does not require a
  PowerShell Runspace on the worker.

---

## [ERR-20260728-082] release-package-http-range-fixture-flake

**Logged**: 2026-07-28T11:20:00+08:00
**Priority**: medium
**Status**: resolved
**Area**: tests

### Summary

The full Release package gate was interrupted by an intermittent local HTTP probe assertion after
all preceding architecture, Clippy, clipboard, desktop and download-engine tests passed.

### Error

`probe_prefers_a_safe_content_disposition_name_and_falls_back_when_head_is_rejected` asserted
`fallback.supports_ranges`, but the observed fallback probe reported it as false.

### Context

- Command: `scripts/package.ps1 -Configuration Release`
- The failing test uses the local HTTP contract fixture and is unrelated to the PlatformHost
  completion-window action changes.
- The focused test must be repeated before deciding whether this is fixture nondeterminism or a
  real probe regression; packaging must not bypass verification.

### Suggested Fix

Run the focused contract repeatedly with the required low-memory Cargo environment. If it flakes,
make the fixture's HEAD/range request ordering and response headers deterministic before rerunning
the complete Release gate.

### Metadata

- Reproducible: yes, only when the eight local HTTP contracts run concurrently
- Related Files: `rust/crates/nxtools-download/tests/http_client_contract.rs`, `scripts/package.ps1`

### Resolution

- **Resolved**: 2026-07-28T11:35:00+08:00
- **Notes**: The tests now share an async contract gate and the fixture handles each connection
  without detached worker threads. The complete eight-test target passed 30 consecutive Release
  runs; temporary `[DEBUG-http-range]` probes were removed.

---

## [ERR-20260728-003] windows-powershell-wrapper-inherited-pwsh-module-path

**Logged**: 2026-07-28T10:48:00+08:00
**Priority**: low
**Status**: resolved
**Area**: infra

### Summary

A background `powershell.exe` wrapper inherited PowerShell 7's module path and could not resolve
`Get-FileHash` while assembling the Release package.

### Error

`Get-FileHash : The term 'Get-FileHash' is not recognized`

### Context

- The Rust Release binaries had already built successfully.
- The failure occurred in `scripts/write-release-binary-manifest.ps1`, before package copying.
- Running the same package script directly in the owning PowerShell 7 shell succeeded.

### Suggested Fix

Use `pwsh.exe` for background wrappers launched from a PowerShell 7 environment, or invoke the
package script directly. Do not launch Windows PowerShell with a PowerShell 7-only inherited
`PSModulePath`.

### Metadata

- Reproducible: yes
- Related Files: scripts/package.ps1, scripts/write-release-binary-manifest.ps1

### Resolution

- **Resolved**: 2026-07-28T10:48:00+08:00
- **Notes**: The cached low-memory Release build was assembled directly with PowerShell 7.

---

## [ERR-20260728-001] cargo-test-desktop-library-target

**Logged**: 2026-07-28T00:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

The desktop package was tested with `--lib` even though it has no library target.

### Error

`error: no library targets found in package 'nxtools-desktop'`

### Context

- Attempted `cargo test -p nxtools-desktop --lib -j 1`.
- The package exposes the `nxtools-desktop` binary target.

### Suggested Fix

Use `cargo test -p nxtools-desktop --bin nxtools-desktop -j 1`, or omit the target selector.

### Metadata

- Reproducible: yes
- Related Files: `rust/apps/nxtools-desktop/Cargo.toml`

### Resolution

- **Resolved**: 2026-07-28T00:00:00+08:00
- **Notes**: Verification was rerun against the correct binary target.

---

## [ERR-20260728-002] stale-download-source-regression-marker

**Logged**: 2026-07-28T00:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

A source-architecture test still expected a synchronous `this.file_name.update` call after the
implementation had moved the input update through a cloned entity on the source-window turn.

### Error

`downloads::tests::probe_completion_hydrates_the_visible_file_name_input` failed while the actual
`file_name.update(... input.set_value(...))` path remained present.

### Context

- The test scanned `downloads.rs` source rather than exercising the GPUI callback.
- The production code uses `Some((this.file_name.clone(), ...))` followed by
  `file_name.update(...)` to avoid re-entering a leased entity.

### Suggested Fix

Match the current safe entity handoff and the visible `input.set_value` call together.

### Metadata

- Reproducible: yes
- Related Files: `rust/apps/nxtools-desktop/src/downloads.rs`

### Resolution

- **Resolved**: 2026-07-28T00:00:00+08:00
- **Notes**: The stale source marker was updated without changing runtime behavior.

---

## [ERR-20260727-083] release-build-wrapper-timeout

**Logged**: 2026-07-27T17:43:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

完整 Release 构建门禁超过了单次 Shell 命令 120 秒时限，但底层 Cargo 编译仍在继续。

### Error

`command timed out after 124036 milliseconds`

### Context

- Command: `scripts/build.ps1 -Configuration Release`
- The command includes architecture gates, workspace Clippy/tests, Release linking and C# tests.
- Verification will continue by polling the existing build and splitting remaining checks into bounded steps.

### Suggested Fix

Run long Release gates through a yielded execution cell or split them into independently observable commands.

### Metadata

- Reproducible: environment-dependent
- Related Files: scripts/build.ps1

### Resolution

- **Resolved**: 2026-07-27T17:44:00+08:00
- **Notes**: The lingering Cargo process finished; remaining architecture, warning-free C# build,
  real-process regression suite and release packaging completed in bounded commands.

---

## [ERR-20260727-PD1] product-design user-context preflight path

**Logged**: 2026-07-27T17:12:00+08:00
**Priority**: low
**Status**: resolved
**Area**: docs

### Summary
The Product Design user-context skill documents its preflight script relative to the skill directory,
not the plugin root.

### Error
```
can't open file '<product-design-plugin-root>\scripts\user_context_preflight.py'
```

### Context
- The audit workflow required the Product Design user-context preflight.
- The actual script is under `skills/user-context/scripts/user_context_preflight.py`.

### Suggested Fix
Resolve script references relative to the `SKILL.md` that declares them.

### Metadata
- Reproducible: yes
- Related Files: Product Design `skills/user-context/SKILL.md`

### Resolution
- **Resolved**: 2026-07-27T17:12:00+08:00
- **Notes**: Located the packaged script and reran it from the owning skill directory.

---

## [ERR-20260727-082] release-package-wrapper-timeout

**Logged**: 2026-07-27T15:28:00+08:00
**Priority**: low
**Status**: resolved
**Area**: infra

### Summary

The Release packaging wrapper exceeded the tool's 120-second command timeout while Cargo was still
successfully compiling in child processes.

### Error

`command timed out after 124028 milliseconds`

### Context

- `scripts/package.ps1 -Configuration Release -SkipVerification`
- Single Cargo job, incremental compilation disabled, 256 codegen units.
- The wrapper was terminated, but its Cargo/rustc children remained active and required polling
  rather than starting a competing build.

### Suggested Fix

Run the Release build as a yielded long-running cell or split compilation, manifest generation,
.NET publish and package copying into resumable steps.

### Metadata

- Reproducible: environment-dependent
- Related Files: scripts/package.ps1

### Resolution

- **Resolved**: 2026-07-27T15:36:00+08:00
- **Notes**: Split the long Cargo Release build into a yielded cell, then reran the cached packaging
  wrapper for manifest generation, .NET publish and file copying. The package completed successfully.

---

## [ERR-20260727-005] download-fixture-clippy-collapsible-if

**Logged**: 2026-07-27T00:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

The new deterministic HTTP test fixture passed its behavior tests but failed the workspace's strict
Clippy gate because a nested range-validation branch was collapsible.

### Error

`clippy::collapsible-if` in `tests/support/mod.rs`, promoted to an error by `-D warnings`.

### Context

- Command: `cargo clippy -p nxtools-download --all-targets -j 1 -- -D warnings`
- Product code and all 14 tests had already passed.
- The failure confirms that reusable test-support code is part of the release lint surface.

### Suggested Fix

Use an `if let` chain for range parsing and bounds validation, then rerun the complete crate test and
Clippy gates without lowering lint severity.

### Metadata

- Reproducible: yes
- Related Files: rust/crates/nxtools-download/tests/support/mod.rs

### Resolution

- **Resolved**: 2026-07-27T00:00:00+08:00
- **Notes**: Collapsed the conditional and retained the strict `-D warnings` gate.

---

## [ERR-20260727-001] clippy-assertions-on-constants

**Logged**: 2026-07-27T00:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

Strict Clippy rejected a runtime `assert!` whose operands were compile-time UI constants.

### Error

`clippy::assertions-on-constants`: consider moving the assertion into a const block.

### Context

- The UI hierarchy test compared three `f32` typography constants.
- Exact-value assertions already covered each constant, so the extra constant boolean assertion was redundant.

### Suggested Fix

Keep the exact-value assertions and remove the redundant constant boolean assertion, or use a compile-time
const assertion when only ordering matters.

### Metadata

- Reproducible: yes
- Related Files: rust/apps/nxtools-desktop/src/quick_launch.rs

### Resolution

- **Resolved**: 2026-07-27T00:00:00+08:00
- **Notes**: Removed the redundant assertion and reran strict Clippy.

---

## [ERR-20260727-002] release-package-command-timeout

**Logged**: 2026-07-27T00:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: infra

### Summary

The first low-memory Release packaging call exceeded a two-minute shell timeout before Cargo linked
the desktop executable.

### Error

`command timed out after 124050 milliseconds`

### Context

- The package script was correct and no Cargo/Rust child process remained afterward.
- Release artifact timestamps confirmed that the previous package had not been overwritten.

### Suggested Fix

Use the same single-job, non-incremental low-memory environment with a longer command timeout.

### Metadata

- Reproducible: environment-dependent
- Related Files: scripts/package.ps1
- See Also: ERR-20260724-081

### Resolution

- **Resolved**: 2026-07-27T00:00:00+08:00
- **Notes**: Retried packaging with a longer bounded timeout and verified output timestamps.

---

## [ERR-20260727-003] rustc-windows-crate-stack-buffer-overrun

**Logged**: 2026-07-27T00:00:00+08:00
**Priority**: medium
**Status**: resolved
**Area**: infra

### Summary

Forcing `-C codegen-units=256` for a full Release rebuild caused `rustc` to crash while compiling
the feature-heavy `windows` crate.

### Error

`STATUS_STACK_BUFFER_OVERRUN (0xc0000409)` while compiling `windows 0.61.3`.

### Context

- The low-memory flag had previously helped final linking, but changing global `RUSTFLAGS` invalidated
  the existing Release cache and applied 256 codegen units to every dependency.
- The crash happened before project linking and was unrelated to the UI source changes.

### Suggested Fix

Prefer the existing default-flag Release cache with `-j 1`. Apply special codegen settings only when
the final project link actually exhausts memory, not to every dependency in a clean rebuild.

### Metadata

- Reproducible: environment-dependent
- Related Files: scripts/package.ps1
- See Also: ERR-20260724-081

### Resolution

- **Resolved**: 2026-07-27T00:00:00+08:00
- **Notes**: Retried with default Rust flags and a single Cargo job so the existing Release cache remained reusable.

---

## [ERR-20260727-004] powershell-get-content-format-stack-overflow

**Logged**: 2026-07-27T00:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tooling

### Summary

A simple `Get-Content -TotalCount` inspection crashed PowerShell's formatting pipeline with a stack
overflow.

### Error

`Stack overflow` inside `System.Management.Automation.Language.Compiler` and the format output manager.

### Context

- The source file itself was valid and had already compiled.
- The failure occurred while printing file content, not while parsing Rust.

### Suggested Fix

Use `rg -n` or bounded indexed reads for source inspection and avoid treating PowerShell formatter
failures as source failures.

### Metadata

- Reproducible: unknown
- Related Files: rust/apps/nxtools-desktop/src/ui/mod.rs

### Resolution

- **Resolved**: 2026-07-27T00:00:00+08:00
- **Notes**: Continued inspection with ripgrep and patch context.

---

## [ERR-20260725-082] rustfmt-default-edition-mismatch

**Logged**: 2026-07-25T00:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

Direct `rustfmt` invocation parsed the Rust 2024 source with an older default edition.

### Error

`let chains are only allowed in Rust 2024 or later`

### Context

- The targeted format check ran `rustfmt rust/apps/nxtools-desktop/src/main_window.rs`.
- The workspace source legitimately uses Rust 2024 let-chains.

### Suggested Fix

Pass `--edition 2024` when formatting an individual source file outside Cargo.

### Metadata

- Reproducible: yes
- Related Files: rust/apps/nxtools-desktop/src/main_window.rs

### Resolution

- **Resolved**: 2026-07-25T00:00:00+08:00
- **Notes**: Re-ran targeted formatting with `rustfmt --edition 2024`.

---

## [ERR-20260725-001] environment-multipath-stale-cardinality-assertion

**Logged**: 2026-07-25T00:25:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

The full Finder suite retained a single-result `%PATH%` assertion after multi-path expansion was
implemented.

### Error

`resolved_variable_owns_the_query_instead_of_merging_file_candidates: left 10, right 1`

### Context

- `%PATH%` now intentionally emits one result per bounded, deduplicated segment.
- The test's actual invariant was that an external file candidate must not be merged.

### Suggested Fix

Assert that the bounded result set is non-empty, contains only environment-variable results, and
does not include the injected external candidate instead of asserting cardinality one.

### Metadata

- Reproducible: yes
- Related Files: `rust/crates/nxtools-finder/tests/environment_variable.rs`

### Resolution

- **Resolved**: 2026-07-25T00:26:00+08:00
- **Notes**: Updated the assertion to match the intended provider-ownership invariant.

---

## [ERR-20260724-084] process-probe-partial-name-miss

**Logged**: 2026-07-24T18:25:00+08:00
**Priority**: low
**Status**: resolved
**Area**: infra

### Summary

A read-only build-process probe returned exit code 1 because some requested process names were not
running, even though it successfully showed active Cargo and rustc processes.

### Error

`Get-Process cargo,rustc,dotnet,pwsh -ErrorAction SilentlyContinue` returned exit code 1.

### Context

- The probe was used only to verify that Release packaging was actively compiling.
- Active `cargo` and `rustc` processes confirmed the build was not stalled.

### Suggested Fix

Query each optional process name through `Get-Process -ErrorAction SilentlyContinue` in a loop, or
filter the full process list, so missing optional names do not mark the probe failed.

### Metadata

- Reproducible: yes
- Related Files: none

### Resolution

- **Resolved**: 2026-07-24T18:26:00+08:00
- **Notes**: Packaging was left running and completed successfully.

---

## [ERR-20260724-083] package-command-timeout-too-short

**Logged**: 2026-07-24T17:35:00+08:00
**Priority**: low
**Status**: resolved
**Area**: infra

### Summary

The Release packaging command was started with a one-second process timeout and was terminated
before the build could complete.

### Error

`command timed out after 1030 milliseconds`

### Context

- Operation: `scripts/package.ps1 -Configuration Release`
- The timeout was a tool-call parameter mistake, not a compiler or packaging failure.

### Suggested Fix

Use a multi-minute command timeout and rely on the execution cell's normal incremental yielding.

### Metadata

- Reproducible: yes
- Related Files: `scripts/package.ps1`

### Resolution

- **Resolved**: 2026-07-24T17:36:00+08:00
- **Notes**: Retried with a 10-minute process timeout.

---

## [ERR-20260724-082] documentation-patch-context-mismatch

**Logged**: 2026-07-24T17:10:00+08:00
**Priority**: low
**Status**: resolved
**Area**: docs

### Summary

A combined documentation patch used a wrapped sentence as a large context anchor and did not apply.

### Error

`apply_patch verification failed: Failed to find expected lines`

### Context

- The first attempted operation updated the phase plan and reference-research note together.
- The target sentence existed, but its actual line wrapping differed from the patch context.
- The same mismatch recurred in a later multi-file Rust patch after `cargo fmt` changed wrapping.

### Suggested Fix

Read the immediate target ranges and apply separate patches anchored on stable section headings or
exact short lines.

### Metadata

- Reproducible: yes
- Recurrence-Count: 6
- Related Files: `docs/quick-launch-plan.md`, `docs/reference-code/quick-launch-research.md`

### Resolution

- **Resolved**: 2026-07-24T17:11:00+08:00
- **Notes**: Retried with narrow section anchors.

---

## [ERR-20260724-069] cargo-test-multiple-positional-filters

**Logged**: 2026-07-24T15:05:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

One `cargo test` invocation passed two positional test filters, but Cargo accepts only one.

### Error

`error: unexpected argument 'calculator_' found`

### Context

- The command attempted to run the asset embedding test and calculator-prefixed tests together.
- This was a command construction error; neither test suite nor product code had failed.

### Suggested Fix

Run the two filters as separate `cargo test` commands, or use one shared substring only when the
test names deliberately share it.

### Metadata

- Reproducible: yes
- Related Files: `rust/apps/nxtools-desktop/src/assets.rs`,
  `rust/apps/nxtools-desktop/src/quick_launch.rs`

### Resolution

- **Resolved**: 2026-07-24T15:05:00+08:00
- **Notes**: Verification was split into independent test invocations.

---

## [ERR-20260724-069] rust-test-gpui-metadata-pagefile-limit

**Logged**: 2026-07-24T13:02:00+08:00
**Priority**: medium
**Status**: in_progress
**Area**: tests

### Summary

The focused desktop regression test could not load the existing GPUI Rust metadata because
Windows exhausted its commit/page-file budget.

### Error

`failed to mmap ... libgpui-*.rlib: 页面文件太小，无法完成操作。 (os error 1455)`

### Context

- Command: `cargo test -p nxtools-desktop initial_hidden_gpui_window_defers_native_placement_and_visibility -- --exact --nocapture`
- The compiler failed before entering the requested test.
- The failure is environmental and does not count as a red or green regression verdict.

### Suggested Fix

Retry with the repository's low-memory Rust build settings and no concurrent compilation; if the
commit limit remains exhausted, use the deterministic architecture guard during iteration and run
the complete Rust suite after memory pressure falls.

### Metadata

- Reproducible: yes
- Related Files: `rust/apps/nxtools-desktop/src/quick_launch/file_dialog_search.rs`
- See Also: the machine had only about 2.7 GiB of free commit while an unrelated Zed process
  reserved about 14.7 GiB; do not terminate unrelated user processes

---

## [ERR-20260724-066] powershell-datetimeoffset-filter-type-mismatch

**Logged**: 2026-07-24T11:12:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tooling

### Summary

A read-only JSONL log filter compared a deserialized `DateTime` value with a
`DateTimeOffset` literal and aborted the parallel inspection command.

### Error

`Could not compare "07/24/2026 09:44:43" to "07/24/2026 11:04:44 +08:00".`

### Context

- The command was narrowing PlatformHost events to a three-second diagnostic window.
- The failure did not modify application state or source code.

### Suggested Fix

Normalize both sides to `DateTimeOffset`, or filter the ISO-8601 timestamp text before
deserializing the matching records.

### Metadata

- Reproducible: yes
- Related Files: `C:\Users\Administrator\AppData\Local\NxTools\NxTools.Next\data\logs\platform-host.log.2026-07-24.jsonl`

### Resolution

- **Resolved**: 2026-07-24T11:12:00+08:00
- **Notes**: Subsequent probes normalize the deserialized timestamp before comparison.

---

## [ERR-20260724-056] swiftlist-inline-search-path-assumption

**Logged**: 2026-07-24T10:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: reference-research

### Summary

The first reference lookup assumed `InlineSearchWindow.xaml` lived directly under `App\Views`;
SwiftList stores it in the `App\Views\InlineSearchWindow` subdirectory.

### Resolution

Locate reference files with `rg --files` before opening an assumed path.

### Metadata

- Reproducible: yes
- Related Files: `D:\Code\SwiftList-main\App\Views\InlineSearchWindow\InlineSearchWindow.xaml`

---

## [ERR-20260724-057] dnspy-console-help-switch

**Logged**: 2026-07-24T10:01:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tooling

### Summary

`dnSpy.Console.exe --help` returned exit code 1 because that console build does not accept the
conventional help switch, even though it printed its usage.

### Resolution

Use the usage emitted by the no-argument command and the already generated managed-source output;
do not treat `--help` as a portable dnSpy.Console option.

### Metadata

- Reproducible: yes
- Related Files: `D:\Tools\dnSpy-net-win64\dnSpy.Console.exe`

---

## [ERR-20260724-058] ripgrep-windows-wildcard-path

**Logged**: 2026-07-24T10:02:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tooling

### Summary

A reference search passed a Windows wildcard as a path argument to `rg`; ripgrep does not expand
that wildcard in this PowerShell invocation.

### Resolution

Pass the containing directory and use `-g` globs for file filtering.

### Metadata

- Reproducible: yes
- Related Files: none
- Pattern-Key: powershell.rg_windows_wildcard_path
- Recurrence-Count: 2
- First-Seen: 2026-07-24
- Last-Seen: 2026-08-08

The pattern recurred while locating the old browser-extension packaging call chain: the exact
results were valid, but a trailing `*.csproj` path made the read-only command exit nonzero. Future
searches must pass the repository directory and select project files with `-g '*.csproj'`.

---

## [ERR-20260724-056] file-dialog-search-cache-revert-delimiter

**Logged**: 2026-07-24T00:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: frontend

### Summary

Reverting an unsafe file-dialog window-command cache left one extra Rust closing delimiter.

### Error

`cargo fmt --all -- --check` and `cargo check -p nxtools-desktop` reported an unexpected closing
delimiter in `quick_launch/file_dialog_search.rs`.

### Context

- The cache was rejected because `send_command` confirms only enqueueing; it does not confirm that
  the correlated Windows command executed successfully.
- The immediate format/check gate caught the mechanical revert error before packaging.

### Suggested Fix

After replacing a whole function body, inspect the edited boundary and run the parser/formatter
before any broader build.

### Metadata

- Reproducible: yes
- Related Files: rust/apps/nxtools-desktop/src/quick_launch/file_dialog_search.rs

### Resolution

- **Resolved**: 2026-07-24T00:00:00+08:00
- **Notes**: Removed the extra delimiter and retained retryable command dispatch.

---

## [ERR-20260724-057] duplicated-workspace-prefix

**Logged**: 2026-07-24T00:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

A follow-up `rg` check used a workspace-relative path while its working directory was already the
`rust` subdirectory.

### Error

`rg` reported that `rust/apps/nxtools-desktop/...` did not exist.

### Context

- The preceding `cargo check -p nxtools-desktop` completed successfully.
- Only the trailing source-location check failed.

### Suggested Fix

Resolve command paths relative to the explicit `workdir`; do not repeat the subdirectory prefix.

### Metadata

- Reproducible: yes
- Related Files: none

### Resolution

- **Resolved**: 2026-07-24T00:00:00+08:00
- **Notes**: Re-ran the source check with `apps/nxtools-desktop/...`.

---

## [ERR-20260723-071] powershell-foreach-object-piped-without-grouping

**Logged**: 2026-07-23T18:46:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tooling

### Summary

A final package-hash report piped directly after a statement-level PowerShell `foreach` block,
which produced an empty-pipe parser error.

### Error

`ParserError: An empty pipe element is not allowed.`

### Context

- The Release package had already completed successfully.
- The command only formatted artifact/protected hash evidence.

### Suggested Fix

Collect `foreach` output into an array first, then pipe that array to `Format-Table`.

### Metadata

- Reproducible: yes
- Related Files: none

### Resolution

- **Resolved**: 2026-07-23T18:47:00+08:00
- **Notes**: Re-ran the report with an explicit `$rows` array.

---

## [ERR-20260723-070] protected-package-rename-staled-architecture-gate

**Logged**: 2026-07-23T18:38:00+08:00
**Priority**: medium
**Status**: resolved
**Area**: tests

### Summary

The Quick Launch architecture gate still required the retired desktop-only
`protected_copy_matches_release` symbol after the implementation was strengthened to compare the
entire protected package.

### Error

The elevated-navigation gate failed even though the Rust/C#/LocalSystem privilege boundary remained
unchanged. A companion inspection also passed a wildcard path directly to ripgrep on Windows and
reported an invalid path syntax error.

### Context

- The implementation now uses `protected_package_matches_release`.
- The new invariant additionally requires `NxTools.Indexer.exe` and
  `NxTools.PlatformHost.exe` sibling comparisons.

### Suggested Fix

Update the gate to assert the stronger package-level symbol and both protected sibling binaries.
Discover Windows wildcard paths before passing them to ripgrep.

### Metadata

- Reproducible: yes
- Related Files: tests/architecture/quick-launch-lifecycle.ps1, rust/apps/nxtools-desktop/src/main.rs
- See Also: ERR-20260723-055

### Resolution

- **Resolved**: 2026-07-23T18:39:00+08:00
- **Notes**: The gate now requires atomic desktop, indexer and PlatformHost matching.

---

## [ERR-20260723-069] architecture-script-exit-skipped-second-gate

**Logged**: 2026-07-23T18:34:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

Two PowerShell architecture gates were invoked sequentially in one shell command, but the first
script terminated the host process after passing, so the second gate never ran.

### Error

The command returned success while printing only the indexer lifecycle PASS line and no Quick
Launch lifecycle output.

### Context

- No project code failed.
- A missing expected PASS line was treated as a failed verification rather than inferred success.

### Suggested Fix

Run architecture scripts that may call `exit` in separate shell invocations.

### Metadata

- Reproducible: yes
- Related Files: tests/architecture/indexer-lifecycle.ps1, tests/architecture/quick-launch-lifecycle.ps1
- See Also: ERR-20260722-046

### Resolution

- **Resolved**: 2026-07-23T18:35:00+08:00
- **Notes**: Ran the Quick Launch architecture gate in its own PowerShell process.

---

## [ERR-20260723-068] protected-package-test-rustfmt-diff

**Logged**: 2026-07-23T18:27:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

The first formatting gate after adding the protected three-binary package regression found one
multi-line `std::fs::write` call that did not match rustfmt output.

### Error

`cargo fmt --all -- --check` reported a single formatting diff in the new desktop test.

### Context

- The difference was whitespace/layout only.
- No product behavior or test assertion changed.

### Suggested Fix

Run `cargo fmt --all` after the test reaches green, then repeat the check as an independent gate.

### Metadata

- Reproducible: yes
- Related Files: rust/apps/nxtools-desktop/src/main.rs

### Resolution

- **Resolved**: 2026-07-23T18:28:00+08:00
- **Notes**: Applied rustfmt and repeated the formatting check.

---

## [ERR-20260723-067] exact-rust-test-filter-omitted-module-path

**Logged**: 2026-07-23T17:51:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

A focused Rust test command combined a bare function name with `--exact`, so Cargo successfully ran
zero tests instead of exercising the new regression.

### Error

The test output reported `0 passed; ... 63 filtered out`.

### Context

- Rust's exact filter compares the full test name, including the module prefix.
- A zero-test success is not accepted as a feedback-loop result.

### Suggested Fix

Either omit `--exact` for a unique function-name filter or pass the full module-qualified test name.

### Metadata

- Reproducible: yes
- Related Files: rust/apps/nxtools-indexer/src/lib.rs

### Resolution

- **Resolved**: 2026-07-23T17:52:00+08:00
- **Notes**: Re-ran with the unique function-name filter and no `--exact`.

---

## [ERR-20260723-066] indexer-test-parallel-build-exhausted-commit

**Logged**: 2026-07-23T17:48:00+08:00
**Priority**: medium
**Status**: resolved
**Area**: tests

### Summary

The first focused indexer regression-test build used Cargo's default parallelism while Windows had
insufficient free virtual-memory commit, so rustc failed before reaching the new assertion.

### Error

Rust reported OS error 1455, metadata mmap failures and allocation failures while compiling shared
Windows dependencies.

### Context

- The intended test covers planned USN rebuild state being painted as a user-facing error.
- The failure was environmental and is not accepted as the required red-capable product signal.

### Suggested Fix

Set `CARGO_BUILD_JOBS=1` for indexer/GPUI builds on this workstation and prefer the already-warm
profile when a focused test needs to compile.

### Metadata

- Reproducible: environment-dependent
- Related Files: rust/apps/nxtools-indexer/src/lib.rs
- See Also: ERR-20260723-060, ERR-20260722-050, ERR-20260722-052

### Resolution

- **Resolved**: 2026-07-23T17:49:00+08:00
- **Notes**: Re-ran the focused test with one Cargo build job.

---

## [ERR-20260723-065] localsystem-profile-log-probe-access-denied

**Logged**: 2026-07-23T17:44:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tooling

### Summary

A read-only diagnostic attempted to probe the LocalSystem profile's application-data log path and
PowerShell emitted an access-denied error.

### Error

`Test-Path: Access to ... systemprofile\AppData\Local\NxTools... is denied.`

### Context

- The service status was already available through the authenticated indexer pipe.
- No privileged filesystem access was required to diagnose the reported state.

### Suggested Fix

Prefer the product's authenticated status protocol and user-visible logs; do not probe protected
service-profile directories from an unelevated diagnostic session.

### Metadata

- Reproducible: yes
- Related Files: tests/performance/indexer-service-smoke.ps1

### Resolution

- **Resolved**: 2026-07-23T17:45:00+08:00
- **Notes**: Switched to the indexer pipe status probe.

---

## [ERR-20260723-064] optional-rg-miss-failed-combined-probe

**Logged**: 2026-07-23T17:38:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tooling

### Summary

A combined diagnostic command returned a failure because its final optional ripgrep search had no
matches, even though the preceding exact searches succeeded.

### Error

PowerShell returned exit code 1 after `rg` correctly found the reported MFT error string but a later
optional search found no existing regression fixture.

### Context

- The command was read-only.
- A no-match result was expected while checking whether the bug already had a test.

### Suggested Fix

Run required and optional searches separately, or explicitly convert an expected ripgrep no-match
exit code into a typed `NO_MATCH` result.

### Metadata

- Reproducible: yes
- Related Files: rust/apps/nxtools-indexer/src/usn.rs

### Resolution

- **Resolved**: 2026-07-23T17:39:00+08:00
- **Notes**: Subsequent optional searches are isolated and their no-match status is handled.

---

## [ERR-20260723-063] ripgrep-leading-pattern-parsed-as-option

**Logged**: 2026-07-23T17:24:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tooling

### Summary

A source-inspection search started its pattern with `--install-indexer-service`, so ripgrep parsed
the pattern as an unsupported command-line option.

### Error

`rg: unrecognized flag --install-indexer-service|Install.*Service|protected`

### Context

- The failed command was read-only and did not affect project state.
- The other inspection commands in the same PowerShell invocation had already returned useful
  output, but the combined invocation correctly remained failed.

### Suggested Fix

Insert ripgrep's `--` end-of-options marker before any pattern that can begin with a hyphen.

### Metadata

- Reproducible: yes
- Related Files: none

### Resolution

- **Resolved**: 2026-07-23T17:25:00+08:00
- **Notes**: Re-ran the search as `rg -n -- "..." ...`.

---

## [ERR-20260723-062] optional-protected-state-probe-exited-nonzero

**Logged**: 2026-07-23T16:57:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

A diagnostic command treated absent optional protected-install state as a PowerShell command failure.

### Error

The combined probe returned exit code 1 after confirming the service, protected directory and HKLM
installation record were absent.

### Context

- The absence was the expected diagnostic signal for the reported missing index volumes.
- `Get-ChildItem`/`Get-ItemProperty` were made non-terminating, but the combined command still
  surfaced a nonzero process status.

### Suggested Fix

For optional external state, test `Test-Path` and provider availability explicitly, print a typed
`ABSENT` result, and finish with an explicit successful exit rather than relying on provider error
semantics.

### Metadata

- Reproducible: yes
- Related Files: none

### Resolution

- **Resolved**: 2026-07-23T16:57:00+08:00
- **Notes**: Subsequent service/install probes use explicit presence branches.

---

## [ERR-20260723-061] windowsapps-cdb-direct-launch-denied

**Logged**: 2026-07-23T16:45:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

Directly launching the Microsoft Store WinDbg `cdb.exe` below `WindowsApps` was denied even though
the executable was discoverable by filesystem enumeration.

### Error

`Program 'cdb.exe' failed to run ... WindowsApps ... Access denied.`

### Context

- Attempted to analyze an existing NxTools native crash dump without restarting the UI.
- The Store package exposes the debugger binary but applies package execution restrictions to its
  physical path.

### Suggested Fix

Use the package execution alias, invoke the debugger through its app-package launcher, or copy the
debugger package payload to a user-executable temporary directory before offline analysis.

### Metadata

- Reproducible: yes
- Related Files: none

### Resolution

- **Resolved**: 2026-07-23T16:47:00+08:00
- **Notes**: Copied the 34 MB `amd64` debugger payload to a user-executable temporary directory
  and ran its `cdb.exe` against the existing dump with the local Rust Release PDB. This recovered
  the complete `gpui::app::entity_map::double_lease_panic` stack without restarting NxTools.

---

## [ERR-20260723-060] parallel-rust-check-exhausted-windows-commit

**Logged**: 2026-07-23T16:05:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

A release `cargo check` rebuilt the Windows dependency graph with the default parallel job count
while the workstation had little free virtual-memory commit. Several rustc workers failed with
allocation errors and Windows error 1453.

### Error

`cargo check -p nxtools-protocol -p nxtools-indexer -p nxtools-desktop --release` failed in
unrelated dependencies with “配额不足” and allocation failures.

### Context

- The C# build in the same command completed successfully.
- Free physical memory was available, but free virtual-memory commit was below 2 GiB.
- No source diagnostic preceded the rustc worker crashes.

### Suggested Fix

On this workstation, set `CARGO_BUILD_JOBS=1` (or at most 2) for large feature-triggered rebuilds.
Do not kill unrelated user applications to reclaim memory.

### Metadata

- Reproducible: environment-dependent
- Related Files: Cargo.toml

### Resolution

- **Resolved**: 2026-07-23T16:07:00+08:00
- **Notes**: Re-ran the same release check with one Cargo build job; it completed successfully.

---

## [ERR-20260723-059] powershell-help-switch-ran-runtime-smoke

**Logged**: 2026-07-23T15:10:00+08:00
**Priority**: medium
**Status**: resolved
**Area**: tests

### Summary

Passing `-?` to the Quick Launch runtime smoke script did not print usage; PowerShell bound the
script's optional parameters and executed the smoke test, briefly launching the packaged UI despite
the manual-testing constraint.

### Error

`pwsh -File tests/runtime/quick-launch-search-smoke.ps1 -?` ran the test and reported a desktop PID.

### Context

- The user explicitly owns interactive UI testing.
- The script has no help handler, so `-?` is not a safe read-only inspection mechanism.

### Suggested Fix

Read a script's `param` block with `Get-Content` before invoking it. Never probe project scripts with
`-?` unless comment-based help is known to exist. After an accidental launch, stop only the exact
packaged executable path.

### Metadata

- Reproducible: yes
- Related Files: tests/runtime/quick-launch-search-smoke.ps1

### Resolution

- **Resolved**: 2026-07-23T15:10:00+08:00
- **Notes**: Checked for and stopped only the exact artifact path if it remained running. No
  further UI runtime tests will be launched without the user's request.

---

## [ERR-20260723-058] csharp-discard-shadowed-by-lambda-parameter

**Logged**: 2026-07-23T14:20:00+08:00
**Priority**: low
**Status**: resolved
**Area**: backend

### Summary

A Win32 enumeration callback named its unused `IntPtr` parameter `_`, so a later `_ = uintValue`
statement assigned to that parameter instead of using a C# discard.

### Error

`CS0266: Cannot implicitly convert type 'uint' to 'nint'.`

### Context

- Operation: build the Directory Opus exact-process command-window lookup.
- The failure was compile-time and no package was produced from the bad source.

### Suggested Fix

Do not name lambda parameters `_` when the callback body also uses discard assignments; give the
parameter a harmless explicit name.

### Metadata

- Reproducible: yes
- Related Files: csharp/NxTools.PlatformHost/Windows/DirectoryOpusFileManagerOperations.cs

### Resolution

- **Resolved**: 2026-07-23T14:21:00+08:00
- **Notes**: Renamed the callback parameter and retained the discard assignment.

---

## [ERR-20260723-057] powershell-pid-reserved-variable

**Logged**: 2026-07-23T14:12:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

A read-only Win32 window probe attempted to assign `$pid`, which is PowerShell's read-only automatic
`$PID` variable, so the reported owner process IDs were invalid.

### Error

`Cannot overwrite variable PID because it is read-only or constant.`

### Context

- Operation: verify that Directory Opus's lister and command window belong to the same process.
- The product implementation was not involved.

### Suggested Fix

Use a non-reserved name such as `$ownerProcessId` for P/Invoke out parameters.

### Metadata

- Reproducible: yes
- Related Files: none

### Resolution

- **Resolved**: 2026-07-23T14:13:00+08:00
- **Notes**: Reran the probe with `$ownerProcessId`.

---

## [ERR-20260723-056] protocol-test-context-mismatch

**Logged**: 2026-07-23T13:48:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

An `apply_patch` attempt guessed an outdated Rust protocol test function name and found no matching
context.

### Error

`apply_patch verification failed: Failed to find expected lines ... fn
quick_launch_configuration_and_request_round_trip()`

### Context

- Operation: add the third-party file-manager host round-trip fixture.
- The actual test is named `quick_launch_hotkey_and_panel_request_round_trip`.

### Suggested Fix

Locate the exact current test boundary with `rg` before patching a large, frequently edited test
module.

### Metadata

- Reproducible: yes
- Related Files: rust/crates/nxtools-protocol/src/lib.rs

### Resolution

- **Resolved**: 2026-07-23T13:49:00+08:00
- **Notes**: Located the actual function and resumed with its exact context.

---

## [ERR-20260723-056] assumed-reference-source-path

**Logged**: 2026-07-23T14:20:00+08:00
**Priority**: low
**Status**: resolved
**Area**: docs

### Summary

A SwiftList reference file was read from an assumed directory even though the repository keeps it
one level deeper under `Core/Hook/ExplorerIntegration`.

### Error

`Get-Content` could not find `Core/Hook/ExplorerTracker.cs`. A follow-up search command also passed
PowerShell's `-ErrorAction` token to `rg`, which `rg` interpreted as its own `-E` option.

### Context

- The reference sweep needed the real `ExplorerTracker.cs` path.
- The file was subsequently located with `rg --files ... | rg 'ExplorerTracker\.cs$'`.

### Suggested Fix

Discover uncertain reference paths with `rg --files` before reading them, and keep PowerShell
parameters attached only to PowerShell cmdlets rather than commands such as `rg`.

### Metadata

- Reproducible: yes
- Related Files: D:\Code\SwiftList-main\Core\Hook\ExplorerIntegration\ExplorerTracker.cs

### Resolution

- **Resolved**: 2026-07-23T14:20:00+08:00
- **Notes**: The exact path was discovered and the reference review continued from it.

---

## [ERR-20260723-055] diagnostic-probe-out-parameter-capture

**Logged**: 2026-07-23T11:25:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

An inline C# file-dialog diagnostic failed to compile because its enumeration lambda assigned
directly to an `out` parameter.

### Error

`CS1628: Cannot use ref, out, or in parameter 'address' inside an anonymous method, lambda
expression, query expression, or local function.`

### Context

- The probe enumerated native child windows and attempted to return the discovered address edit.
- This affected only the temporary diagnostic harness, not product code.

### Suggested Fix

Capture the handle in an ordinary local variable inside the lambda, then assign that local to the
`out` parameter after enumeration completes.

### Metadata

- Reproducible: yes
- Related Files: csharp/NxTools.PlatformHost/Windows/FileDialogWindowAdapter.cs

### Resolution

- **Resolved**: 2026-07-23T11:25:00+08:00
- **Notes**: The next probe uses a local capture variable instead of closing over an `out` parameter.

---

## [ERR-20260722-055] quick-launch-focus-test-fixture-and-tooling

**Logged**: 2026-07-22T17:20:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

The first regression fixture compared against a selection value captured before an asynchronous
search refresh, and two diagnostic commands used the wrong protocol method names and PowerShell's
reserved `$Host` variable. Strict Clippy also rejected cloning the `Copy`-typed `AnyWindowHandle`
in the new test.

### Resolution

Compare keyboard movement with the selection immediately before the second key, use
`encode_line`/`decode_line`, and name the log variable `$hostLog`. The final architecture gate was
also proven red by removing only the focus-restoration call before restoring the implementation;
the test now copies the window handle directly.

### Metadata

- Reproducible: yes
- Related Files: rust/apps/nxtools-desktop/src/quick_launch.rs, rust/crates/nxtools-protocol/src/lib.rs, tests/architecture/quick-launch-lifecycle.ps1

---

## [ERR-20260722-045] usewpf-changed-generated-global-usings

**Logged**: 2026-07-22T13:18:00+08:00
**Priority**: low
**Status**: resolved
**Area**: build

### Summary

Enabling `<UseWPF>true</UseWPF>` in the existing WinForms platform-host project changed its
generated global-using set and removed `System.IO`, breaking unrelated files that relied on the
WinForms SDK defaults.

### Resolution

Keep the existing WinForms project mode and add only the `Microsoft.WindowsDesktop.App.WPF`
framework reference needed for `Imaging.CreateBitmapSourceFromHBitmap`. This preserves current
implicit usings while matching the reference bitmap conversion boundary.

### Metadata

- Reproducible: yes
- Related Files: csharp/NxTools.PlatformHost/NxTools.PlatformHost.csproj
- Pattern-Key: dotnet.windowsdesktop.generated_global_usings
- Recurrence-Count: 1

---

## [ERR-20260722-039] listary-decompilation-path-assumption

**Logged**: 2026-07-22T11:10:00+08:00
**Priority**: low
**Status**: resolved
**Area**: docs
**Recurrence-Count**: 2
**Last-Seen**: 2026-07-22

### Summary

A reference inspection command assumed `Class431.cs` lived under the `Implementations` directory,
but `rg` had reported it under `Listary/Core/Commands`.

### Error

```text
Cannot find path '.../Core/Commands/Implementations/Class431.cs' because it does not exist.
```

### Resolution

- Use the exact path returned by `rg -l` before reading obfuscated/decompiled reference types.
- The failed command was read-only and made no project changes.
- The same pattern later guessed a standalone `JsonDefaults.cs` after `rg` had already located the
  type in `ProtocolEnvelope.cs`; exact search-result paths must be propagated to every batched read.

---

## [ERR-20260721-WPR] wpr-cpu-profile

**Logged**: 2026-07-21T13:18:00+08:00
**Priority**: medium
**Status**: wont_fix
**Area**: tests

### Summary

Windows Performance Recorder could not enable the system CPU profile in the current session.

### Error

`Failed to enable the policy to profile system performance (0xc5585011)`

### Context

- Attempted to capture the reproduced LocalSystem indexer startup spike with the built-in CPU profile.
- No ETL was produced; no profiler conclusion was inferred.

### Suggested Fix

Use boundary timing and the deterministic CPU harness in this session; run WPR later from a session holding the system profiling privilege if symbol-level attribution is still needed.

### Metadata

- Reproducible: yes
- Related Files: tests/performance/indexer-idle-cpu.ps1

### Resolution

- **Resolved**: 2026-07-21T13:18:00+08:00
- **Notes**: Not required for the first diagnosis because the real-service CPU regression is deterministic and the startup call chain can be timed at explicit boundaries.

---

## [ERR-20260721-IDXCPU] indexer-idle-cpu

**Logged**: 2026-07-21T13:14:00+08:00
**Priority**: high
**Status**: resolved
**Area**: tests

### Summary

The first idle-CPU probe could not read process times from the LocalSystem indexer.

### Error

`NxToolsProcessTimes.TotalSeconds: OpenProcess failed`

### Context

- The probe restarted `NxToolsIndexerV1` and attempted `PROCESS_QUERY_LIMITED_INFORMATION` from the current PowerShell session.
- Task Manager and Windows performance counters can still observe the process.

### Suggested Fix

Use the Windows Process performance counter instance selected by PID instead of opening the service process directly.

### Metadata

- Reproducible: yes
- Related Files: tests/performance/indexer-idle-cpu.ps1

### Resolution

- Replaced direct `OpenProcess` timing with `Get-Counter` process-instance selection by service PID.
- The probe now covers restart, handshake phase, normalized CPU, core-equivalent CPU and private working set, and fails above the configured average threshold.
- **Resolved**: 2026-07-21T13:45:00+08:00

---

## [ERR-20260720-032] parallel-indexer-process-lifecycle-release-flake

**Logged**: 2026-07-20T16:25:00+08:00
**Priority**: medium
**Status**: resolved
**Area**: tests

### Summary

The Release package gate ran two real `NxTools.Indexer.exe` lifecycle tests concurrently; one child
closed during handshake, while the same tests passed reliably when run serially.

### Resolution

- Serialize the two real-process lifecycle tests with a shared mutex.
- Keep unit tests parallel; only tests that launch the single-instance indexer are serialized.

---

## [ERR-20260720-031] search-boundary-fixture-used-excluding-default-context

**Logged**: 2026-07-20T16:12:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

The real search coordinator fixture used `IndexSearchContext::default()`, whose include flags are
both false, so the correctly executed replacement query returned an empty batch.

### Resolution

- Boundary fixtures that expect file results now explicitly enable files and directories.

---

## [ERR-20260720-030] concurrent-search-test-assumed-preemption-before-receipt

**Logged**: 2026-07-20T16:05:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

An equal-generation coordinator test incorrectly required the older query to emit no batch at all,
even though it may legally finish before the replacement request reaches the coordinator.

### Resolution

- Assert the observable invariant: once the replacement result arrives, an older result cannot
  become the final published batch.
- The test now waits for the replacement and verifies the last batch remains that result.

---

## [ERR-20260720-029] cargo-test-multiple-positional-filters

**Logged**: 2026-07-20T15:10:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

`cargo test` accepts only one positional test-name filter, so passing two test names was rejected
before compilation.

### Resolution

- Run the package test target once, or invoke each exact filter in a separate command.
- Prefer a shared substring only when it is unambiguous.

---

## [ERR-20260720-023] release-artifact-name-assumption

**Logged**: 2026-07-20T16:58:00+08:00
**Priority**: low
**Status**: resolved
**Area**: infra

### Summary

Release hash verification assumed the Cargo binary name instead of reading the packaged executable name.

### Error

```text
Cannot find path 'artifacts\win-x64\nxtools-desktop.exe' because it does not exist.
```

### Context

- The Release package completed successfully.
- `scripts/package.ps1` publishes the desktop binary as `artifacts\win-x64\NxTools.exe`.
- Only the optional hash lookup failed; the package itself was unaffected.

### Suggested Fix

List the artifact directory or read `scripts/package.ps1` before validating a packaged binary, then hash
the emitted `NxTools.exe` name.

### Metadata

- Reproducible: yes
- Related Files: scripts/package.ps1

### Resolution

- **Resolved**: 2026-07-20T16:59:00+08:00
- **Notes**: Re-ran SHA-256 verification against the actual `NxTools.exe` artifact.

---

## [ERR-20260720-022] windowsapps-cdb-direct-launch

**Logged**: 2026-07-20T13:44:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

The WinDbg Store package exposes `cdb.exe` under `WindowsApps`, but PowerShell cannot execute that
physical path directly because the package directory denies process launch.

### Error

```text
Program 'cdb.exe' failed to run: Access denied.
```

### Context

- Attempted to load the NxTools Release executable and PDB for a crash-offset lookup.
- The debugger binary was discovered under the versioned Microsoft.WinDbg Store package directory.

### Suggested Fix

Use the package execution alias when available, copy/use an approved debugger distribution outside
`WindowsApps`, or analyze the PDB with DIA/LLVM tooling instead of invoking the protected path.

### Metadata

- Reproducible: yes
- Related Files: target/release/nxtools_desktop.pdb

---

## [ERR-20260720-023] start-process-cdb-command-quoting

**Logged**: 2026-07-20T13:46:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

PowerShell `Start-Process -ArgumentList` did not retain quoting around the multi-token cdb `-c`
command, so cdb parsed `/f` as a process switch.

### Error

```text
cdb: Invalid switch 'f'
```

### Context

- The quick-launch smoke harness passes `.reload /f; ...` to cdb.
- A string array does not guarantee command-line quoting for an argument that contains spaces.

### Suggested Fix

Wrap the cdb command payload in explicit double quotes before passing it through `Start-Process`.

### Metadata

- Reproducible: yes
- Related Files: tests/diagnostics/quick-launch-popup-smoke.ps1

### Resolution

- **Resolved**: 2026-07-20T13:46:00+08:00
- **Notes**: Added explicit quotes around the cdb command payload.

---

## [ERR-20260720-024] smoke-test-matches-debugger-command-text

**Logged**: 2026-07-20T13:49:00+08:00
**Priority**: medium
**Status**: resolved
**Area**: tests

### Summary

The crash smoke test matched `c0000409` inside cdb's echoed command rather than an actual exception,
producing a false failure after the popup remained alive.

### Error

```text
Quick-launch popup crashed under the debugger.
```

### Context

- cdb echoes `sxe 0xc0000409` at startup.
- The first matcher searched for the bare exception code anywhere in output.

### Suggested Fix

Require cdb's actual `exception - code c0000409` line or a symbolized Rust panic frame.

### Metadata

- Reproducible: yes
- Related Files: tests/diagnostics/quick-launch-popup-smoke.ps1

### Resolution

- **Resolved**: 2026-07-20T13:49:00+08:00
- **Notes**: Tightened the crash-output regular expression.

---

## [ERR-20260720-NET] net8-hex-api

**Logged**: 2026-07-20T15:30:00+08:00
**Priority**: low
**Status**: resolved
**Area**: backend

### Summary
The PlatformHost targets .NET 8, which does not expose `Convert.ToHexStringLower`.

### Error
```text
CS0117: Convert does not contain a definition for ToHexStringLower
```

### Context
- Command: `dotnet build csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj`
- The API was used only to create a deterministic lowercase shell-icon cache key.

### Suggested Fix
Use `Convert.ToHexString(...).ToLowerInvariant()` for the .NET 8 target.

### Metadata
- Reproducible: yes
- Related Files: csharp/NxTools.PlatformHost/Windows/ShellIconExtractor.cs

### Resolution
- **Resolved**: 2026-07-20T15:30:00+08:00
- **Notes**: Replaced the newer API with the .NET 8 equivalent.

---

## [ERR-20260720-SQL] rusqlite-unsigned-read

**Logged**: 2026-07-20T15:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: backend

### Summary
`rusqlite` cannot decode a SQLite INTEGER column directly into `u64`.

### Error
```text
the trait bound `u64: FromSql` is not satisfied
```

### Context
- Command: `cargo test -p nxtools-finder`
- Quick-launch activation counts are non-negative SQLite INTEGER values.

### Suggested Fix
Read the column as `i64`, clamp corrupt negative values to zero, then convert to `u64`.

### Metadata
- Reproducible: yes
- Related Files: rust/crates/nxtools-finder/src/lib.rs

### Resolution
- **Resolved**: 2026-07-20T15:00:00+08:00
- **Notes**: Added the checked domain-boundary conversion.

---

## [ERR-20260720-FZY] finder-path-subsequence-noise

**Logged**: 2026-07-20T15:05:00+08:00
**Priority**: medium
**Status**: resolved
**Area**: backend

### Summary
Fuzzy subsequence matching over an entire executable path admitted unrelated application results.

### Error
```text
new query for "note" still contained Calculator because characters matched its parent path
```

### Context
- Command: `cargo test -p nxtools-finder`
- Application display names should be fuzzy; full paths should only supply lexical matches.

### Suggested Fix
Keep subsequence scoring for display names and restrict target paths to exact/prefix/substring hits.

### Metadata
- Reproducible: yes
- Related Files: rust/crates/nxtools-finder/src/lib.rs

### Resolution
- **Resolved**: 2026-07-20T15:05:00+08:00
- **Notes**: Split name fuzzy scoring from path lexical scoring.

---

## [ERR-20260720-NOT] clippy-notification-action-type-complexity

**Logged**: 2026-07-20T12:37:07+08:00
**Priority**: low
**Status**: resolved
**Area**: frontend

### Summary

The first notification action callback field tripped the workspace's warning-deny Clippy policy.

### Error

```text
clippy::type-complexity for Rc<dyn Fn(&mut Window, &mut App)>
```

### Context

- Command: `scripts/build.ps1 -Configuration Debug`
- The callback is intentionally UI-thread local, but its raw trait-object spelling exceeded the
  configured complexity threshold.

### Suggested Fix

Name GPUI callback trait-object types before storing them in reusable UI models.

### Metadata

- Reproducible: yes
- Related Files: rust/apps/nxtools-desktop/src/notifications.rs

### Resolution

- **Resolved**: 2026-07-20T12:37:07+08:00
- **Notes**: Introduced `NotificationActionHandler` and kept the callback ownership unchanged.

---

## [ERR-20260718-023] clippy-identical-selected-text-color

**Logged**: 2026-07-18T00:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: frontend

### Summary

The sidebar redesign left an obsolete selected-state conditional whose two text-color branches
were identical, and the warning-deny Release gate rejected it.

### Error

```text
clippy::if_same_then_else
```

### Context

- Command: `scripts/package.ps1 -Configuration Release`
- The selected state now uses the blue rail and icon instead of a different text color.

### Suggested Fix

Use the common text token directly when the selected state no longer changes that property.

### Metadata

- Reproducible: yes
- Related Files: rust/apps/nxtools-desktop/src/main_window.rs

### Resolution

- **Resolved**: 2026-07-18T00:00:00+08:00
- **Notes**: Replaced the conditional with `theme.text`.

---

## [ERR-20260718-022] release-package-timeout-too-short

**Logged**: 2026-07-18T00:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tooling

### Summary

The Release packaging command was initially invoked with a one-second process timeout, which is
too short for the workspace Clippy, test, Cargo build, and .NET publish pipeline.

### Error

```text
command timed out after 1625 milliseconds
```

### Context

- Command: `scripts/package.ps1 -Configuration Release`
- The timeout was an orchestration mistake, not a build failure.

### Suggested Fix

Run release packaging with a timeout sized for the full build pipeline.

### Metadata

- Reproducible: yes
- Related Files: scripts/package.ps1
- Recurrence-Count: 5
- Last-Seen: 2026-07-21

---

## [ERR-20260721-QLTHEME] unused-launcher-semantic-color

**Logged**: 2026-07-21T00:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: frontend

### Summary

The Release warning gate rejected an unused `element_hover` token added during the launcher visual pass.

### Error

`field element_hover is never read; -D dead-code implied by -D warnings`

### Context

- The launcher deliberately keeps one selection visual so pointer hover does not compete with keyboard selection.
- The token was copied from Zed's semantic palette but had no current renderer consumer.

### Suggested Fix

Add only semantic roles used by the current vertical slice; introduce hover when a distinct hover state is specified.

### Metadata

- Reproducible: yes
- Related Files: rust/apps/nxtools-desktop/src/theme.rs

### Resolution

- Removed the unused role instead of suppressing the warning.

---

## [ERR-20260721-PATCHCTX] apply-patch-stale-function-context

**Logged**: 2026-07-21T00:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: backend

### Summary

A cleanup patch used an approximate body for a long function and failed its context check.

### Error

`apply_patch verification failed: Failed to find expected lines`

### Context

- The function had a `filter(...).map(...)` chain rather than the remembered `is_some_and(...)` form.
- No file was partially modified.

### Suggested Fix

Read the exact current function range before removing a long block.

### Metadata

- Reproducible: yes
- Related Files: rust/apps/nxtools-indexer/src/lib.rs

### Resolution

- Recreated the deletion from the exact source text.

### Resolution

- **Resolved**: 2026-07-18T00:00:00+08:00
- **Notes**: Re-ran with a full Release-build timeout.

---

## [ERR-20260718-003] git-sparse-checkout-file-argument

**Logged**: 2026-07-18T15:40:00+08:00
**Priority**: low
**Status**: resolved
**Area**: infra

### Summary

`git sparse-checkout set` in cone mode rejected a single file path mixed with a directory.

### Error

```text
fatal: 'codex-rs/Cargo.lock' is not a directory; to treat it as a directory anyway, rerun with --skip-checks
```

### Context

- Reference audit of the OpenAI Codex TUI at tag `rust-v0.144.5`.
- The command mixed `codex-rs/tui` and `codex-rs/Cargo.lock` in cone mode.

### Suggested Fix

Sparse-checkout only the required directory, or use non-cone mode when individual files are required.

### Metadata

- Reproducible: yes
- Related Files: none

### Resolution

- **Resolved**: 2026-07-18T15:40:00+08:00
- **Notes**: Continued with `git sparse-checkout set codex-rs/tui` only.

---

## [ERR-20260718-015] regression-test-unit-binding

**Logged**: 2026-07-18T14:25:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

Clippy rejected an unnecessary `let _ =` around the unit-returning GPUI regression-test update.

### Resolution

Removed the unit binding and reran the full static-analysis gate.

### Metadata

- Reproducible: yes
- Related Files: rust/apps/nxtools-desktop/src/quick_launch.rs
- Recurrence-Count: 2
- Last-Seen: 2026-07-20

### Recurrence

The same lint recurred in a production notification update added after the earlier targeted Clippy
run. Run the affected-package Clippy command again after final review fixes, before packaging.

---

## [ERR-20260718-014] gpui-test-context-read-api

**Logged**: 2026-07-18T14:08:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

The GPUI regression test passed `TestAppContext` directly to `Entity::read`, which requires an `App` reference.

### Resolution

Read the entity inside `TestAppContext::read`, which exposes the required immutable `App` context.

---

## [ERR-20260718-013] gpui-test-support-not-enabled

**Logged**: 2026-07-18T14:05:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

The first GPUI entity-reentrancy regression test could not import `TestAppContext` because GPUI gates its test harness behind the `test-support` feature.

### Resolution

Enabled GPUI `test-support` only as a desktop crate dev-dependency, keeping the production dependency configuration unchanged.

---

## [ERR-20260718-012] cdb-symbol-server-stall

**Logged**: 2026-07-18T14:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tooling

### Summary

The first CDB dump-analysis command stalled while contacting Microsoft's symbol server.

### Resolution

Terminated the stalled debugger and loaded the matching local Rust PDB from `target/release`, which produced the complete application call stack immediately.

---

## [ERR-20260718-011] preview-reuse-style-checks

**Logged**: 2026-07-18T13:50:00+08:00
**Priority**: low
**Status**: resolved
**Area**: code

### Summary

The initial preview-reuse patch needed rustfmt normalization and used two nested conditionals rejected by Clippy's `collapsible_if` lint.

### Resolution

Applied rustfmt and collapsed the conditional error handling before rerunning the full quality gate.

---

## [ERR-20260718-010] moved-preview-item

**Logged**: 2026-07-18T13:45:00+08:00
**Priority**: low
**Status**: resolved
**Area**: code

### Summary

The first preview-reuse implementation moved a clipboard item into the reuse closure and then tried to retain it for the stale-window creation fallback.

### Resolution

Clone the item only for the reuse closure, preserving the original for the create fallback. The focused preview lifecycle regression test now compiles and passes.

---

## [ERR-20260718-009] powershell-reserved-host-variable

**Logged**: 2026-07-18T13:20:00+08:00
**Last-Seen**: 2026-08-08
**Pattern-Key**: tooling.powershell_reserved_automatic_variable
**Recurrence-Count**: 2
**Priority**: low
**Status**: resolved
**Area**: tooling

### Summary

A log-inspection command, and later a browser architecture gate, assigned to PowerShell's reserved,
read-only `$Host` automatic variable.

### Resolution

Use descriptive non-reserved names such as `$hostLogFile`; avoid `$Host`, `$Error`, `$PID`, and other PowerShell automatic variables in diagnostic scripts.

---

## [ERR-20260718-008] wrong-project-root

**Logged**: 2026-07-18T15:05:00+08:00
**Priority**: medium
**Status**: resolved
**Area**: tooling

### Summary

The first status/search command used the environment-provided old `NxTools` root even though the active rewrite lives in the sibling `NxTools.Next` repository.

### Resolution

Stopped immediately after the read-only status output, preserved all old-project user changes, and pinned every subsequent command and patch to `D:\RiderProject\NxTools.Next`. For this thread, verify the Git root before any repository action.

---

## [ERR-20260718-005] powershell-source-search-and-svg-fetch

**Logged**: 2026-07-18T12:05:00+08:00
**Last Seen**: 2026-07-18T16:39:00+08:00
**Occurrences**: 5
**Priority**: low
**Status**: resolved
**Area**: infra

### Summary

Two diagnostic commands used an invalid wildcard directory and an invalid PowerShell `foreach` pipeline; the raw SVG web reader also rejected valid SVG responses.

### Error

```text
The filename, directory name, or volume label syntax is incorrect.
An empty pipe element is not allowed.
Failed to fetch raw.githubusercontent.com ... (400) OK.
```

### Context

- The failures occurred while locating GPUI asset-source code and reading official Material Symbol SVG files.
- No project source or asset was written by the failed commands.

### Suggested Fix

Enumerate wildcard directories before passing paths to `rg`, accumulate `foreach` results before piping, and use a strict single-request fallback for raw SVG responses.

### Metadata

- Reproducible: yes
- Related Files: rust/apps/nxtools-desktop/src/assets.rs

### Resolution

- **Resolved**: 2026-07-18T12:05:00+08:00
- **Notes**: Switched to enumerated paths and successful strict requests, then embedded the verified SVG contents with `apply_patch`.

The same wildcard-path assumption recurred while inspecting Cargo manifests and registry sources. Always enumerate the exact path first with `Get-ChildItem` or `rg --files`; do not pass PowerShell-style `**` or wildcard directory components to Windows `rg`.

It recurred once more when `rg` received `csharp/.../Windows/*.cs` literally. Pass the directory
plus `-g '*.cs'` instead of a wildcard path on Windows.

---

## [ERR-20260718-007] image-signature-validation

**Logged**: 2026-07-18T15:30:00+08:00
**Priority**: medium
**Status**: resolved
**Area**: backend

### Summary

The first QQ image-promotion test let GDI+ inspect every single file path directly; invalid or extensionless input can surface as `OutOfMemoryException` rather than a format-specific exception.

### Resolution

Added cheap extension/header signature checks, loaded through a shared-read stream, and treated GDI+'s invalid-image `OutOfMemoryException` as a rejected candidate. The regression test now covers an image whose extension cannot be trusted.

---

---

## [ERR-20260718-006] clipboard-history-row-argument-count

**Logged**: 2026-07-18T12:20:00+08:00
**Priority**: low
**Status**: resolved
**Area**: frontend

### Summary

Adding category-menu discoverability temporarily pushed `history_row` beyond the workspace's Clippy argument-count limit.

### Error

```text
this function has too many arguments (8/7)
```

### Context

- Command: `cargo clippy --workspace --all-targets -- -D warnings`
- Category options and the unavailable hint were passed as two independent arguments.

### Suggested Fix

Model related category-menu state as one typed value instead of expanding the rendering function signature.

### Metadata

- Reproducible: yes
- Related Files: rust/apps/nxtools-desktop/src/ui/history.rs

### Resolution

- **Resolved**: 2026-07-18T12:20:00+08:00
- **Notes**: Introduced `CategorySourceMenu` to group options and unavailable state without suppressing Clippy.

---

## [ERR-20260718-004] release-package-locked-destination

**Logged**: 2026-07-18T11:10:00+08:00
**Priority**: low
**Status**: promoted
**Area**: infra

### Summary

Release packaging could not overwrite the default artifact because that packaged executable was still running.

### Error

```text
The process cannot access artifacts\win-x64\NxTools.exe because it is being used by another process.
```

### Context

- Command: `scripts/package.ps1 -Configuration Release`
- The Release build and tests completed before `Copy-Item` failed.
- The running process belonged to the default package directory and was not terminated.

### Suggested Fix

Use the package script's `-OutputPath` option to create a side-by-side package when the default artifact is locked.

### Metadata

- Reproducible: yes
- Related Files: scripts/package.ps1

### Resolution

- **Resolved**: 2026-07-18T11:10:00+08:00
- **Notes**: Generated the package in a new versioned output directory without interrupting the user's running app.
- **Recurrence 2026-07-21**: `NxToolsIndexerV1` kept `artifacts\win-x64\NxTools.Indexer.exe`
  locked. Before overwriting the default package, stop the packaged Windows service and restart it
  after the copy; otherwise use a side-by-side `-OutputPath`.
- **Recurrence 2026-07-22**: the packaged desktop itself was still running after the Release gate;
  with explicit permission to close it, stop only the process whose executable path exactly equals
  the destination artifact before retrying the package.
- **Recurrence-Count**: 3
- **Promoted**: AGENTS.md

---

## [ERR-20260718-003] clipboard-category-regression-test

**Logged**: 2026-07-18T11:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

A regression test assumed `ClipboardHistory::open_in_memory`, but this project only exposes `ClipboardHistory::open`.

### Error

```text
no associated function or constant named `open_in_memory` found for struct `ClipboardHistory`
```

### Context

- Command: `cargo test -p nxtools-clipboard invalid_category_kind_is_rejected_instead_of_becoming_all`
- The unverified helper assumption prevented the new corruption regression test from compiling.

### Suggested Fix

Use the same `tempfile::tempdir` plus file-backed SQLite setup as the crate's existing tests, and verify available test helpers before calling them.

### Metadata

- Reproducible: yes
- Related Files: rust/crates/nxtools-clipboard/src/lib.rs

### Resolution

- **Resolved**: 2026-07-18T11:00:00+08:00
- **Notes**: Replaced the nonexistent helper with the established temporary database fixture.

---

## [ERR-20260718-003] cargo-test-multiple-positional-filters

**Logged**: 2026-07-18T10:40:00+08:00
**Last Seen**: 2026-07-18T13:25:00+08:00
**Occurrences**: 2
**Priority**: low
**Status**: resolved
**Area**: tooling

### Summary

`cargo test` accepts only one positional test-name filter; passing two test names made that step fail while a later PowerShell command masked the aggregate exit code.

### Resolution

Use one shared module/name substring or run focused tests as separate commands, and do not infer a chained native-command result only from the final process exit code.

The mistake recurred while validating two clipboard crash regression tests. Run each focused test as its own `cargo test` invocation.

---

## [ERR-20260718-002] stale-platform-protocol-file-names

**Logged**: 2026-07-18T10:05:00+08:00
**Priority**: low
**Status**: resolved
**Area**: backend

### Summary

Repository inspection referenced guessed protocol file names that do not exist.

### Error

```text
Cannot find path '...Protocol\PlatformCommandProcessor.cs'
Cannot find path '...Protocol\PlatformCommand.cs'
```

### Context

- The actual command parsing and dispatch live in `PlatformCommandParser.cs` and `PlatformPipeClient.cs`.

### Suggested Fix

Run `rg --files` for the target directory before opening architecture-sensitive files.

### Metadata

- Reproducible: yes
- Related Files: csharp/NxTools.PlatformHost/Protocol

### Resolution

- **Resolved**: 2026-07-18T10:06:00+08:00
- **Notes**: Located the real parser and dispatcher from the repository file list.

---

## [ERR-20260718-001] product-design-user-context-preflight-path

**Logged**: 2026-07-18T09:35:00+08:00
**Last Seen**: 2026-07-27T00:00:00+08:00
**Occurrences**: 3
**Priority**: low
**Status**: resolved
**Area**: config

### Summary

The Product Design user-context preflight script was resolved from the plugin root instead of the skill-local scripts directory.

### Error

```text
can't open file '...product-design\\0.1.50\\scripts\\user_context_preflight.py': No such file or directory
```

### Context

- The script referenced by `skills/user-context/SKILL.md` lives under `skills/user-context/scripts/`.

### Suggested Fix

Resolve relative script references against the directory containing the selected `SKILL.md`.

### Metadata

- Reproducible: yes
- Related Files: product-design/skills/user-context/SKILL.md

### Resolution

- **Resolved**: 2026-07-18T09:36:00+08:00
- **Notes**: Re-ran the preflight from the skill-local scripts directory successfully.

The same path error recurred during the clipboard settings visual audit. Always resolve Product
Design scripts relative to the selected focused skill directory, not the plugin package root.
It recurred again with Product Design `0.1.52`; the correct executable remains
`skills/user-context/scripts/user_context_preflight.py`.

---

## [ERR-20260717-002] debug-package-missing-binaries

**Logged**: 2026-07-17T18:10:00+08:00
**Priority**: medium
**Status**: resolved
**Area**: infra

### Summary

The Debug package script copied top-level Rust executables after tests without explicitly building binary targets.

### Error

```text
Cannot find path 'target\debug\nxtools-indexer.exe' because it does not exist.
```

### Context

- Command: `scripts/package.ps1 -Configuration Debug`
- `cargo test` produced hashed test executables but not `target\debug\nxtools-indexer.exe`.

### Suggested Fix

Run `cargo build -p nxtools-desktop -p nxtools-indexer` for the selected profile before copying package files.

### Metadata

- Reproducible: yes
- Related Files: scripts/package.ps1

### Resolution

- **Resolved**: 2026-07-17T18:10:00+08:00
- **Notes**: The packaging script now explicitly builds both Rust binary targets before publishing and copying artifacts.

---

## [ERR-20260717-001] cargo-release-build

**Logged**: 2026-07-17T15:43:00+08:00
**Last Seen**: 2026-07-27T15:30:00+08:00
**Occurrences**: 3
**Priority**: high
**Status**: resolved
**Area**: infra

### Summary

GPUI 0.2.2 Debug builds succeed without `fxc.exe`, but non-debug Windows builds require it.

### Error

```text
Failed to find fxc.exe
```

### Context

- Command: `cargo build --release -p nxtools-desktop -p nxtools-indexer`
- Environment: Windows 10 with Rust 1.96; Windows SDK was rooted at `D:\Windows Kits\10` while the preflight only checked C:.
- GPUI's build script only compiles HLSL ahead of time when debug assertions are disabled.

### Suggested Fix

Install the Windows 10/11 SDK shader tools or set `GPUI_FXC_PATH`. Keep the build-script
preflight so missing toolchains fail before the expensive Release compilation begins.

### Metadata

- Reproducible: yes
- Related Files: scripts/build.ps1, docs/building.md

### Resolution

- **Resolved**: 2026-07-18T10:45:00+08:00
- **Notes**: Installed the Microsoft Windows SDK package and changed `Find-Fxc` to discover `KitsRoot10` through both Windows SDK registry views. The preflight now finds SDK tools regardless of drive.
  A later split Release build bypassed that preflight; setting `GPUI_FXC_PATH` to the discovered
  SDK executable before standalone Cargo commands remains required.

---
## [ERR-20260718-004] standalone-diagnostic-inherited-parent-workspace

**Logged**: 2026-07-18T00:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tooling

### Summary
A temporary Cargo diagnostic nested under the repository inherited the parent workspace and could not run.

### Error
```
current package believes it's in a workspace when it's not
```

### Context
- Command: `cargo run --quiet --manifest-path .diagnostics\\clipboard-arboard-probe\\Cargo.toml`
- The probe is intentionally not a product workspace member.

### Suggested Fix
Add an empty `[workspace]` table to standalone diagnostic manifests nested beneath the repository.

### Resolution
Added `[workspace]` to the probe manifest before rerunning it.

---
## [ERR-20260718-005] clipboard-win-register-format-return-type

**Logged**: 2026-07-18T00:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tooling

### Summary
The temporary Rust probe treated `clipboard_win::register_format` as a `Result`; version 5.4.1 returns an `Option`.

### Error
```
no method named `ok` found for enum `Option<T>`
```

### Resolution
Removed `.ok()` and mapped the returned `Option` directly.

---

## [ERR-20260718-016] retention-purge-test-error-variant

**Logged**: 2026-07-18T00:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

A physical-purge regression test expected the repository's update-oriented `ItemNotFound` error,
but the private row reader correctly surfaced SQLite `QueryReturnedNoRows` after deletion.

### Error

```text
retention_physically_purges_old_deleted_rows_and_reports_image_files: assertion failed
```

### Context

- Command: `cargo test -p nxtools-clipboard`
- The behavior under test was physical deletion, not the public error mapping of an update method.

### Suggested Fix

Verify physical deletion directly with `SELECT COUNT(*)` and keep public behavior assertions scoped
to public repository methods.

### Metadata

- Reproducible: yes
- Related Files: rust/crates/nxtools-clipboard/src/lib.rs

### Resolution

- **Resolved**: 2026-07-18T00:00:00+08:00
- **Notes**: Replaced the error-variant assertion with a direct database row-count assertion.

---

## [ERR-20260718-017] gpui-keydown-test-version-mismatch

**Logged**: 2026-07-18T00:00:00+08:00
**Priority**: medium
**Status**: resolved
**Area**: tests

### Summary

A hotkey unit test copied a `KeyDownEvent` field from a newer Zed/GPUI revision that is absent in
the project's pinned GPUI 0.2.2.

### Error

```text
struct KeyDownEvent has no field named prefer_character_input
```

### Context

- Command: `cargo test --workspace`
- `cargo check -p nxtools-desktop` did not compile the `#[cfg(test)]` module.

### Suggested Fix

Construct events against the pinned crate's local type definition and always compile the full test
target after adding unit tests.

### Metadata

- Reproducible: yes
- Related Files: rust/apps/nxtools-desktop/src/main_window.rs

### Resolution

- **Resolved**: 2026-07-18T00:00:00+08:00
- **Notes**: Removed the unsupported field and retained the two fields defined by GPUI 0.2.2.

---

## [ERR-20260718-018] hotkey-byte-display-formatted-as-number

**Logged**: 2026-07-18T00:00:00+08:00
**Priority**: medium
**Status**: resolved
**Area**: frontend

### Summary

Recorded alphanumeric hotkeys displayed their ASCII byte values (`86`) instead of key labels (`V`).

### Error

```text
assertion failed: left "Ctrl+Shift+86", right "Ctrl+Shift+V"
```

### Context

- Command: `cargo test --workspace`
- `u8::to_string()` formats the numeric byte; it does not interpret the byte as a character.

### Suggested Fix

Convert validated ASCII bytes to `char` before formatting display labels.

### Metadata

- Reproducible: yes
- Related Files: rust/apps/nxtools-desktop/src/main_window.rs

### Resolution

- **Resolved**: 2026-07-18T00:00:00+08:00
- **Notes**: Changed the display conversion to `char::from(upper_ascii_byte).to_string()`.

---

## [ERR-20260718-019] release-command-timeout-too-short

**Logged**: 2026-07-18T00:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: infra

### Summary

The first Release package command used a one-second shell timeout and was terminated before the
build could yield progress.

### Error

```text
command timed out after 1454 milliseconds
```

### Context

- Command: `scripts/package.ps1 -Configuration Release`
- Release packaging runs Clippy, all tests, Rust builds and .NET publish.

### Suggested Fix

Give build scripts a real process timeout and use the orchestration layer's yield/wait mechanism for
intermediate updates.

### Metadata

- Reproducible: yes
- Related Files: scripts/package.ps1

### Resolution

- **Resolved**: 2026-07-18T00:00:00+08:00
- **Notes**: Reran with a ten-minute command timeout.

---

## [ERR-20260718-020] clippy-items-after-test-module

**Logged**: 2026-07-18T00:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

The new hotkey test module was inserted before existing production helper functions, violating the
workspace's warning-deny Clippy policy.

### Error

```text
clippy::items-after-test-module
```

### Context

- Command: `scripts/package.ps1 -Configuration Release`
- The repository treats all Clippy warnings as errors.

### Suggested Fix

Keep file-local test modules at the end of their Rust source file.

### Metadata

- Reproducible: yes
- Related Files: rust/apps/nxtools-desktop/src/main_window.rs

### Resolution

- **Resolved**: 2026-07-18T00:00:00+08:00
- **Notes**: Moved the test module after all production helpers.

---

## [ERR-20260718-021] clippy-nonminimal-pending-check

**Logged**: 2026-07-18T00:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: backend

### Summary

The pending-hotkey correlation guard used a negated `is_some_and`, which Clippy rejected under the
workspace warning-deny policy.

### Error

```text
clippy::nonminimal-bool
```

### Context

- Command: `scripts/package.ps1 -Configuration Release`
- The logic was correct but not in the canonical Option predicate form.

### Suggested Fix

Use `Option::is_none_or` for negative correlation guards.

### Metadata

- Reproducible: yes
- Related Files: rust/apps/nxtools-desktop/src/clipboard.rs

### Resolution

- **Resolved**: 2026-07-18T00:00:00+08:00
- **Notes**: Replaced the negated predicate with `is_none_or`.

---
## [ERR-20260720-ARG] cargo-test-filter

**Logged**: 2026-07-20T09:49:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary
Cargo accepts one positional test-name filter, not two.

### Error
```text
error: unexpected argument 'preview_appearance_is_short_generation_scoped_and_never_repeats' found
```

### Context
- Attempted to run two named Rust tests in one `cargo test` invocation.

### Suggested Fix
Run each filter separately or use one shared substring.

### Metadata
- Reproducible: yes
- Related Files: rust/apps/nxtools-desktop/src/ui/mod.rs, rust/apps/nxtools-desktop/src/ui/preview.rs

### Resolution
- **Resolved**: 2026-07-20T09:49:00+08:00
- **Notes**: Re-ran both tests with separate filters.

---
## [ERR-20260720-026] recursive-diagnostic-cleanup-blocked

**Logged**: 2026-07-20T14:10:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tooling

### Summary

The command safety policy rejected PowerShell `Remove-Item` cleanup, including an explicitly
workspace-bounded diagnostic directory.

### Resolution

- Deleted the two known generated log files through `apply_patch` instead.
- Avoid recursive or enumerated deletion commands for diagnostic cleanup.

### Metadata

- Reproducible: yes
- Recurrence-Count: 2
- Last-Seen: 2026-07-27

---

## [ERR-20260720-025] powershell-rg-windows-glob

**Logged**: 2026-07-20T14:02:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tooling

### Summary

An `rg` command used a Unix-style `src/*.rs` path under PowerShell, causing one branch of a batched
read-only inspection to fail.

### Resolution

- Re-ran the inspection with explicit file paths.
- Use `rg ... src -g "*.rs"` rather than a shell path glob on Windows.

---
## [ERR-20260720-028] parallel-single-instance-crash-harness

**Logged**: 2026-07-20T14:08:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

Two quick-launch crash-harness runs were started in parallel even though the harness intentionally
requires an exclusive NxTools instance. One run was rejected and the other process needed cleanup.

### Resolution

- Run single-instance desktop interaction harnesses serially.
- The harness guard remains exact-process based so NxToolsLite does not count as NxTools.

---

## [ERR-20260720-027] swiftlist-shell-icon-helper-path-assumption

**Logged**: 2026-07-20T14:12:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tooling

### Summary

The SwiftList icon helper was assumed to live under `App/Helpers`; the actual source is under
`App/Services`.

### Resolution

- Located the file with `rg --files` before reading it.
- Do not infer reference-project paths from type names.

---
### ERR-20260720-033: cargo test accepts only one positional filter

- Command: `cargo test -p nxtools-indexer test_a test_b`
- Failure: Cargo rejected the second test name as an unexpected argument, while a later command in
  the same PowerShell script masked the aggregate exit code.
- Prevention: run each filtered test as a separate `cargo test` command, or use one shared substring;
  check native success after every command in multi-step verification scripts.
- Recurrence-Count: 2
- Last-Seen: 2026-07-21
## 2026-07-21: strict Clippy caught overwritten compaction accumulator

- Symptom: `cargo clippy -p nxtools-indexer --all-targets -- -D warnings` rejected an initially
  empty `compaction_mounts` vector that was always replaced by the unified post-poll scan.
- Fix: declare the vector only at the unified scan site; keep strict Clippy in the phase release
  gate so state-machine cleanup cannot leave dead assignments.

---

## [ERR-20260722-034] rust-1.96-single-range-vec-lint

**Logged**: 2026-07-22T00:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

Strict Clippy rejected a single `Range<usize>` created with `vec![range]` under Rust 1.96.

### Error

`clippy::single-range-in-vec-init` is denied by the workspace `-D warnings` release check.

### Context

- Command: `cargo clippy -p nxtools-finder -p nxtools-desktop --all-targets -- -D warnings`
- Related file: `rust/apps/nxtools-desktop/src/quick_launch.rs`

### Suggested Fix

Construct the one-element range collection with `std::iter::once(range).collect()` and rerun the
strict check.

### Metadata

- Reproducible: yes

### Resolution

- **Resolved**: 2026-07-22T00:00:00+08:00
- **Notes**: Replaced the single-range `vec![]` construction with
  `std::iter::once(range).collect()`; the strict Clippy command now passes.

---

## [ERR-20260722-035] release-package-command-timeout

**Logged**: 2026-07-22T09:22:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests
**Recurrence-Count**: 2
**Last-Seen**: 2026-07-22

### Summary

The first Release package invocation inherited a 10-second shell timeout and was terminated while
the package gate was still compiling.

### Resolution

- Re-ran `scripts/package.ps1 -Configuration Release` with a 10-minute command budget.
- Long-running package scripts must set their shell timeout independently from the UI yield interval.
- The stage-3 action slice repeated this mistake with a one-second shell timeout; the corrected
  invocation again uses a ten-minute process budget.

---

## [ERR-20260722-036] packaged-indexer-locked-by-service

**Logged**: 2026-07-22T09:23:00+08:00
**Priority**: low
**Status**: resolved
**Area**: infra

### Summary

The Release gate passed, but the final copy could not replace `artifacts/win-x64/NxTools.Indexer.exe`
because the installed `NxToolsIndexerV1` service runs that exact artifact.

### Resolution

- Verified the service image path, stopped only `NxToolsIndexerV1`, completed the exact artifact
  copies, and restored the service to `Running`/`Automatic`.
- SHA-256 comparison confirmed the packaged desktop and indexer binaries match the Release targets.

---

## [ERR-20260722-037] usn-journal-entry-deleted-stuck-partial

**Logged**: 2026-07-22T09:40:00+08:00
**Priority**: high
**Status**: resolved
**Area**: backend

### Summary

Windows USN read error 1181 (`ERROR_JOURNAL_ENTRY_DELETED`) was recorded as a generic polling
failure, leaving the main window in `PARTIAL` instead of scheduling the existing scoped rebuild.

### Resolution

- Classified Win32 1181 as `JournalPoll::RebuildRequired`.
- The packaged LocalSystem service rebuilt only the affected volume, returned to `ready` with
  2,793,802 items and an empty `last_error`, while the real search smoke test returned six paths.

---

## [ERR-20260722-038] index-settings-task-tooling-failures

**Logged**: 2026-07-22T10:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

One multi-file patch assumed a nonexistent protocol command list, and one source-inspection test
matched its own assertion string. A read of the LocalSystem profile log path was also denied.

### Resolution

- Split documentation and code patches at verified anchors.
- Restricted the source assertion to production text before `#[cfg(test)]`.
- Used the authenticated indexer pipe status response instead of reading the protected service
  profile; it reported `last_error = null`.

---

## [ERR-20260722-040] github-contents-array-projection

**Logged**: 2026-07-22T11:20:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tooling

### Summary

PowerShell failed to expand `download_url` directly from a GitHub Contents API array response while
locating the official Material Symbols `content_copy` asset.

### Resolution

- Inspected the response as JSON, selected the exact 24 px rounded asset, and fetched its raw
  contents read-only before adding it with `apply_patch`.
- For GitHub Contents arrays, inspect or explicitly enumerate the returned objects before property
  expansion.

---

## [ERR-20260722-041] powershell-foreach-pipeline-parser

**Logged**: 2026-07-22T11:35:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tooling

### Summary

PowerShell rejected piping a statement-level `foreach` block directly into `Format-Table` while
verifying packaged binary hashes.

### Resolution

- Assigned the `foreach` output to `$rows` first, then piped `$rows` to the formatter.
- The corrected verification confirmed all three formal artifacts exactly match the staging hashes.

### Metadata

- Reproducible: yes
- Pattern-Key: powershell.foreach_pipeline_parser
- Recurrence-Count: 6
- First-Seen: 2026-07-22
- Last-Seen: 2026-08-08
- Promoted: AGENTS.md

The pattern recurred during the Phase 4 browser-extension reference inventory despite already being
promoted to `AGENTS.md`; no source was changed by the failed read-only command. The corrected command
again assigns the loop output to `$rows` before formatting.

---

## [ERR-20260722-042] parallel-reference-read-javascript-syntax

**Logged**: 2026-07-22T12:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tooling

### Summary

A JavaScript orchestration cell for four parallel reference reads had mismatched parentheses and
failed before invoking any shell command.

### Resolution

- Split the large orchestration cell into smaller read-only batches and use `String.raw` for Windows
  paths where needed.
- No command ran and no workspace state changed.

---

## [ERR-20260722-043] release-package-tool-timeout

**Logged**: 2026-07-22T12:35:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tooling

### Summary

The complete Release package script exceeded a 120-second shell-tool timeout and was terminated
before copying fresh artifacts, even though the preceding focused tests and Clippy checks passed.

### Resolution

- Verified no Cargo, Rust compiler, or .NET child process remained and that the artifact timestamp
  was still the previous package.
- Re-run the package command with a timeout sized for the full workspace Release gate, rather than
  treating the tool timeout as a compilation failure.

---

## [ERR-20260722-044] package-indexer-service-file-lock

**Logged**: 2026-07-22T12:42:00+08:00
**Priority**: medium
**Status**: resolved
**Area**: infra

### Summary

The Release gate passed, but `scripts/package.ps1` could not replace the formal
`NxTools.Indexer.exe` because the installed `NxToolsIndexerV1` service was running that file.

### Suggested Fix

Stop the known NxTools indexer service before replacing its packaged binary, copy the already
verified Release artifacts, restart the same service, and then teach the package workflow to handle
this lifecycle explicitly or publish atomically to a versioned staging directory.

The second occurrence showed that SCM can report `Stopped` before the old service process has
released its executable image. Waiting on the captured LocalSystem PID was then rejected by Windows
when packaging from a normal desktop process. The package workflow now waits for SCM `Stopped` and
uses a 15-second bounded file-copy retry, which observes the actual resource needed without opening
a privileged process handle.

### Metadata

- Reproducible: yes
- Related Files: scripts/package.ps1
- Pattern-Key: package.windows_service_binary_lock
- Recurrence-Count: 3

### Resolution

- Stopped only the installed service whose executable path exactly matched the formal package
  destination, replaced all three verified Release binaries, and restored the automatic service.
- Updated the package script to perform the same bounded stop/copy/restart lifecycle and to restart
  the service from `finally` if a later copy fails.

---

## [ERR-20260722-045] multi-hunk-struct-patch-anchor

**Logged**: 2026-07-22T12:48:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tooling

### Summary

One combined patch for the Shell query generation guard failed to match a struct field even though
the field existed; the patch mixed two distant struct anchors in one operation.

### Resolution

- Re-read the focused source range and applied the field/type change with a smaller verified patch.
- No source mutation occurred from the failed patch.

### Metadata

- Reproducible: unknown
- Related Files: rust/apps/nxtools-desktop/src/quick_launch.rs
- See Also: ERR-20260722-038

---

## [ERR-20260722-046] chained-native-command-masked-format-failure

**Logged**: 2026-07-22T12:55:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tooling

### Summary

A PowerShell verification command ran several native tools sequentially. `cargo fmt --check`
reported a formatting diff, but a later successful Release build made the overall shell invocation
exit successfully.

### Resolution

- Applied `cargo fmt --all`, then ran each remaining release gate as its own checked command.
- Do not infer all chained native tools passed from the last process exit code.

---

## [ERR-20260722-047] virtual-list-gate-still-expected-wheel-callback

**Logged**: 2026-07-22T15:20:00+08:00
**Priority**: medium
**Status**: resolved
**Area**: tests

### Summary

The Release package gate still required the retired `on_scroll_wheel ->
request_visible_result_icons` chain after quick-launch results moved to a GPUI virtual list whose
rendered range covers wheel, scrollbar and keyboard scrolling.

### Error

`tests/architecture/quick-launch-lifecycle.ps1` rejected the new presentation-driven icon path.

### Suggested Fix

When an interaction ownership boundary changes, update its architecture gate in the same slice.
Assert the new `uniform_list -> visible_range -> request_presented_result_icons` chain rather than
the old input-device-specific callback.

### Metadata

- Reproducible: yes
- Related Files: tests/architecture/quick-launch-lifecycle.ps1, rust/apps/nxtools-desktop/src/quick_launch.rs

---

## [ERR-20260722-048] idle-gate-sampled-active-index-migration

**Logged**: 2026-07-22T15:35:00+08:00
**Priority**: high
**Status**: resolved
**Area**: tests

### Summary

The indexer idle-CPU release gate began sampling immediately after the service reached the Windows
`Running` state, even though the index protocol was still `volumes_ready` with zero indexed items
during a one-time v1-to-v2 cache rebuild.

### Error

Active rebuild CPU was reported as an idle regression (2.46% normalized average).

### Suggested Fix

After restart, wait on index status events until the protocol phase is `ready` (with a bounded
timeout) before warmup and performance-counter sampling.

### Metadata

- Reproducible: yes
- Related Files: tests/performance/indexer-idle-cpu.ps1, rust/apps/nxtools-indexer/src/mft/cache.rs

---

## [ERR-20260722-049] rayon-fold-borrowed-snapshot-index

**Logged**: 2026-07-22T16:10:00+08:00
**Priority**: low
**Status**: resolved
**Area**: code

### Summary

The lightweight name-search candidate stored the outer snapshot index, but the nested Rayon fold
closure borrowed that index even though the parallel iterator can outlive the outer closure frame.

### Error

Rust `E0373`: the fold closure may outlive the current function while borrowing
`snapshot_index`.

### Resolution

Capture the copyable snapshot index by value in the fold closure and explicitly borrow the shared
cancellation flag before creating that closure.

### Metadata

- Reproducible: yes
- Related Files: rust/apps/nxtools-indexer/src/lib.rs

---

## [ERR-20260722-050] release-workspace-test-exhausted-host-memory

**Logged**: 2026-07-22T16:18:00+08:00
**Priority**: medium
**Status**: resolved
**Area**: tooling

### Summary

The Release package pipeline ran workspace Clippy and workspace tests in parallel while the host
had about 3 GiB of 16 GiB RAM free. Concurrent optimized `nxtools-indexer` compilations exhausted
physical/commit memory before packaging.

### Error

`rustc-LLVM ERROR: out of memory` while linking release library and test targets.

### Suggested Fix

Retry the verified pipeline with `CARGO_BUILD_JOBS=1` and, when its registered executable exactly
matches the package under test, stop the packaged LocalSystem indexer before compiling and restart
it afterward.

### Resolution

The complete Release pipeline passed with `CARGO_BUILD_JOBS=1`; the exact packaged service was
stopped only for the build/copy window and restored afterward.

### Metadata

- Reproducible: environment-dependent
- Related Files: scripts/build.ps1, scripts/package.ps1

---

## [ERR-20260722-051] stopped-service-image-handle-lingered

**Logged**: 2026-07-22T17:05:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tooling

### Summary

SCM reported the packaged indexer service as stopped, but Windows still held the executable image
briefly, so a one-shot local deployment copy failed before the `finally` block restored service.

### Resolution

Use the same bounded copy-retry behavior as the formal package script after confirming the service
registration points at the exact package executable.

### Metadata

- Reproducible: timing-dependent
- Related Files: scripts/package.ps1

---

## [ERR-20260722-052] rustc-could-not-map-gpui-with-low-commit

**Logged**: 2026-07-22T17:40:00+08:00
**Priority**: medium
**Status**: resolved
**Area**: tooling

### Summary

Windows had only about 2.45 GiB free virtual commit while unrelated GUI workloads retained large
private allocations. `rustc` could not memory-map existing GPUI rlibs for a focused desktop test.

### Error

Rust `E0786`, OS error 1455: the paging file was too small to mmap `libgpui` and
`libgpui_component` metadata.

### Suggested Fix

Do not terminate unrelated user processes. Retry with one build job after commit pressure falls;
keep the previously successful full Release gate as the last packaged baseline until the new
desktop delta has itself passed.

### Resolution

The focused test passed against the smaller Release dependency metadata with one build job. The
long-running first attempt was allowed to finish rather than starting overlapping compilers.

### Metadata

- Reproducible: environment-dependent
- Pattern-Key: rustc.gpui_low_commit_mmap
- Recurrence-Count: 5
- Last-Seen: 2026-07-22
- Related Files: rust/apps/nxtools-desktop/src/quick_launch.rs

---

## [ERR-20260722-053] synthetic-quick-launch-smoke-missed-real-popup

**Logged**: 2026-07-22T15:08:00+08:00
**Priority**: high
**Status**: resolved
**Area**: tests

### Summary

The first automated quick-launch crash probe injected double-Ctrl with `keybd_event`; the Windows
low-level hook correctly rejected injected input, and a broad window-size matcher then moved the
pointer over the main window and falsely reported that the popup survived.

### Error

The probe printed `PASS` even though the desktop log contained no `quick-launch popup opened` or
query-publication event for that run.

### Suggested Fix

Drive the versioned `quick_launch_panel_requested` protocol boundary, require the collapsed popup
and a published populated result window before hovering, and keep a deterministic source-lifecycle
gate for the exact leased-entity call chain.

### Metadata

- Reproducible: yes
- Related Files: tests/runtime/quick-launch-search-smoke.ps1, tests/architecture/quick-launch-lifecycle.ps1
- See Also: ERR-20260720-024

### Resolution

- **Resolved**: 2026-07-22T15:08:00+08:00
- **Notes**: The smoke probe now uses the protocol boundary and the architecture gate fails whenever a result-pointer callback synchronously updates the application.

---

## [ERR-20260722-054] recursive-temp-cleanup-command-policy

**Logged**: 2026-07-22T15:13:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

A combined diagnostic command containing `Remove-Item -Recurse` was rejected by the execution
policy even though the temporary WinDbg path was validated first.

### Resolution

Split cleanup from artifact verification, compare the resolved path with the one permitted temp
directory, then call `System.IO.Directory.Delete(path, true)` only after that exact equality check.

### Metadata

- Reproducible: policy-dependent
- Related Files: none

---

## [ERR-20260723-055] directory-navigation-architecture-gate-stale

**Logged**: 2026-07-23T11:43:13+08:00
**Priority**: medium
**Status**: resolved
**Area**: tests

### Summary

Release packaging failed because the Quick Launch architecture gate still required Rust's removed
`directory_navigation_target` fallback after the contract moved all Windows path and shortcut
resolution into C#.

### Error

`tests/architecture/quick-launch-lifecycle.ps1` rejected the build while matching
`fn directory_navigation_target` and `ApplicationSource::IndexedFile => ...parent()`.

### Context

- Command: `scripts/package.ps1 -Configuration Release`
- The implementation and reference contract now require Rust to accept only C#'s absolute
  `ShellActionMenu.navigation_directory`.
- The stale gate encoded the superseded source-type heuristic and therefore opposed the documented
  responsibility boundary.

### Suggested Fix

Update the gate to require the protocol field, C# shortcut/filesystem resolution, Rust absolute-path
acceptance, and the absence of the Rust fallback/source-type heuristic.

### Metadata

- Reproducible: yes
- Related Files: tests/architecture/quick-launch-lifecycle.ps1,
  docs/reference-code/quick-launch-research.md

### Resolution

- **Resolved**: 2026-07-23T11:45:00+08:00
- **Notes**: The gate now requires C# navigation/shortcut resolution, the protocol's optional final
  directory, Rust absolute-path acceptance, and the absence of the removed Rust source heuristic.

---

## [ERR-20260724-059] release-package-short-command-timeout

**Logged**: 2026-07-24T10:00:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tooling

### Summary

The first Release packaging invocation used a one-second command timeout. The verification process
was terminated after the architecture gate even though packaging is a multi-stage build.

### Resolution

Rerun long package commands through the yielding execution cell with a full command timeout, then
poll the returned cell instead of imposing a short process timeout. The rerun completed and created
the Release package.

### Metadata

- Reproducible: yes
- Related Files: `scripts/package.ps1`
- Recurrence-Count: 2
- Last-Seen: 2026-07-24

### Recurrence

- **2026-07-24T10:42:00+08:00**: The same package command was accidentally launched with a
  one-second shell timeout. No package result was inferred from the terminated invocation; the
  command was immediately restarted through a yielding execution cell with a full process timeout.

---

## [ERR-20260724-060] listary-reference-doc-context-mismatch

**Logged**: 2026-07-24T10:16:24+08:00
**Priority**: low
**Status**: resolved
**Area**: docs

### Summary

An `apply_patch` operation for the Listary file-dialog research note failed because one hunk
combined sections that occur in a different order in the document.

### Error

`apply_patch verification failed: Failed to find expected lines`

### Context

- The patch mixed the top capability table, the managed call-chain section, and the later
  implementation-invariants section in one operation.
- No file content was changed by the failed operation.

### Suggested Fix

Patch each precisely located section independently after locating its heading or exact line.

### Metadata

- Reproducible: yes
- Related Files: `docs/reference-code/listary-file-dialog-quick-switch.md`

### Resolution

- **Resolved**: 2026-07-24T10:16:24+08:00
- **Notes**: Switched to small heading-scoped documentation hunks.

---

## [ERR-20260724-061] product-design-critical-override-path

**Logged**: 2026-07-24T10:16:24+08:00
**Priority**: low
**Status**: resolved
**Area**: tooling

### Summary

The first critical-override refresh used a nonexistent `_shared` path.

### Error

`Cannot find path ...\skills\_shared\critical-overrides.md because it does not exist.`

### Context

- The installed Product Design plugin stores the file under
  `product-design\0.1.52\references\critical-overrides.md`.
- The self-improvement skill itself was read successfully before the second path failed.

### Suggested Fix

Resolve plugin reference files from the installed skill's own relative links or search the
installed package before reading them.

### Metadata

- Reproducible: yes
- Related Files: none

### Resolution

- **Resolved**: 2026-07-24T10:16:24+08:00
- **Notes**: Located and read the installed `references\critical-overrides.md`.

---

## [ERR-20260724-062] guessed-platform-command-file-path

**Logged**: 2026-07-24T10:18:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tooling

### Summary

A source-inspection command guessed a nonexistent `Protocol/PlatformCommands.cs` path.

### Error

`Cannot find path ...\Protocol\PlatformCommands.cs because it does not exist.`

### Context

- The protocol records needed for this task are declared in `Windows/WindowsPlatformEvent.cs`.
- The failed read did not modify files.

### Suggested Fix

List or search the directory before reading a guessed source filename.

### Metadata

- Reproducible: yes
- Related Files: `csharp/NxTools.PlatformHost/Windows/WindowsPlatformEvent.cs`

### Resolution

- **Resolved**: 2026-07-24T10:18:00+08:00
- **Notes**: Located the declaration with `rg` and read the actual file.

---

## [ERR-20260724-063] desktop-test-rustc-out-of-memory

**Logged**: 2026-07-24T10:30:00+08:00
**Priority**: medium
**Status**: resolved
**Area**: tests

### Summary

Linking the large `nxtools-desktop` debug test binary exhausted available memory before compiler
diagnostics could complete.

### Error

`rustc-LLVM ERROR: out of memory` while compiling the desktop binary with `--test`.

### Context

- Command: `cargo test -p nxtools-desktop ... --no-run`
- The same invocation also ran `cargo fmt --check`; its only findings were formatting diffs.
- The failure occurred during the heavyweight GPUI desktop test binary link, not in the focused
  protocol/indexer crates.

### Suggested Fix

Format first, then verify with one build job and reduced debug information or the Release profile;
run focused lightweight crate tests separately.

### Metadata

- Reproducible: unknown
- Related Files: `rust/apps/nxtools-desktop/src/quick_launch/file_dialog_search.rs`

### Resolution

- **Resolved**: 2026-07-24T10:39:00+08:00
- **Notes**: Re-ran the focused desktop test in the Release profile with one build job; the
  heavyweight GPUI test binary linked successfully and the new test passed.

---

## [ERR-20260724-064] powershell-window-probe-reserved-pid

**Logged**: 2026-07-24T11:05:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tooling

### Summary

A read-only native-window probe attempted to assign to PowerShell's reserved `$PID` variable.

### Error

`Cannot overwrite variable PID because it is read-only or constant.`

### Context

- The probe was intended only to compare `GetWindowRect` and DWM visible bounds.
- The failed invocation did not modify or move any application windows.

### Suggested Fix

Use a non-reserved name such as `$processIdValue` for `GetWindowThreadProcessId`.

### Metadata

- Reproducible: yes
- Related Files: none

### Resolution

- **Resolved**: 2026-07-24T11:05:00+08:00
- **Notes**: Corrected the probe variable name before retrying.

---

## [ERR-20260724-065] source-guard-matched-its-own-literal

**Logged**: 2026-07-24T11:25:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

A source-level guard for forbidden Material icon usages matched the exact icon literal written
inside the guard itself.

### Error

`fixed_dialog_search_header_does_not_advertise_unimplemented_actions` failed after the UI usages
had already been removed.

### Context

- The guard used `include_str!` on its own Rust source file.
- The forbidden strings appeared verbatim in the test assertions.

### Suggested Fix

Construct the forbidden usage strings at runtime so the guard scans production syntax without
matching its own test literals.

### Metadata

- Reproducible: yes
- Related Files: `rust/apps/nxtools-desktop/src/quick_launch/file_dialog_search.rs`

### Resolution

- **Resolved**: 2026-07-24T11:25:00+08:00
- **Notes**: The guard now builds each `MaterialIcon::<name>` pattern dynamically.

---

## [ERR-20260724-067] interactive-directory-opus-test-changed-state

**Logged**: 2026-07-24T11:24:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

The full PlatformHost console suite reached a real Directory Opus integration check whose
foreground/pane state changed during the run, after all deterministic file-dialog regressions
had passed.

### Error

`FAIL real Directory Opus lister exposes a third-party navigation context: assertion failed`

### Context

- The failure depends on the current interactive Directory Opus foreground and pane state.
- The new deterministic file-dialog reuse, retry, and geometry tests all passed before it.
- The test run did not launch or manipulate Directory Opus.

### Suggested Fix

Use the focused deterministic regression set while iterating on file-dialog movement, and run the
real Directory Opus boundary only when its documented foreground precondition is deliberately set.

### Metadata

- Reproducible: no
- Related Files: `csharp/NxTools.PlatformHost.Tests/Program.cs`

### Resolution

- **Resolved**: 2026-07-24T11:24:00+08:00
- **Notes**: Classified as an unrelated interactive-state failure; subsequent verification uses
  deterministic file-dialog tests plus the architecture gate.

---

## [ERR-20260724-068] parallel-verification-treated-empty-rg-as-failure

**Logged**: 2026-07-24T11:27:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

A parallel verification batch failed as a whole because the cleanup `rg` probe correctly found
no debug markers and returned its standard no-match exit code.

### Error

`Script error: Exit code: 1`

### Context

- The batch combined the PlatformHost suite, architecture gate, whitespace check, and debug-marker
  cleanup probe.
- `rg` exit code 1 means “no matches,” which is the expected cleanup result.

### Suggested Fix

Normalize `rg` exit code 1 to success for absence checks, and print each parallel result
independently.

### Metadata

- Reproducible: yes
- Related Files: none

### Resolution

- **Resolved**: 2026-07-24T11:27:00+08:00
- **Notes**: Follow-up verification treats no-match cleanup probes as passing.

---

## [ERR-20260724-070] release-package-duplicate-build-resource-limit

**Logged**: 2026-07-24T14:52:00+08:00
**Priority**: medium
**Status**: resolved
**Area**: infra

### Summary

The verified release package flow rebuilt the same Rust binaries and exceeded the command
observation window after an earlier build had already hit the host virtual-memory limit.

### Error

`memory allocation of 2097152 bytes failed`

`command timed out after 304053 milliseconds`

### Context

- `scripts/package.ps1 -SkipVerification` still runs `cargo build` before packaging.
- The desktop Release binary had already been built successfully with one Cargo job,
  incremental Release output, and 128 codegen units.
- The command timeout did not terminate the full child process tree, so the background Cargo
  process had to be monitored before deciding whether the package completed.

### Suggested Fix

For memory-constrained packaging, build once with the low-peak profile, verify the target binary
timestamp, then run only the manifest, .NET publish, and package copy steps. A future script option
could explicitly skip the Cargo build when trusted binaries already exist.

### Metadata

- Reproducible: yes
- Related Files: `scripts/package.ps1`, `scripts/build.ps1`
- See Also: ERR-20260724-059, ERR-20260724-063
- Recurrence-Count: 2
- Last-Seen: 2026-07-28

### Resolution

- **Resolved**: 2026-07-24T14:52:00+08:00
- **Notes**: Built the desktop binary with the low-peak Release profile and assembled the package
  from verified target binaries without starting the application. On 2026-07-28, the same
  60-second observation timeout elapsed while the low-peak Release link continued successfully in
  the background; the process was monitored to completion before cached package assembly resumed.

---

## [ERR-20260724-071] tool-orchestration-command-name-mismatch

**Logged**: 2026-07-24T14:52:00+08:00
**Priority**: low
**Status**: resolved
**Area**: infra

### Summary

The JavaScript tool orchestrator called `tools.exec_command`, but this environment exposes the
PowerShell runner as `tools.shell_command`.

### Error

`TypeError: tools.exec_command is not a function`

### Context

- The failure occurred during a read-only process and artifact status check.
- No project state was changed.

### Suggested Fix

Use the exact nested tool name exposed in the current tool metadata before composing parallel
calls.

### Metadata

- Reproducible: yes
- Related Files: none

### Resolution

- **Resolved**: 2026-07-24T14:52:00+08:00
- **Notes**: Re-ran the checks through `tools.shell_command`.

---

## [ERR-20260724-072] powershell-lastexitcode-null-guard

**Logged**: 2026-07-24T14:52:00+08:00
**Priority**: low
**Status**: resolved
**Area**: infra

### Summary

A native-exit-code guard ran immediately after a PowerShell script and treated an unset
`$LASTEXITCODE` as failure, ending a packaging command after only the manifest step.

### Error

The command exited successfully before running `dotnet publish` or copying package files.

### Context

- `write-release-binary-manifest.ps1` is a PowerShell script, not a native executable.
- `$LASTEXITCODE` is not a reliable success signal for that script invocation.

### Suggested Fix

Let terminating PowerShell errors propagate for script calls. Check `$LASTEXITCODE` only directly
after native commands such as `cargo` or `dotnet`.

### Metadata

- Reproducible: yes
- Related Files: `scripts/write-release-binary-manifest.ps1`

### Resolution

- **Resolved**: 2026-07-24T14:52:00+08:00
- **Notes**: Split manifest, .NET publish, and file assembly into correctly guarded steps.

---

## [ERR-20260724-073] calculator-test-format-check

**Logged**: 2026-07-24T15:02:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

The first calculator verification batch returned failure because the edited regression test had
two rustfmt-only line wrapping differences.

### Error

`cargo fmt --all --check` reported formatting diffs in
`rust/crates/nxtools-finder/tests/calculator.rs`.

### Context

- The check ran in parallel with the focused Finder and desktop tests.
- The reported changes were mechanical wrapping only.

### Suggested Fix

Run `cargo fmt --all` after test edits and repeat each verification item so one parallel failure
does not obscure the other results.

### Metadata

- Reproducible: yes
- Related Files: `rust/crates/nxtools-finder/tests/calculator.rs`

### Resolution

- **Resolved**: 2026-07-24T15:02:00+08:00
- **Notes**: Applied rustfmt; Finder tests, desktop calculator tests, format check, and strict Clippy
  all pass.

---

## [ERR-20260724-074] calculator-output-semantic-deduplication

**Logged**: 2026-07-24T15:28:00+08:00
**Priority**: medium
**Status**: resolved
**Area**: backend

### Summary

The first multi-result implementation deduplicated calculator outputs by display text and used a
generic id for the primary decimal value, which either removed a valid binary-zero row or emitted
the decimal row twice.

### Error

`=132 + 23` returned six rows instead of five, while `=1 - 1` could not retain both decimal `0` and
binary `0` as distinct result types.

### Context

- Calculator representations are semantic result types, not unique strings.
- Different representations can legitimately share the same visible value.

### Suggested Fix

Assign every representation a stable semantic id (`decimal`, `hexadecimal`, `binary`, and so on)
and deduplicate only by that id.

### Metadata

- Reproducible: yes
- Related Files: `rust/crates/nxtools-finder/src/calculator.rs`

### Resolution

- **Resolved**: 2026-07-24T15:28:00+08:00
- **Notes**: Decimal primary results now use the `decimal` id and output deduplication is type-based.

---

## [ERR-20260724-075] calculator-octal-derived-value

**Logged**: 2026-07-24T15:28:00+08:00
**Priority**: medium
**Status**: resolved
**Area**: backend

### Summary

Legacy octal output used an ambiguous leading-zero string, so the multi-result layer could parse
`0377` as decimal 377 and derive incorrect secondary representations.

### Error

An explicit `=255 to oct` result could produce secondary values for 377 instead of 255.

### Context

- The single-result implementation never had to parse its formatted octal value again.
- Multi-result derivation requires an unambiguous round-trip representation.

### Suggested Fix

Use the explicit `0o` prefix and recognize it as a typed octal display value before deriving any
secondary formats.

### Metadata

- Reproducible: yes
- Related Files: `rust/crates/nxtools-finder/src/calculator.rs`

### Resolution

- **Resolved**: 2026-07-24T15:28:00+08:00
- **Notes**: Octal output is now `0o377`; standalone gating, reverse conversion and expression
  parsing all recognize `0o`/`0O`, and regression coverage verifies all derived values remain 255.

---

## [ERR-20260724-076] calculator-large-integer-currency-precision

**Logged**: 2026-07-24T15:42:00+08:00
**Priority**: high
**Status**: resolved
**Area**: backend

### Summary

The first currency representation converted exact `i64` calculator values to `f64`, allowing
integers above `2^53` to generate and copy a rounded Chinese amount.

### Error

`9007199254740993` could generate a currency representation for `9007199254740992`.

### Context

- The primary decimal result remained exact.
- Only the derived currency row lost precision through the integer-to-floating conversion.

### Suggested Fix

Keep exact integer amounts on an integer path and use floating-point cents only for decimal results.

### Metadata

- Reproducible: yes
- Related Files: `rust/crates/nxtools-finder/src/calculator.rs`

### Resolution

- **Resolved**: 2026-07-24T15:42:00+08:00
- **Notes**: Integer currency conversion now uses `unsigned_abs` and exact group formatting; a
  regression verifies the full amount for `9007199254740993`.

---

## [ERR-20260724-077] ripgrep-unbalanced-pattern

**Logged**: 2026-07-24T15:42:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

A source-location probe passed an incorrectly escaped grouped expression to ripgrep.

### Error

`regex parse error: unclosed group`

### Context

- The probe was read-only and did not affect project files.
- The needed locations were simple fixed fragments.

### Suggested Fix

Use fixed-string search or PowerShell `Select-String` when locating literal Rust syntax that
contains parentheses and quotes.

### Metadata

- Reproducible: yes
- Related Files: none

### Resolution

- **Resolved**: 2026-07-24T15:42:00+08:00
- **Notes**: Re-ran the probe with `Select-String` and exact context.

---

## [ERR-20260724-078] cargo-test-output-pipe-timeout

**Logged**: 2026-07-24T16:10:00+08:00
**Priority**: medium
**Status**: resolved
**Area**: tests

### Summary

The first full desktop test run used a one-second command timeout, which closed the Rust test
harness output pipe while tests were still reporting results.

### Error

`io error when listing tests: BrokenPipe`

### Context

- Command: `cargo test --release -p nxtools-desktop`
- The release build completed, but the command runner timed out before the test harness finished.
- The resulting worker panics were caused by the closed output pipe, not failing assertions.

### Suggested Fix

Use a realistic timeout for full Cargo test suites and reserve short yields for process-aware command
execution that returns a resumable cell ID.

### Metadata

- Reproducible: yes
- Related Files: none
- Pattern-Key: cargo.command_timeout_must_cover_build
- Recurrence-Count: 3

### Resolution

- **Resolved**: 2026-07-24T16:11:00+08:00
- **Notes**: Confirmed no test process remained, then reran with a ten-minute timeout; all 140 tests
  passed. The same short-timeout mistake recurred twice during the URL slice before the low-memory
  build was rerun with a full ten-minute timeout.

---

## [ERR-20260724-079] guessed-reference-paths-in-batched-read

**Logged**: 2026-07-24T16:22:00+08:00
**Priority**: low
**Status**: resolved
**Area**: docs

### Summary

A batched reference read guessed SwiftList and Listary file paths, so one missing path made the
whole command return failure and obscured useful search output.

### Error

`Cannot find path ... because it does not exist.`

### Context

- The provider existed under `Plugins/CoreExtensions/Providers`, not the guessed parent directory.
- Listary `Class179` existed under `Core/ViewModels`, not `Core/Actions`.

### Suggested Fix

Resolve reference paths with `rg --files` first, then read confirmed paths in independent calls.

### Metadata

- Reproducible: yes
- Related Files: docs/reference-code/quick-launch-research.md

### Resolution

- **Resolved**: 2026-07-24T16:24:00+08:00
- **Notes**: Located both files from the source index and completed the call-chain review.

---

## [ERR-20260724-080] powershell-reflection-argument-wrapper

**Logged**: 2026-07-24T16:28:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

PowerShell passed pipeline `PSObject` wrappers to `MethodInfo.Invoke` instead of a typed object array.

### Error

`Object of type 'System.Management.Automation.PSObject' cannot be converted to type 'System.String'.`

### Context

- The read-only probe invoked Listary's internal URL normalization method against sample strings.

### Suggested Fix

Assign the pipeline value to a typed string and pass `[object[]]@($inputValue)` to reflection.

### Metadata

- Reproducible: yes
- Related Files: none

### Resolution

- **Resolved**: 2026-07-24T16:29:00+08:00
- **Notes**: The typed invocation exposed the URL regex and normalization behavior successfully.

---

## [ERR-20260724-081] windows-build-commit-memory-exhaustion

**Logged**: 2026-07-24T16:40:00+08:00
**Priority**: medium
**Status**: resolved
**Area**: tests

### Summary

Rust/LLVM compilation failed while Windows had less than 2 GB free committed virtual memory.

### Error

`页面文件太小，无法完成操作 (os error 1455)` / `rustc-LLVM ERROR: out of memory`

### Context

- Existing user applications accounted for most committed memory and were deliberately left running.
- Both GPUI Debug metadata mapping and later Rust code generation were affected.
- Some interrupted artifacts subsequently produced a transient `can't find crate` error.

### Suggested Fix

Use `CARGO_BUILD_JOBS=1`, disable incremental compilation for the verification run, and set
`RUSTFLAGS=-C codegen-units=256` to reduce peak compiler memory without terminating user processes.

### Metadata

- Reproducible: environment-dependent
- Related Files: none
- Recurrence-Count: 9
- Last-Seen: 2026-07-30

### Resolution

- **Resolved**: 2026-07-24T16:44:00+08:00
- **Notes**: Low-peak Release verification passed all finder suites; the cached desktop Release
  suite then passed 142 tests with a single Cargo job. On 2026-07-25, desktop test linking again
  reached `LNK1102`; `cargo check` remained green and verification switched back to the documented
  low-peak environment. On 2026-07-27, a full workspace test reached `os error 1455` while mapping
  existing GPUI rlibs; package-scoped tests and the non-GPUI workspace suites remained the fallback.
  Later that day, download-engine compilation failed first with a small allocation failure and then
  a transient standard-library metadata-stub error while free virtual memory was nearly zero.
  The Phase 1A desktop test link later failed with OS 1453 while only about 425 MiB physical memory
  remained; download-core tests had already passed, so verification resumed with cached, single-job,
  non-incremental package commands after memory recovered. The Phase 1A add-window regression suite
  later hit OS 1455 again while linking the GPUI test binary; `cargo check` stayed green and the
  interaction invariants were additionally protected by a source architecture gate. On 2026-07-28,
  a package wrapper launched without inheriting the documented low-peak environment and several
  Release test-target `rustc` processes exited with `0xc0000409`; rerunning the package with
  `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0` and `RUSTFLAGS=-C codegen-units=256` is required even
  when the same suites already passed individually.
  Later on 2026-07-28, a post-refactor full download test rebuild failed to map cached Serde
  metadata and then exhausted a 62 KiB allocation while Windows reported only about 391 MiB free
  virtual memory. Full-target Clippy and the behavioral suites had already passed; verification
  remained split until system commit memory recovered. The Phase 3 GPUI filter test rebuild later
  hit the same OS 1455 while mapping `libtest` with about 4.2 GiB free virtual commit; no Cargo or
  rustc child survived, so verification was deferred until commit pressure fell. The subsequent
  production-row GPUI regression rebuild hit OS 1455 while mapping cached GPUI metadata; the
  packaged NxTools and indexer processes were stopped before retrying with the low-peak settings.
  On 2026-07-29, the first selected-task-detail GPUI test rebuild hit OS 1455 while mapping cached
  GPUI and download rlibs despite the low-peak settings; no user application was terminated and
  verification was retried after the failed compiler process released its commit charge.
  On 2026-07-30, the first post-review task-window regression rebuild exhausted a 2 MiB compiler
  allocation and then failed to map cached GPUI metadata with OS 1455. No compiler process survived;
  verification resumed with the same low-peak environment after virtual commit recovered.
  Later on 2026-07-30, the first capability-driven context-menu regression rebuild failed to map
  cached GPUI metadata with OS 1455. The unchanged low-peak command passed after commit memory
  recovered; no source or cache deletion was required.
  The following workspace Clippy/test pass also exhausted commit while mapping Rust's `libtest`
  metadata and terminated that rustc with `0xc0000409`. Targeted menu builds passed before and
  after the failure; verification resumed only after the compiler exited and commit recovered.

---
## [ERR-20260728-GITPUBLISH] github_publish_prerequisites_missing

**Logged**: 2026-07-28T17:43:00+08:00
**Priority**: medium
**Status**: resolved
**Area**: infra

### Summary

The publish workflow could not start because the checkout has no Git remote and GitHub CLI is not
installed.

### Error

```text
gh: The term 'gh' is not recognized
git remote -v: no configured remotes
```

### Context

- The user requested publishing the current NxTools.Next project.
- Preflight stopped before staging because the destination and mixed-worktree scope are unresolved.

### Suggested Fix

Configure the intended remote URL, install and authenticate GitHub CLI when the destination is
GitHub, then explicitly stage project sources while excluding local diagnostics and build caches.

### Metadata

- Reproducible: yes
- Related Files: `.git/config`, `.gitignore`

### Resolution

- **Resolved**: 2026-07-28T17:48:00+08:00
- **Notes**: The user requested a direct Git push, not PR creation. GitHub CLI is unnecessary for
  that narrower flow; plain Git and the configured credential helper are sufficient.

---
## [ERR-20260728-MUTABLEFIXTURE] mutable_resource_checkpoint_race

**Logged**: 2026-07-28T17:55:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

The combined Release suite let the mutable-resource fixture finish before the test paused it at a
checkpoint.

### Error

```text
resume mutable task: InvalidAction { action: "resume", status: Completed }
```

### Context

- Every preceding architecture, CLI, clipboard, core and desktop test passed.
- The exact mutable-resource test passed immediately when rerun in isolation.

### Suggested Fix

Make the fixture's checkpoint pause deterministic instead of relying on transfer timing. Until
then, confirm this failure with the exact test before changing transfer production code.

### Metadata

- Reproducible: no
- Related Files: `rust/crates/nxtools-download/tests/engine_contract.rs`
- See Also: `ERR-20260728-HTTPFIXTURE`

### Resolution

- **Resolved**: 2026-07-28T17:55:00+08:00
- **Notes**: Exact Release test passed on the immediate rerun; no product change was needed.

---
## ERR-20260808-INTEGRATIONTESTPATH

**Date**: 2026-08-08
**Context**: Resuming the download/browser validation work after context compaction
**Error**: A repository search assumed integration tests lived under `src/tests*`; they actually live under the crate-level `tests/` directory.
**Root cause**: I relied on a summarized path instead of resolving the files with `rg --files` first.
**Prevention**: Resolve uncertain test paths with `rg --files <crate> | rg 'tests|support'` before a multi-file search.
## ERR-20260808-DESKTOPCHECKTIMEOUT

**Date**: 2026-08-08
**Context**: Verifying the two Desktop compile fixes exposed by Release packaging
**Error**: The first single-job `cargo check -p nxtools-desktop --bin nxtools-desktop` exceeded the 240-second tool timeout without reporting a compiler error.
**Root cause**: The feedback-loop timeout did not account for the first full GPUI/Desktop dependency check with incremental compilation disabled.
**Prevention**: Keep the required single-job/zero-incremental settings, but allow at least ten minutes for the first Desktop check; confirm no Cargo or rustc process survives a tool timeout before retrying.

**Resolution**: Re-ran the same check with a ten-minute timeout; it completed successfully in 89 seconds after the first attempt had populated part of the dependency cache.

## ERR-20260808-DESKTOPRELEASECOMPILE

**Date**: 2026-08-08
**Context**: First atomic Release package after adding browser takeover and taskbar visibility motion
**Error**: Desktop Release compilation found incompatible `Div`/`AnimationElement<Div>` render arms and one `DownloadRequest` initializer missing `expected_total_bytes`.
**Root cause**: Earlier targeted checks covered BrowserHost and the download core but did not compile the Desktop binary that integrates both changes.
**Prevention**: Before declaring a cross-surface slice package-ready, run a single-job `cargo check -p nxtools-desktop --bin nxtools-desktop`; normalize conditional GPUI render branches to `AnyElement` and update all request constructors when the domain request changes.

**Resolution**: Converted both taskbar overlay render arms to `AnyElement`, supplied `expected_total_bytes: None` for manual desktop requests, and passed the targeted Desktop check.
## ERR-20260808-MANIFESTREGEXESCAPE

**Date**: 2026-08-08
**Context**: Read-only verification of packaged Rust hashes against the generated C# manifest
**Error**: An interpolated PowerShell regex used C-style escaped quotes and failed at parse time.
**Root cause**: PowerShell double-quoted strings do not use backslash to escape embedded quotes.
**Prevention**: Avoid nested interpolated regex quoting for fixed generated manifests; extract all 64-character hash literals once and compare them in declared order.

## ERR-20260808-CLIPBOARDCOMPLETIONREENTRY

**Date**: 2026-08-08
**Context**: First packaged browser-takeover acceptance run
**Error**: Protected `NxTools.exe` terminated with `0xc0000409` after `ClipboardPanel::handle_command_result` synchronously updated an already leased `ClipboardApplication`.
**Root cause**: Successful paste usage persistence and copy-only refresh crossed from the panel back into the application during the application's platform-notification callback.
**Prevention**: Every panel callback reached from an application notification must defer application mutation to the next GPUI turn; any panel refresh depending on that mutation must be deferred for a second turn and protected by an architecture gate.

**Resolution**: Deferred usage persistence and copy-only refresh across separate GPUI turns, added `clipboard-lifecycle.ps1`, passed Desktop check/Clippy and BrowserHost gates, and produced a new hash-verified atomic package.
## ERR-20260808-FOREACHPIPE

**Date**: 2026-08-08
**Context**: Read-only verification of the repackaged protected binary hashes
**Error**: A statement-level PowerShell `foreach` was piped directly to `Format-Table`, causing `An empty pipe element is not allowed`.
**Root cause**: The command ignored the explicit repository diagnostic rule in `AGENTS.md`.
**Prevention**: Always assign statement-level `foreach` output to a task-specific variable before piping it to formatting cmdlets.
**Recurrence**: Repeated during the 18:02 final hash report even though the rule had already been
promoted to `AGENTS.md`; the package itself was unaffected.
**Recurrence count**: 2 for this entry; see also `ERR-20260723-071`.
**Latest resolution**: Re-ran the report by assigning all rows to `$artifactHashRows` before
`Format-Table`.

## [ERR-20260808-GPUITESTTIMEOUT] desktop_gpui_test_timeout_left_child_builds_running

**Logged**: 2026-08-08T16:17:00+08:00
**Priority**: high
**Status**: resolved
**Area**: tests

### Summary

A targeted Desktop GPUI test exceeded the 120-second tool timeout while compiling `aws-lc-sys`,
and the timeout wrapper left its Cargo/build-script process tree running.

### Error

```text
command timed out after 124032 milliseconds
```

### Context

- Command: single-job, non-incremental `cargo test -p nxtools-desktop <exact-test>`.
- No Rust compiler diagnostic was emitted before the timeout.
- Two Cargo processes and one `aws-lc-sys` build script remained after the wrapper returned.

### Suggested Fix

Allow the repository's documented long timeout for a first Desktop test build. After every tool
timeout, inspect the exact Cargo process tree and stop only the verified command descendants before
continuing, so an abandoned build cannot consume commit memory in the background.

### Metadata

- Reproducible: unknown
- Related Files: `rust/apps/nxtools-desktop/src/downloads.rs`, `AGENTS.md`
- See Also: `ERR-20260808-DESKTOPCHECKTIMEOUT`

### Resolution

- **Resolved**: 2026-08-08T16:18:00+08:00
- **Notes**: Verified and stopped only PIDs 15720, 21956 and 16036 from the timed-out test tree;
  subsequent verification reported no remaining targeted process. A second run with a ten-minute
  timeout also produced no compiler diagnostic or test binary; this time the wrapper left no child
  build process. Do not repeat the same Desktop test-target build in this session; use the passing
  Desktop check plus architecture/engine/host gates and leave the GPUI test executable gate noted.

---

## [ERR-20260808-RELEASEMANIFESTPATH] assumed_generated_manifest_directory

**Logged**: 2026-08-08T16:34:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

Post-package hash verification assumed the generated release manifest lived under `Windows/`;
the package script writes it under `Generated/`.

### Error

```text
Cannot find path 'csharp/NxTools.PlatformHost/Windows/ReleaseBinaryManifest.g.cs'
```

### Context

- The Release package itself completed successfully.
- Only the follow-up read-only hash verification command failed.

### Suggested Fix

Resolve generated artifact locations with `rg --files` before reading them instead of relying on
memory from a previous package pass.

### Metadata

- Reproducible: yes
- Related Files: `csharp/NxTools.PlatformHost/Generated/ReleaseBinaryManifest.g.cs`

### Resolution

- **Resolved**: 2026-08-08T16:34:00+08:00
- **Notes**: Located the actual generated manifest with `rg --files`; hash verification resumed
  against that exact path.

---

## [ERR-20260808-ACTIVELOGSHARE] direct_readlines_on_active_log

**Logged**: 2026-08-08T16:43:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

`File.ReadLines` could not open the active BrowserHost log while that process held the file.

### Error

```text
The process cannot access browser-host.log.2026-08-08 because it is being used by another process.
```

### Context

- A combined diagnostic command read Desktop, BrowserHost and PlatformHost logs.
- Desktop and PlatformHost evidence was returned, but the active BrowserHost file used a sharing
  mode incompatible with `File.ReadLines`.

### Suggested Fix

For an actively written log, open a `FileStream` with `FileShare.ReadWrite` and read through a
`StreamReader`, or make a bounded snapshot through that sharing mode before filtering. Keep each
process log extraction separate so one locked file cannot obscure the other evidence.

### Metadata

- Reproducible: yes
- Related Files: `docs/diagnostics.md`

### Resolution

- **Resolved**: 2026-08-08T16:43:00+08:00
- **Notes**: The required timing evidence came from the independently readable Desktop and
  PlatformHost logs; future active-log reads will use shared access.

---

## [ERR-20260808-DOCSPATCHCONTEXT] apply_patch_context_mismatch

**Logged**: 2026-08-08T17:44:00+08:00
**Priority**: low
**Status**: resolved
**Area**: docs

### Summary

A combined documentation patch was rejected because its download-plan context did not match the
file's actual line wrapping.

### Error

```text
apply_patch verification failed: Failed to find expected lines in docs/download-plan.md
```

### Context

- One patch attempted four independent documentation updates.
- The final plan hunk used remembered wrapping instead of the exact nearby lines.
- `apply_patch` rejected the operation without partially changing the files.

### Suggested Fix

Read the exact local context first and apply independent small patches when updating unrelated
document sections.

### Metadata

- Reproducible: yes
- Related Files: `docs/download-plan.md`, `docs/diagnostics.md`, `docs/architecture.md`, `docs/reference-code/download.md`

### Resolution

- **Resolved**: 2026-08-08T17:44:00+08:00
- **Notes**: Reapplied the four updates with exact, file-specific contexts.

---

## [ERR-20260808-POWERSHELLREGEXQUOTE] powershell_rg_double_quote_parse

**Logged**: 2026-08-08T17:47:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

A PowerShell source-search command parsed a double-quoted `rg` regex as shell syntax.

### Error

```text
ParserError: Expressions are only allowed as the first element of a pipeline.
```

### Context

- The regex contained an escaped double quote and alternation inside a double-quoted PowerShell
  string.
- The search was read-only and made no filesystem changes.

### Suggested Fix

Use a single-quoted PowerShell string for literal `rg` regular expressions that contain quotes or
alternation.

### Metadata

- Reproducible: yes
- Related Files: `browser_extension/nxtools-interceptor/background.js`

### Resolution

- **Resolved**: 2026-08-08T17:47:00+08:00
- **Notes**: Re-ran the query with a single-quoted regex and verified the 15-second create-task timeout.

---

## [ERR-20260808-DIALOGCOMPLETIONTYPE] browser_dialog_test_completion_type

**Logged**: 2026-08-08T17:55:00+08:00
**Priority**: low
**Status**: resolved
**Area**: tests

### Summary

The first window-failure regression used `Err(())` while the real GPUI completion channel carries
a bounded error string.

### Error

```text
expected `String`, found `()`
```

### Context

- The test intentionally exercises the real `BrowserDownloadDialogRequest::completion` boundary.
- The incorrect payload shape failed compilation before reaching the intended red assertion.

### Suggested Fix

Read the public completion type at the boundary and send a short synthetic string error.

### Metadata

- Reproducible: yes
- Related Files: `rust/crates/nxtools-download/tests/engine_contract.rs`

### Resolution

- **Resolved**: 2026-08-08T17:55:00+08:00
- **Notes**: Sent `Err("window creation failed".into())`; the test then failed for the intended retryability reason before the implementation changed.

---

## [ERR-20260808-PACKAGETOOLTIMEOUT] release_wrapper_outer_timeout

**Logged**: 2026-08-08T18:00:00+08:00
**Priority**: medium
**Status**: resolved
**Area**: infra

### Summary

The first safe Release packaging attempt was terminated by an accidental one-second outer command
timeout before the wrapper could begin meaningful work.

### Error

```text
command timed out after 1030 milliseconds
```

### Context

- Commit headroom was healthy at 6.88 GiB and the package wrapper had the required low-memory Rust
  environment.
- The tool timeout was set to one second while intending only to yield progress early.
- No Cargo, rustc or linker process remained after termination.

### Suggested Fix

Give the shell command its real bounded build timeout and rely on the orchestration cell's yield
mechanism for progress updates.

### Metadata

- Reproducible: yes
- Related Files: `scripts/package.ps1`, `scripts/windows-build-tools.ps1`

### Resolution

- **Resolved**: 2026-08-08T18:00:00+08:00
- **Notes**: Verified no compiler processes remained and restarted packaging with a proper build timeout.

---
