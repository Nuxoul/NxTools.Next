# Bandizip 7.32.0.1 — Shell archive delivery (static reference)

## Mechanisms

### Custom/date-generated Shell item → exact output path

`bdzshl.x64.dll` connects the rendered rule item to the compression destination rather than using the expansion only as a menu label:

1. `FUN_18001a278` calls `FUN_180014b1c` for the configured date/name rule. The expanded value is cached as before, then stored in the Shell owner field at `+0xad8`; the inserted menu action is command `0xbc9` [S1,S2; X1].
2. `InvokeCommand → FUN_180016d1c` maps `0xbc9` back to `owner+0xad8`. The command owner keeps the destination base at `owner+0xa70` [S3; X1].
3. `FUN_18001f92c` removes a redundant leading separator from the generated component, tests whether the base already ends in the separator, and returns either `base + component` or `base + "\\" + component` [S4]. The command is formatted as `c <quoted-output>` and every selected path is appended through the existing common launch/MMF path [S3; X1].
4. In `Bandizip.exe`, `FUN_1400dbca0 → FUN_1400db820 → FUN_1400d7a70` parses command `c` as operation `0x0b`, retains the first positional value as the output pathname at descriptor `+0x08`, and retains the source list [E1,E2]. Window bootstrap `FUN_140145640` invokes CLI operation dispatcher `FUN_140159f90` before ordinary main-window setup; the GUI `WM_COMMAND` dispatcher `FUN_14013fc40` is not the receiver for this Shell command [E3,E4,E9].
5. `FUN_140159f90` case `0x0b` calls `FUN_14013d910`. The coordinator copies descriptor `+0x08` into its ordinary-create output string and into the task record consumed by `CArchivingDlg`/the worker [E4,E5].

The actual creation destination for the custom rule item is therefore the Shell-derived base folder at DLL owner `+0xa70` joined with the exact `FUN_180014b1c` expansion retained at `+0xad8`. It is not recomputed by the EXE and is not merely presentation text.

### Compress and email → EXE flag → MAPI handoff

The email item uses the same destination join with the ordinary ZIP candidate:

- DLL command `0xbbd` joins `owner+0xa70` with the standard ZIP name at `owner+0xa78`, quotes the full path, and emits `c -email <output> <selected paths...>` [S3,S4; X1].
- EXE instruction flow matches `-email` and writes descriptor `+0xdc`. The neighboring `+0xe4` flag is `-delsrc`; the serializer independently emits `-email ` from `+0xdc` and `-delsrc ` from `+0xe4` [E1; X2,X3].
- `FUN_140159f90` case `0x0b` runs `FUN_14013d910`, then tests descriptor `+0xdc` and passes descriptor `+0x08`—the same created-output pathname—to `FUN_140122120` [E4,E5,E6; X2].

There is **no successful-create gate before the mail handoff**. The coordinator return is already in `EBX`, with terminal values selected from progress-dialog state after the modal operation, but the caller does not test that return before calling `FUN_140122120`; it stores the coordinator result only after the MAPI call. The only preceding branch is the coordinator's auxiliary format/output parameter, which is zero for this ordinary ZIP path [E4,E5; X2]. A canceled or failed compression can therefore still invoke the mail provider with the intended pathname. If no file was published, the provider receives a nonexistent attachment path; if an older file already occupies that path, it can receive that stale file.

### Attachment shape, mail-client UI, result and lifetime

`FUN_140122120` performs a Simple MAPI handoff [E6,E7,E8; A1]:

- An ordinary pathname reaches the ANSI conversion fallback `0x1400e3a90`, using `WideCharToMultiByte` code page3 (`CP_THREAD_ACP`). The preceding `0x140029850` path handles integer string-resource IDs, not ordinary pathname pointers. Conversion has an independent unchecked result described below. [E7,X4,A2]
- It builds one `MapiFileDesc`: reserved/flags zero, `nPosition = -1`, `lpszPathName = converted output path`, optional filename null, file type null.
- It builds one otherwise-empty `MapiMessage`: no subject, body, recipients or originator; `nFileCount = 1`; `lpFiles` points to that descriptor.
- It loads `mapi32.dll` with the system-DLL search flag, resolves ANSI `MAPISendMail`, and calls it with null session/UI handle, the message, flags `9`, and reserved zero. Public constants make `9` equal to `MAPI_LOGON_UI | MAPI_DIALOG`, so the boundary requests the configured Simple MAPI provider's send-note UI. This is a mail-client draft/compose handoff, not evidence that mail was sent.
- `MAPISendMail == SUCCESS_SUCCESS (0)` makes the helper return `1`; every nonzero result makes it return `0`. The caller discards that helper result. `MAPI_E_USER_ABORT (1)`, missing/open-failed attachment, provider absence, login failure and other native failures therefore have no distinct application outcome and do not replace the already recorded compression result [E4,E6; A1].

