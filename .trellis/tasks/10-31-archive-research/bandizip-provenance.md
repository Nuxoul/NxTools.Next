# Bandizip download provenance propagation

## Setting to native output context

The inspected Bandizip7.32.0.1 installation connects `bCopyZoneIdentifier` from the EXE settings to the archive library's source ADS holder and both direct and buffered output consumers. Provenance is separate from malware scanning: the former copies retained `Zone.Identifier` bytes under its own conditions; the latter invokes a provider on selected member contents.

Settings writer `Bandizip.exe!0x14016a570` binds provenance to application settings `+0x4a0`; scan enablement is the separate `+0x4a4` field. Options helper `0x1400f14c0` reads the32-bit provenance setting at instruction `0x1400f157f`, stores it at `[RBP-0x15]`, and passes a block beginning at `[RBP-0x69]` through IArk vtable slot `+0x150`. The difference is exactly `0x54`. This path copies the value directly, without bit packing or inversion.

The `CreateArk`/`CArkMan` construction and vtable bytes connect that slot to `ark.x64.dll!0x1800084a0`. The setter copies exactly `0x80` bytes into manager `+0x98`. Its provenance flag is therefore manager `+0xec`, or output-context `+0x54`.

| Object/location | Meaning |
| --- | --- |
| EXE settings `+0x4a0` | Persisted provenance setting |
| EXE options `+0x54` | Value passed through IArk slot `+0x150` |
| `CArkMan+0x98` | Shared options/output-context base |
| `CArkMan+0xec` | Context `+0x54`, provenance enablement |
| `CArkMan+0x118` | Context `+0x80`, pointer to the ADS holder |
| `CArkMan+0x130` | Owning/loading alias of that same holder pointer |
| Holder `+0` / `+8` | Retained byte pointer / byte length |

Constructor `0x180004190` allocates a16-byte holder, initializes its pointer/length and stores the same pointer at manager `+0x130` and `+0x118`. Context `+0x80` is **not inline ADS contents**. The setter's `0x80`-byte options copy ends immediately before that holder-pointer slot.

## Source capture and operation lifetime

EXE open worker `0x1400f0020` applies the options before IArk `Open` at vtable `+0x20`. The native slot resolves to `0x180006430 → 0x180006460`. After successful archive open, the latter tests manager `+0xec`, clears the existing holder and calls `0x18000d500` to read source `:Zone.Identifier` into it. A failed ADS read clears the holder but leaves the archive-open result unchanged.

EXE operation worker `0x1401b3c80` also reapplies the options helper before its operation dispatch. Source ADS capture is connected to successful Open, rather than a new source-ADS read for every extracted member. The retained bytes and the current output flag have separate lifetimes.

Family construction in the containing function queried at `0x180010315` supplies manager `+0x98` to the ZIP extractor constructor `0x180023d70`. Its base constructor `0x180057890` stores the supplied context pointer at extractor `+0x788`. This is the same context consumed by output code, not another flag reconstructed from the member's name or scan result.

## Direct and buffered consumers

Direct finalization `0x18005a6a0` obtains the destination path from the output interface, loads extractor `+0x788`, tests context `+0x54`, loads the holder pointer at context `+0x80`, and calls `0x18000d6d0`. The selected-stream member-completion path `0x180077c80` contains the same provenance call before ordinary metadata/close handling.

For buffered output, the installed factory `0x18001aec0` is invoked with extractor `+0x788` as its context argument. It passes that argument unchanged into `0x18001a9c0`, which retains it at output object `+0xe0`. Normal buffered close `0x180023620` tests that retained context's `+0x54`, obtains its holder at `+0x80` and invokes the same ADS writer. Upstream queue drain/stop and main-file finalization remain the separately recorded [output lifecycle](bandizip-solid-output.md).

## Target filtering and copy result

`0x18000d6d0` first requires nonempty retained bytes and a nonempty destination. It finds the final dot suffix, builds a suffix search string with a trailing semicolon and searches its compiled wide-string extension list through `0x180176390`. The list covers executable/script/document and archive/image suffixes, including EXE, ZIP,7z, DOCX and JS; it is not all output files. The search helper compares literal UTF-16 substrings, not archive contents or a general filesystem wildcard API. Its retained instruction body is `ark-provenance-static-zone-filter.txt:4-158`; the caller's suffix construction and branch are `bandizip-tools-follow-static-zone.txt:129-180`.

Only the admitted path constructs destination `:Zone.Identifier`, opens it, writes the holder bytes through the native byte writer, flushes and closes. Failed write/flush deletes the partial ADS. Empty input, no dot suffix or a failed extension-list lookup returns1 without creating an ADS, so that return value is not proof that a stream was written.

Both inspected output consumers ignore the ADS writer's return and continue their ordinary main-file metadata/close handling. Source-read success, attempted provenance copy, successful ADS write and successful main-file extraction are therefore separate results:

| Boundary | Observed handling |
| --- | --- |
| Source ADS read fails | Clear holder; retain the archive-open result |
| Holder empty or destination excluded | Skip ADS creation; writer returns1 |
| ADS write/flush fails | Delete partial ADS; writer returns0 |
| ADS write and flush succeed | Writer returns1 |
| Caller settles main-file result | Uses its main-file write/close handling, not the ADS writer return |

