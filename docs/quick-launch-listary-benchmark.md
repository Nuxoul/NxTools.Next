# Quick Launch / Listary installed comparison

## Scope

The external runner compares the installed NxTools Quick Launch and Listary launchers through their real Windows UI Automation controls. It does not replace a provider, call the index service as an unapproved client, execute a result, alter settings/history, rebuild a package or restart a service.

The reference entry points and original show/hide/focus owners are recorded in [the Listary result reference](reference-code/quick-launch-listary-results.md). Listary evidence is limited to its installed user interface and managed boundary; its native engine source is unavailable.

## Run

Use **Windows PowerShell 5.1**, not PowerShell 7, for the .NET Framework UI Automation assemblies. Keep the existing desktop running. The runner starts Listary only if its configured installation is not already running, then stops only that owned instance and its child hook hosts during cleanup. A pre-existing Listary process is preserved.

Do not type into or move focus away from the search windows during sampling. Foreground loss or operator input invalidates the run. No search result is opened.

```powershell
# Pure driver parsing and manifest checks; no app operations.
powershell.exe -NoProfile -File .\tests\performance\quick-launch-listary.ps1 -SelfTest

# Offline statistics and report regressions; no app operations.
$env:PYTHONDONTWRITEBYTECODE = '1'
python -m unittest discover -s tests/performance -p test_quick_launch_listary_report.py -v

# Literal availability only. Choose a new output directory for each retained run.
powershell.exe -NoProfile -File .\tests\performance\quick-launch-listary.ps1 `
    -PreflightOnly -OutputDirectory .\artifacts\performance\launcher-preflight

# Defaults: 23 cases, one warmup and five measured repetitions per product.
powershell.exe -NoProfile -File .\tests\performance\quick-launch-listary.ps1 `
    -OutputDirectory .\artifacts\performance\launcher-comparison

python .\tests\performance\quick_launch_listary_report.py `
    --input .\artifacts\performance\launcher-comparison\raw.json `
    --output-dir .\artifacts\performance\launcher-comparison
```

Override `-NxToolsExecutable` and `-ListaryExecutable` when installations differ. The NxTools desktop must already be running at the specified path, with its own matching PlatformHost. `-CaseId ascii_exact,chinese_literal` can select a smaller set when invoked from PowerShell; `-Repetitions` controls measured repetitions. Every selected case still has its own unscored warmup. Default output directories include a timestamp and run ID.

## Corpus and order

[The manifest](../tests/performance/quick-launch-listary-cases.json) defines 15 targets and 23 queries:

- ASCII exact/fuzzy/multiword and Chinese literal queries.
- Full/partial/initial/mixed pinyin, polyphonic names and `v` spelling.
- Prefix, suffix, quoted, exclusion and OR syntax.
- Directory results and same-name files in different parent directories.
- One deliberately absent query.
- Four illustrative real-system queries: `notepad`, `记事本`, `chrome`, `设置`.

The runner materializes the small, harmless text corpus in `tests/fixtures/nxqlb`. Existing fixture content must match; differing files are not overwritten. Fixtures remain for reproducible reruns. No executable or shortcut is created.

Literal filename lookup verifies each target in each product before measurement. Missing targets are retried within the bounded index catch-up window and recorded, not hidden. Quality for a case is comparable only when every expected and forbidden target was literally observable in both products. Missing shared coverage excludes that case from comparative quality; it remains visible in coverage and per-case evidence.

Measured product order alternates AB/BA. Both products receive the same deterministic rotated case order within each repetition. Warmups and availability checks are stored separately from measurement samples. Cache flushing is not performed.

## Measurement

- Start a high-resolution stopwatch immediately before `ValuePattern.SetValue` on the real input.
- Send the value once and await its asynchronous readback. The acknowledgement delay stays inside the measurement.
- Poll real enabled, onscreen result controls. Record the first nonempty accessible-result observation, input-call time and polling overhead.
- Capture a quiet visible snapshot after at least 1000 ms of observation and 250 ms without content/order changes, with a 3500 ms observation deadline. These are observation windows, not claims of provider completion.
- Clear only the benchmark's own query, await the empty value and require the preceding rows to disappear before another sample.