The attached archive is not a temporary mail artifact. It is the normal Shell-selected destination, and neither `FUN_140122120` nor its caller deletes it after MAPI returns. The helper releases its temporary string storage, but the archive remains at the created output path. No post-handoff temp-file cleanup exists on this route.

### Path conversion result

The fallback first queries the required byte count with `cchWideChar=-1` and output capacity0. It computes `N-1`, then calls the same API with the still-null-terminated input (`cchWideChar=-1`) but declared output capacity `N-1`. Assembly at `0x1400e3b20` supplies that reduced capacity; the call at `0x1400e3b2c` is followed by string bookkeeping without checking its return. The helper sets the string length and writes its own terminating zero. Microsoft's contract says that the measured `N` includes the terminator, and that conversion returns zero on failure. The supplied API capacity is one byte short of that measured terminated size, and the conversion result is discarded. Manual termination does not confirm a complete conversion. This is a concrete conversion-result defect, separate from compression or MAPI success. [E7,X4,A2]

### Cancellation and terminal boundaries

| Boundary | Static behavior |
| --- | --- |
| Shell menu dismissed | No `InvokeCommand`, EXE launch or archive creation. |
| Compression canceled/failed | Coordinator returns terminal state, but email handoff is not gated on that state; the intended path is still offered to MAPI. |
| MAPI UI canceled | Public result is `MAPI_E_USER_ABORT`; helper returns false, caller ignores it. |
| MAPI load/export/provider failure | Helper returns false, caller ignores it. |
| MAPI returns zero | Only the Simple MAPI handoff is accepted at this boundary; message transmission is not established. |
| Mail UI closed | The archive is retained at its ordinary output destination; no deletion is attempted. |
| Path conversion does not succeed | The second conversion return is not checked before constructing the attachment descriptor; no distinct conversion outcome is published. |

## Evidence

Canonical REA/Ghidra records identify two separate x64 subjects and address spaces.

| Ref | Canonical filename under `evidence/raw/` | Actual `evidence_id` | Subject / boundary |
| --- | --- | --- | --- |
| S1 | `bdzshl-viewcontrols-follow1-procedure_pseudo_code-0x18001a2a0.json` | `ev_12571bdba6d62d8a467ebd07140e9be182245dd72d0c51dddb676cfd795a4e22` | `D:\Tool\Bandizip\bdzshl.x64.dll`; menu builder, cached custom rule and command `0xbc9` |
| S2 | `bdzshl-viewcontrols-follow2-procedure_pseudo_code-0x180014cb8.json` | `ev_7d3b7ff838daa9f1defd81cb4105ff11fe118fb248ffbb572bafb8619a0f0325` | Shell DLL; `FUN_180014b1c` template/date execution and sanitization |
| S3 | `bandizip-browser-bdzshl-code-0x1800172df.json` | `ev_b1c50768f6743f14816c9457e35dbe42f69299258bb8fd435884f4b32ae082a6` | Shell DLL; `FUN_180016d1c` custom/email commands, selection append and common launch |
| S4 | `bdzshl-delivery-main-procedure_pseudo_code-0x18001f92c.json` | `ev_61b6de56251d4aba4fbe4ad1e4f0560bf47c162deb48cd14743df80820944757` | Shell DLL; separator-aware destination join |
| E1 | `bandizip-browser-code-0x1400d7a70.json` | `ev_17598dbe35fa71507923a1c4122869d3fd40db2c1b5fc4da735c4101d41942c1` | `D:\Tool\Bandizip\Bandizip.exe`; command/option parser |
| E2 | `bandizip-delivery-receiver-analyze_function-0x1400dbd03.json` | `ev_f62d948e10a5ecf323a47f339f263bd7cd0974dc96b90adc0200fc0b9ea1f7a5` | EXE; raw command wrapper `FUN_1400dbca0 → FUN_1400db820` |
| E3 | `bandizip-viewcontrols-follow6-procedure_pseudo_code-0x140145640.json` | `ev_6b08d88b89800889cfcf23392ff60d04718970e0d9cffa6bb575035bf5b0cc6e` | EXE; bootstrap invokes `FUN_140159f90` before ordinary window setup |
| E4 | `bandizip-delivery-final-procedure_pseudo_code-0x140159f90.json` | `ev_d03f8fe6f1cff9c5f0ff6fa44549cf32842594bf240dd094e2afc4ef4cf4f2c7` | EXE; operation `0x0b`, coordinator return, email flag and MAPI call order |
| E5 | `bandizip-create-p-0x14013d910.json` | `ev_8d959a85156d95878d749c29b64d963bcb4e3dd6a56d603a777d919c722758e8` | EXE; output/task planning, progress dialog and terminal return selection |
| E6 | `bandizip-delivery-final-procedure_pseudo_code-0x140122120.json` | `ev_beaf8b069dcf79a30710fbebae72e97d3bc467d0681b1e736b6824500762158e` | EXE; one-attachment Simple MAPI call and collapsed result |
| E7 | `bandizip-delivery-encoding-procedure_pseudo_code-0x1400e3a90.json` | `ev_d0b944f4d1df76ede74c6fa2e89d635f6947096b9c6d0b6f035abb408991266f` | EXE; UTF-16 path to multibyte attachment path |
| E8 | `bandizip-delivery-mail-analyze_function-0x14012220d.json` | `ev_2d3e56699e365940a14fa010df523ba8128ef20a3f87fbf8bd1d59050d8a494e` | EXE; MAPI helper body, sole code caller, dynamic APIs and literal references |
| E9 | `bandizip-delivery-main-procedure_pseudo_code-0x14013fc40.json` | `ev_af0de4df693ca06880deffc88fc698200766db86e8c6af442d25cc168f938462` | EXE; GUI `WM_COMMAND` dispatcher, distinct from CLI operation receiver |