For NxTools, preserve the existing `copy_motw` owner and native source/output identity. Provenance policy, applicable target coverage and copy outcome must remain explicit; scan verdict or primary-file success is not evidence that provenance was copied. The reference's suffix filter and ignored return are observations, not an approved change to NxTools' policy.

## Evidence map

Canonical files are relative to `evidence/raw/`. Their subjects identify `D:\Tool\Bandizip\Bandizip.exe` or `D:\Tool\Bandizip\ark.x64.dll`; [the index](evidence/index.json) retains exact provider/query identities.

| Boundary | Canonical file | Evidence ID |
| --- | --- | --- |
| Settings field | `bandizip-browser-code-from-0x14016ae74.json` | `ev_f9fb926fd126cdfa26cdb64777d5f3c89aa2efa8a5ca0940a26c332774e531d2` |
| EXE option layout/slot call | `bandizip-provenance-asm-0x1400f14c0.json` | `ev_9c145989c2db9fee4fa0a2cbf0444dfa8aa29eb0ca7e6a62a426fe98a0e90821` |
| Options before Open | `bandizip-provenance-p-0x1400f0020.json` | `ev_e941f92f844c86d7af3513131ba0cef457fb38ec34e15d20a99d8ecbc66b9ee8` |
| Options before operation dispatch | `bandizip-provenance-p-0x1401b3c80.json` | `ev_1e9792297dc29037af59e283d218dde985855d1e867712764b1a5dad91573ae9` |
| Concrete manager factory | `ark-provenance-p-0x180001740.json` | `ev_6fe98db86dbad40a02b91d7295cb4be59a29f59c6da8f473025bab23a6397317` |
| Context and holder alias | `ark-provenance-asm-0x180004190.json` | `ev_b6ba62238d04761edf10e6818715c59b7f570eb5aee7b20dca72a7bd49bfbe9a` |
| Open/option vtable slots | `ark-provenance-bytes-0x1801ef958-424.json` | `ev_e87e3c9fb4b7893647e9698b177ce22f0e8ec75aa322446a1362be728dc63f1e` |
| Actual option-copy width | `ark-provenance-asm-0x1800084a0.json` | `ev_c0a550573bd61f489f764e017788c72f2d6a4a9f1eb82e51f610867daab0c33a` |
| Enabled source capture | `ark-provenance-asm-0x180006460.json` | `ev_7b3121964ee1be07056ca6b2c7084cc2385e77852dbeba0a22ba5d030745e326` |
| Source-read failure/result | `ark-provenance-p-0x180006460.json` | `ev_e80af7208b1d47d58fa1dd655c70d73a3eb850ca9bcf8cdbaa2a752b2e341d4c` |
| Family context argument | `ark-provenance-p-0x180010315.json` | `ev_1f910863d6433c9a3194b103528ebe53c5baf9141e1fe7fa5f071abb111320b3` |
| ZIP constructor/base handoff | `ark-provenance-asm-0x180023d70.json` | `ev_c290db73111dc72f698b95a762bb7e3e12c660fe8dd320a2a03036b2839f74c7` |
| Extractor context retention | `ark-provenance-p-0x180057890.json` | `ev_158c98158ca14f30258779182568b594bd36b791163323d707b140e65850aaf6` |
| Member output/finalization | `ark-provenance-p-0x180077c80.json` | `ev_a7778a904d835e6a67d8b040943d37cd5e706cd6335311c6f2bc150d8535c0d3` |
| Factory preserves context | `ark-provenance-p-0x18001aec0.json` | `ev_4c00d5237de095addab8b44c0af6a8350e05790038d6463e49bcaf980bc3b54a` |
| Buffered output retains context | `ark-provenance-p-0x18001a9c0.json` | `ev_10cc678db643aa7d7f3dae9da805ac28039197c6df379d757d0939a48eb7b8d1` |
| Buffered consumer | `ark-provenance-p-0x180023620.json` | `ev_deef005e936c46e7ca136320149235e4092e76d10ae07fd684004097f96bd1c2` |
| Direct consumer | `ark-provenance-p-0x18005a6a0.json` | `ev_35dab75b48ae90fc1767d58510761308d0a50f44be8e811619e2f670f858f261` |
| Source ADS reader | `ark-provenance-p-0x18000d500.json` | `ev_f0e7a276580379e9407079ebeb4674e06d79bc9df7deb591b28c64c29d3e85a8` |
| Target filter/write/flush/delete | `ark-provenance-p-0x18000d6d0.json` | `ev_c0e3e04bd67ce7dedd29fda7d66abffe9322565844239e44cd3b1b178ea2e630` |

## Limitations

This is read-only Ghidra12.1.4 and supplementary Capstone5.0.7 reconstruction of shipped7.32.0.1 artifacts, not original source or a live ADS test. The option-slot's original C++ name remains stripped; its ABI binding and data flow are established. All format/path/case variants, filesystem ADS support, settings changes during an already-open archive and failure interleavings were not exercised. Only compiled artifacts/resources were read: no program or archive operation was executed, no live ADS payload, registry value, user configuration or credential was read, and no service, package or installed file was changed. The applicable-law/EULA boundary remains in [the reference note](../../../docs/reference-code/archive.md).