NxTools results are labelled `name，target`; the unnamed launcher is located by its process and HWND. Listary uses `PART_SearchBox` and `ListItem` results; its more-results navigation row is excluded. Result ordering follows visible row bounds. Parent paths may be displayed with middle ellipsis. Unique corpus names identify ordinary targets; duplicated names additionally require the recorded parent suffix.

Quality uses **top five ordinary visible results** for both products. A case succeeds when all required targets appear there, forbidden targets are absent from captured visible rows, and an expected-empty case has no ordinary rows. Captured visible rows are not the full result set. Real-system queries retain ordered evidence without a quality score because no target intent is imposed.

Latency is **input-to-accessible-result**, including UIA input application, polling and provider-exposure overhead. It is not physical typing, input-method composition, paint latency or native search-engine time. Missing results remain null, never zero milliseconds; errors are counted separately. Percentiles use nearest rank. Overall latency distributions are conditional on a returned result and must be read with no-result counts and individual cases, not used alone to declare a winner.

A separate paired table includes only controlled queries that pass the required-target predicate in every repetition in both products, with complete repetition IDs. It lists the exact query IDs and equal sample counts. Expected-empty and illustrative system queries are excluded from that subset. Syntax cases measure compatibility with the recorded strings; equivalent product-specific expressions need a separate comparison.

## Outputs and limitations

- `raw.json`: installed binary/process metadata, corpus, cases, all availability/warmup/measurement samples, polling costs, run failure and cleanup evidence.
- `summary.json`: common coverage, comparable quality, conditional latency statistics, individual repetitions and sample errors.
- `report.md`: readable measurement tables, ordered examples and exclusions.

This is an existing-configuration installed baseline. Index scope, exclusions, history and language/matching preferences are not normalized or reset. FileVersion and timestamps identify the observed binaries but do not prove correspondence to every current working-tree change. Five repetitions diagnose repeatability; they are not a release latency SLA. Hardware load, physical hotkeys/IME, actual paint, action execution, preview, directory switching and restart/idle-resource acceptance need separate tests.

## Recorded run

[2026-11-17 installed baseline](audits/2026-11-17-quick-launch-listary-benchmark.md): 230 complete formal observations, shared 15-target coverage, category results, paired response timing and real-system candidate differences.

## Speed-focused capture

Run directly inside Windows PowerShell 5.1 to pass the case array:

```powershell
$speedCases = @('ascii_exact', 'chinese_literal', 'full_pinyin', 'pinyin_initials', 'directory', 'same_name', 'system_notepad')
& .\tests\performance\quick-launch-listary.ps1 -Repetitions 10 -CaseId $speedCases `
    -OutputDirectory .\artifacts\performance\launcher-speed
python .\tests\performance\quick_launch_listary_report.py `
    --input .\artifacts\performance\launcher-speed\raw.json `
    --output-dir .\artifacts\performance\launcher-speed
```

The real polling path records `first_result_ms`, `first_target_ms`, `all_targets_ms`, `last_result_change_ms` and `result_change_count`. Target identity uses the same declared names/parent suffixes as the corpus checks. The first nonmatching row does not satisfy target timing; quiet polls and position-only animation do not overwrite content-change time. The broad `notepad` example has no declared target. Missing targets remain null, and records without these timing fields remain unmeasured for those metrics.

Last-change time means the final first-viewport content/order change observed within the capture window, not provider completion, cold-start time or the fixed minimum observation duration. The clock continues to include input acknowledgement and UI Automation overhead.

[2026-11-17 speed-only results](../artifacts/performance/quick-launch-speed-20261117/speed.md): 140 complete formal samples, ten repetitions of each of seven query types per product.
