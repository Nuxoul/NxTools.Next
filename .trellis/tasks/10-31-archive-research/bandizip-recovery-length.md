# Bandizip recovery length: provider output and host boundary

## Scope and result

The shipped x64 `Bandizip.exe` and separate `data\lm.x64.dll` at `D:\Tool\Bandizip` were inspected statically. The latter is447,544bytes and reports file/product version7.32.0.1. Ghidra12.1.4 reconstructions and PE instruction slices are not original source or recovery execution. Bandizip EULA§3/applicable-law limits remain in [the reference note](../../../docs/reference-code/archive.md); no proprietary implementation reuse is proposed.

The recovery length ceiling is **an output of the loaded capability provider**, copied into the worker at Start. The worker's constructor value10,000 is therefore a default, not proof of the effective Standard or paid limit. The advertised Standard-four-character value and exact per-candidate enforcement remain unestablished by these static records.

## Provider to window to worker

1. `CRecoveryMainDlg` constructor `0x140175550` creates the provider holder through `0x14012e490` and retains it at dialog `+0xb20`; it initializes dialog length field `+0x90` to zero. The embedded worker begins at dialog `+0x98`. [R1,R2]
2. Loader `0x14012e650` loads the `lm.x64.dll` module, resolves export `lm`, calls its ABI selector `0x20201021`, and stores the returned interface at holder `+0x10`. The separately observed DLL export constructs a `CLmDll` object. This connects the actual factory/subject rather than treating an EXE address as DLL code. [R3,L1]
3. During `WM_INITDIALOG`, `0x1401762a0` calls `0x14017a350`. After the Ark loader boundary is admitted, that routine invokes provider virtual `+0x38` with the dialog's Ark-loader object and **the address of dialog `+0x90`**. It stores the returned recovery interface at dialog `+0x88`; a null result makes initialization fail. The length field is an output of this provider operation, not the visible thread preference. [R4,R5]
4. Start `0x140178680` separately selects thread count at dialog `+0x588`. It then copies dialog `+0x90` to `+0x5ac`, which is embedded worker `+0x514`, before starting that worker. The provider-supplied value thus replaces the constructor default for this operation. [R6]

The provider vtable at `lm.x64.dll!0x180038e78` binds virtual `+0x38` to entry `0x180009bc0`. Its retained reconstructed views are thunks/branch-fragmented entries, not a decoded maximum-length setter. Original entry bytes begin with a jump to `0x180048d2e`. These observations identify where the numeric policy is supplied, but do not establish a Standard value4, paid value, or the provider's complete candidate-test contract. [L2–L4,S1]

## Host consumption is batch-oriented

Outer plan `0x14017e4b0` uses the operation's worker `+0x514` in a byte-length comparison against its current candidate buffer. After receiving a worker completion, it copies the next buffer into `+0x518`, calls batch submission `0x14017efe0`, and then checks whether `lstrlenA` exceeds the retained ceiling. The over-limit branch sets its stop/exhaustion state after a bounded sleep. [R7]

`0x14017efe0` copies range/count fields into the subworker and signals its work event; it does not itself compare against worker `+0x514`. The generic subworker path in `0x14017f340` forwards a candidate to its retained Ark test interface. This connects the host's scheduling/stop boundary, but does not prove that every internally tested candidate is screened by the same length check. Specialized format paths and the provider/test interface may impose separate constraints. [R8,R9]

Accordingly, the resource phrase “up to four characters,” the thread-count edition branch, the constructor default, the provider output and the host's byte-length stop are different facts. The [control report](bandizip-password-controls.md) remains the owner of pause/resume, explicit stop and UI terminal states.

## Evidence locator

JSON names are relative to `evidence/raw/`. `EXE` means `D:\Tool\Bandizip\Bandizip.exe`; `LM` means `D:\Tool\Bandizip\data\lm.x64.dll`. Their image-base addresses are not interchangeable.

| Ref | Canonical JSON | Actual Evidence ID | Subject |
| --- | --- | --- | --- |
| R1 | `bandizip-recoverylimit-a-0x140175550.json` | `ev_cbb678bc7ad82756eb73fab2db25a516ec84f493274b8b5eef1ac210a429e5f2` | EXE |
| R2 | `bandizip-recoverylimit-p-0x14012e490.json` | `ev_ca9d0322a314302a3e1078ebdbe56fd9bc6684e856f2342d85763298a67e2126` | EXE |
| R3 | `bandizip-recoverylimit-p-0x14012e650.json` | `ev_c1f748049f83651fb1deffd82174bc59c67e463baa5de3fe00a47ca21cb46dcb` | EXE |
| R4 | `bandizip-tools-analysis-0x1401762a0.json` | `ev_64ed0445345b0eae1eb721c3c88329618414b72465bcf4de98848580dd1f6e04` | EXE |
| R5 | `bandizip-passcontrols-code-0x14017a350.json` | `ev_a377e6ee3c759464e0b15268a1849ef7f21052997d015d7d655514b1a9897e38` | EXE |
| R6 | `bandizip-passcontrols-code-0x140178680.json` | `ev_a90751c598bed713a0ebbca496c7982adf7178abf44cba6de2797d07976a6a40` | EXE |
| R7 | `bandizip-passcontrols-code-0x14017e4b0.json` | `ev_421939b16ed6754ab0a7e36a9e0c9714f26de5534b6af178c3e701246b4c7c2d` | EXE |
| R8 | `bandizip-recoverylimit-p-0x14017efe0.json` | `ev_e9800a5a204eb4d6e8198a6425eed27d02a0e9a240d121dbf8ea20067a1282e5` | EXE |
| R9 | `bandizip-passcontrols-code-0x14017f340.json` | `ev_0c9ff4de7812d836ed40a3216c03da66b6d91a9bf87de5dc8d2549f95f2dda17` | EXE |
| L1 | `lm-recoverylimit-a-0x180009680.json` | `ev_b058c4ce14782a3e9f8d64d11a82ab9ca6aff54e950c6857a0a3dca4fbb82242` | LM |
| L2 | `lm-recoverylimit-b-0x180038e78-64.json` | `ev_b3f926c6d52e6678fdbc45c1c212828e9baa879e1142d7cd805bac1a29677556` | LM |
| L3 | `lm-recoverylimit-p-0x180009bc0.json` | `ev_a645a71e4d89c6cca18b55ba37f5bcd67d61669410119ac28c815d3950d845d3` | LM |
| L4 | `lm-recoverylimit-p-0x18004ddb7.json` | `ev_481f599e88af34ad19e72ce4ab9e6013622f120b1f2f39e35353ade8f4ca76d2` | LM |

**S1:** `lm-recoverylimit-static-entry.json` retains bounded source-byte instruction entries and the independent module's observed size/version. It has no REA Evidence ID and makes no complete-function or runtime assertion.

## Replay and limitations

Private `bandizip-recoverylimit-session-snapshot.json` and `lm-recoverylimit-session-snapshot.json` match their separate subjects. The `recipes/rea-bandizip-recoverylimit-*.json` and `recipes/rea-lm-recoverylimit-*.json` retain canonical query parameters. Individual records, rather than snapshots alone, are the observations.

The Standard/paid numeric output and exact effective candidate boundary remain unresolved. No password was recovered or tested; no live credential, stored preference, registry value or activation state was read. No target program, DLL interface or GUI was executed, and no product logic, package, service, dependency or installed binary was changed.
