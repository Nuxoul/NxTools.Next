# REF-20 — owned recipient native execution

Status: specification-built and actual-product controlled recipient execution receipts are accepted as audited evidence in [coordinator acceptance](coordinator-acceptance.md). The coordinator independently reran worker/native creation/readback/TestPlatform boundaries but did not repeat the recipient or loader commands below. Physical/installed/Release acceptance remains separate.

## Executable and source boundary

The own Windows x64 asset is `rust/crates/nxtools-archive/assets/zip-sfx-x64.bin`,835,584bytes. Descriptor offset835,584; physical prefix835,632. The static Release output and bundled asset compare byte-for-byte equal. Current23Rust/embedded-notice build inputs match the retained Release input copies; both test files match the22-test publication-RED and final GREEN snapshots. Final LIST22/RUN22 has22PASS,0FAIL/ignored. Scoped strict Clippy and rustfmt pass.

The PE is8664/PE32+, six sections, no certificate, no overlay; raw image end equals complete file length. Imports are Windows system DLLs only: kernel32/KERNEL32,api-ms-win-core-synch-l1-2-0 and ntdll. The reviewed source does not launch a process or member, invoke network/elevation, modify Registry/associations or depend on installed NxTools/.NET/unzip. Source/fixture/readback/build/license evidence remains under the original B7 scratch and task.

## Approved owned commands and effects

Working directory is exactly `D:\.pi-desktop\data\scratch\0ffe6ffa-9e52-420b-af61-a29fdd0f37b4\b7-ref20-native`. Launch uses an argument array with `shell=false`, redirected stdout/stderr, closed stdin and a90-second per-child timeout. Timeout terminates and waits only the exact child created by that invocation.

The six executable basenames are:

- `ordinary-store.exe`
- `ordinary-deflate.exe`
- `split-store.exe`
- `split-deflate.exe`
- `ordinary-deflate-zip64.exe`
- `split-deflate-zip64.exe`

For each exact file:

1. `<owned-exe> --test`: read its own complete fixed-name family; no extraction or source modification.
2. `<owned-exe> --output <working-directory>\<profile>-output`: create only that absent output root and validated catalog members, using create_new candidates, complete CRC/size/consumption verification and existing safe no-replace rename.
3. Repeat the same output command: expected existing-root refusal; compare all source and previously extracted member bytes, then exclusively reopen every source to verify handle release.

The independent CPython qualifier runs before these commands. It validates actual prefix-inclusive local/central/end/ZIP64 coordinates, packed consumption, Deflate EOS, size/CRC and all literal expected bytes; ordinary profiles additionally use stock zipfile. The six profiles contain9physical files/24catalog entries/49,380decoded bytes. They use the real reviewed PE rather than the earlier nonexecutable512-byte coordinate prefixes.

The separate loader probe invokes only `<working-directory>\loader-probe.exe --licenses`. It copies the exact own bare PE and extends that owned file sequentially to its normal length,4GiB−1,4GiB and4GiB+1. It records actual CreateProcess/native error or child exit, checks embedded-license output on successful starts, closes/waits and deletes that one probe after each case. This isolates Windows physical-file loading from ZIP64 parsing; it cannot prove payload correctness for a large archive or an unlimited loader range.

The driver is scratch `b7-ref20-native.py`. `generate` creates data without execution; `execute-reviewed` and `loader-reviewed` are the explicit reviewed launch commands. Logs, commands, elapsed values, stdout/stderr, exclusive-reopen results and raw native errors stay in the owned evidence directory. No installation, deployment replacement, package/key/ledger operation or system setting is involved.

## Results

All six independently qualified real-stub profiles passed read-only Test and exact extraction. All six repeated output requests failed with existing-root refusal; source and extracted bytes remained unchanged. Every source was exclusively reopened after the child exited. This is18native command outcomes:12successful Test/extract runs and six required refusals, with six complete source-release observations. No archive member was executed.

The four loader observations are:

| Physical EXE bytes | Command | Observed result |
| ---: | --- | --- |
| 835,584 | `--licenses` | Exit0;396,582stdout bytes; no stderr |
| 4,294,967,295 | `--licenses` | Exit0; identical396,582-byte license output; no stderr |
| 4,294,967,296 | `--licenses` | CreateProcess refusal, Windows193 |
| 4,294,967,297 | `--licenses` | CreateProcess refusal, Windows193 |

The own output contract therefore limits the first executable disk, including the complete PE/descriptor/ZIP bytes, to4,294,967,295bytes. Split sets may exceed that total within the original output/count budgets, but their requested per-volume size cannot exceed the first-EXE bound. The actual prefix-plus-span minimum is835,636bytes, subject to original intact-record admission. This bound is measured on the selected own PE rather than inherited from a reference product.

Raw authority: scratch `b7-ref20-native/generated.json`, `execution.json`, `loader.json`, per-command stdout/stderr, `b7-ref20-native-execute.log/.exit0`, `b7-ref20-native-loader.log/.exit0` and wrapperexit0. Pre-execution lane was empty with26,533,916,672available commit bytes and57,948,057,600free disk bytes; the loader probe was removed and no owned native child survived. Large payload correctness, product-generated outputs and Create/UI/Job integration remain separate from this loader observation.

## Product-generated output command review

Independent qualification `b7-ref20-product-oracle-r1.json` passes four actual original Create/Job/native outputs:8physical files,20entries and3,623,188exact decoded bytes. Physical local/central/end coordinates, descriptors, UTF-8, directory attributes, raw Deflate EOS/consumption and CRC pass; ordinary forms also pass stock CPython zipfile. These outputs are distinct from the specification-built profiles above.

The only reviewed executable paths are under `D:\.pi-desktop\data\scratch\0ffe6ffa-9e52-420b-af61-a29fdd0f37b4\b7-ref20-product-creation-green-artifacts`:

| Profile directory | Executable | Extraction destination under scratch `b7-ref20-product-native` |
| --- | --- | --- |
| ordinary-store | `ordinary-store\created.exe` | `ordinary-store-output` |
| ordinary-deflate | `ordinary-deflate\created.exe` | `ordinary-deflate-output` |
| split-store | `split-store\created.exe` | `split-store-output` |
| split-deflate | `split-deflate\created.exe` | `split-deflate-output` |

For each exact absolute executable, run `[exe, "--test"]`, then `[exe, "--output", absolute-fixed-destination]`, then the identical output command expecting existing-root refusal. Working directory is the fixed scratch `b7-ref20-product-native`; launch uses `shell=false`, closed stdin, redirected output and a90-second child timeout. Each destination must be absent before its first extraction. Only the exact invocation's own child can be terminated on timeout.

Before launch, verify the exact reviewed PE prefix/descriptor and qualified family names/sizes. Compare every extracted file byte to that profile's owned expected tree, require both directory entries, preserve all source/output bytes across refusal, then exclusively reopen every source/output to verify child resource release. The unchanged reviewed recipient never starts members, another process, network, elevation, Registry or association operations. No installed/deployed product or reference application is launched. This command set authorizes only these four owned archives and fixed destinations; prior loader observations are not repeated.

### Product-generated native results

All four qualified actual-product profiles passed read-only Test and exact native extraction; all four repeated output commands produced the required existing-root refusal. Source and extracted bytes remained unchanged, both directory entries existed and every source/output file reopened exclusively after child exit. The12command outcomes are retained in scratch `b7-ref20-product-native/execution.json`, individual stdout/stderr and `b7-ref20-product-native-run.log/.exit0`. Exact extraction compares3,623,188bytes across the four profiles. No member or reference application was executed; the prior recipient22/native18/loader4 observations remain unchanged.