Supplemental read-only evidence has no REA ID:

| Ref | File | Boundary |
| --- | --- | --- |
| X1 | `bdzshl-delivery-static-routing.txt` | `0xbc9 → +0xad8`, base `+0xa70`, email `+0xa78`, join calls, custom-rule insertion |
| X2 | `bandizip-delivery-static-gates.txt` | exact `-email/+0xdc`, receiver call order, coordinator terminal values and MAPI call assembly |
| X3 | `bandizip-delivery-static-literals.txt` | resolved command, option, DLL and API literals at the cited instructions |
| X4 | `bandizip-delivery-static-path-conversion.txt` | Actual resource shortcut, measured byte count, reduced conversion capacity, ignored return and manual string termination |
| A1 | `bandizip-delivery-mapi-contract.txt` | Windows SDK 10.0.26100.0 `MapiFileDesc`, `MapiMessage`, flags, signature and result codes |
| A2 | `bandizip-reference-path-conversion-api.json` | Public `WideCharToMultiByte` terminator, capacity and return-value contract |

## Replay

REA replay uses `evidence/raw/tools/rea_follow_session.py` with the private target-matching snapshots `bdzshl-delivery-session-snapshot.json` and `bandizip-delivery-session-snapshot.json`. Run recipes one at a time with `GHIDRA_HEADLESS_MAXMEM=768M`, `REA_GHIDRA_STARTUP_TIMEOUT_MS=900000` and a distinct `REA_SESSION_LABEL`:

- `recipes/rea-bdzshl-delivery-main.json`
- `recipes/rea-bandizip-delivery-main.json`
- `recipes/rea-bandizip-delivery-mail.json`
- `recipes/rea-bandizip-delivery-encoding.json`
- `recipes/rea-bandizip-delivery-final.json`
- `recipes/rea-bandizip-delivery-receiver.json`
- `recipes/rea-bandizip-delivery-entry-xrefs.json`

Each REA recipe closes its session. Static slices replay through `evidence/raw/tools/inspect-pe-static.py` against the shipped DLL/EXE using the exact ranges/functions/literals in their headers. X4 uses `function:0x1400e3a90 function:0x140029850`. A1 is from the named local Windows SDK header; A2 retains [Microsoft's conversion contract](https://learn.microsoft.com/en-us/windows/win32/api/stringapiset/nf-stringapiset-widechartomultibyte).

## Limitations

- This is static reconstruction of `Bandizip.exe` and `bdzshl.x64.dll`, not original source, runtime/Explorer acceptance or observation of a mail client. No target, archive, registry, live configuration, credentials, logs, service, mail client or network was exercised.
- The Shell DLL's long-command MMF path is reused from the established evidence; this report does not repeat its event/lifetime analysis.
- The exact top-level type name for the parsed command descriptor remains unrecovered. The parser wrapper, bootstrap argument and operation receiver are connected by callsites and shared field offsets, but no source-level type declaration exists.
- Simple MAPI provider choice, whether a client displays a draft, and provider-specific treatment of missing/stale attachments require runtime acceptance. Static code proves the requested UI handoff and its result handling only.
- Path conversion is an ANSI/thread-codepage boundary with an unchecked, under-declared output capacity. Actual partial-buffer contents, filename coverage and resulting provider behavior were not exercised; this is not a claim that every attachment fails.
- No send result exists beyond the synchronous `MAPISendMail` return, and that return is discarded by the caller. Nothing here establishes that a message was transmitted.
- Bandizip EULA §3/applicable-law limits remain controlling; no proprietary implementation is proposed for reuse.