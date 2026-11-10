# Bandizip SFX active cancellation: static artifact trace

## Boundary

This trace is limited to the actual shipped x86 `D:\Tool\Bandizip\bdzsfx.x86.sfx` (344,064 bytes). No version is inferred from `Bandizip.exe` or any DLL. Addresses below belong only to this x86 stub. Ghidra 12.1.4 reconstructions and the retained Capstone slices are static observations, not original source or observed execution. The EULA §3/applicable-law boundary applies; no proprietary implementation is reused.

## Joined active-cancel path

### UI flag to extractor event

The command dispatcher `0x004175ff` keeps the already-recorded split: command `2` calls `EndDialog(...,2)` only while `this+0x30 == 0`; while extraction is active it writes `this+0x38 = 1` and returns to the nested message dispatch.

The extraction UI object also owns an event-interface subobject at dialog `+0x20`. During initialization, `0x00417b9f` passes `dialog+0x20` through the extractor primary vtable slot `+0x24`; the released derived table maps that slot to `0x0040b1d9`, which stores the interface at extractor `+0x08`. The event table at `0x004438c8` maps `+0x04` to `0x0041825f` and `+0x0c` to `0x00418328`.

Both event callbacks consume the active-cancel field because their `this` is `dialog+0x20`, so callback `+0x18` is dialog `+0x38`:

- `0x0041825f` pumps pending Win32 messages every 200 ms through `0x004183da`, then writes `1` to its cancel out-parameter when callback `+0x18` is nonzero.
- `0x00418328` updates progress, writes `1` to its cancel out-parameter when callback `+0x18` is nonzero, and pumps messages through the same helper. `0x004183da` uses `PeekMessageW(PM_REMOVE)`, `TranslateMessage`, and `DispatchMessageW`, which is what allows command `2` to run during the otherwise synchronous extraction call.

The extractor event subobject table at `0x00442f08` maps `+0x0c` to `0x0040a6c1`. That method calls the attached dialog event at vtable `+0x0c` with extractor status fields. Once either published status is nonzero, it writes extractor result `0xa0000017` at primary-object `+0x27c`. The selected-member routine `0x0040b2ac` stops its loop when member helper `0x0040b4e8` returns zero; wrapper `0x0040b234` consequently returns failure to `0x00418017`.

### Current output close and delete policy

`0x0040a6c1` contains the current-output cancellation branch. After setting `0xa0000017`, it calls the supplied current output's vtable `+0x24` with argument `1` only when extraction context `+0x38` is enabled and a current output pointer exists. The direct output table at `0x00442e90` maps `+0x24` to `0x00410511`. That method finalizes state, calls `CloseHandle`, and calls `DeleteFileW` only when its argument is nonzero and a pathname was retained.

The shipped SFX context initializer `0x0040eb5e` writes context `+0x38 = 0`; no later enabling write is established in the inspected construction/open/extraction chain. The ordinary decoder-failure tail at `0x0040b938` calls the same output slot with argument `0`, so current-output handle close is connected, but cancellation-driven `DeleteFileW` is **not** established for this shipped stub. No rollback of earlier completed members is present in this path.

### Terminal UI

`0x00418017` treats extractor result `0xa0000017` as a quiet non-success: absent its separate `this+0x74` error marker, it does not show `ERR_ERROR_OCCURRED`. It cannot enter the success-only `RUN` call or the success completion/autoclose branch. It then calls `0x00417f72(...,1)`, clears active state `this+0x30 = 0`, and releases the destination string.

`0x00417f72(...,1)` re-enables destination edit `1000`, Select Folder `0x3e9`, Extract `0x3ea`, and Overwrite `0x3eb`, and changes command `2` from localized `BTN_CANCEL` back to `BTN_CLOSE`. Active cancellation therefore returns to the still-open extraction dialog; it does not call `EndDialog` and does not launch `RUN`.

## RUN process boundary

The exact process-creation API is dynamically resolved rather than present as a normal PE import. Initializer `0x00414c2c` loads `kernel32.dll`, resolves `CreateProcessW` with `GetProcAddress`, and stores it in the first function-pointer slot consumed by `0x00414cde`. It separately loads `Shell32.dll` and resolves `ShellExecuteW` for the existing fallback path. The primary `RUN` attempt is therefore `CreateProcessW`; fallback is `ShellExecuteW` with verb `open`.

## Joined mechanism

`WM_COMMAND/2` while active → dialog `+0x38 = 1` → progress callback pumps/dispatches messages and publishes cancel `1` → extractor event method records `0xa0000017` → current decoder terminates and closes the current output with argument `0` on the joined default path → selected extraction returns failure → no `RUN`, no completion/autoclose, no ordinary error dialog → controls are re-enabled and Cancel becomes Close.

## Remaining precise edge

The remaining policy edge is enabling extraction context `+0x38` inside this SFX. The close-with-delete implementation and conditional `close(1)` call are exact, but the stub initializes the policy to zero and no later enabling write is established. The connected path closes the current output without proving deletion of its partial pathname after active cancellation.

## Canonical evidence

Every JSON subject below is exactly `D:\Tool\Bandizip\bdzsfx.x86.sfx` (x86), provider `ghidra` / Ghidra 12.1.4. Paths are relative to `evidence/raw/`.

| Boundary | Canonical JSON file | Evidence ID | Subject |
| --- | --- | --- | --- |
| Active/idle Cancel dispatch and flag write | `sfx-advanced-controls-procedure_pseudo_code-0x0041762b.json` | `ev_bd6aa0393883699b7963dc32e0e0c8fc31b7730271b13c20057b0886c34e8dcc` | `D:\Tool\Bandizip\bdzsfx.x86.sfx` |
| Extract result, quiet cancel, RUN success gate and terminal return | `sfx-advanced-execute-procedure_pseudo_code-0x00418017.json` | `ev_9d07d5af61fbd79e907b1043f35f9d435452e9e50c81a9083f35ee8c42ef3d99` | `D:\Tool\Bandizip\bdzsfx.x86.sfx` |
| Selected extraction wrapper | `sfx-advanced-run-procedure_pseudo_code-0x0040b234.json` | `ev_421783117a5ac5e7eccfa76ab81859199f4c53c1628e4a090ce3fe89ce964037` | `D:\Tool\Bandizip\bdzsfx.x86.sfx` |
| Extractor construction / retained context pointer | `sfx-cancel-extractor-constructor-analyze_function-0x0040a251.json` | `ev_37dde0f85d2600ac704ac0ada4c1f4ec6a10a0c84625c374bf6ad6d2ebcfeffb` | `D:\Tool\Bandizip\bdzsfx.x86.sfx` |
| Selected-member dispatch and member-stop edge | `sfx-cancel-extractor-dispatch-analyze_function-0x0040b2ac.json` | `ev_f9fb25b3d26231c20d27cce3fd1fc2b08dbae93d8e1c25515ddd51ff141dbdbc` | `D:\Tool\Bandizip\bdzsfx.x86.sfx` |
| Member output acquisition/failure cleanup | `sfx-cancel-member-cleanup-analyze_function-0x0040b746.json` | `ev_15d0c93b24a86dd364489856c27ec889e9b3671be20323c9a00bfe39782c51d5` | `D:\Tool\Bandizip\bdzsfx.x86.sfx` |
| Direct output close and conditional `DeleteFileW` | `sfx-cancel-output-close-delete-analyze_function-0x004105f9.json` | `ev_e4736e01c44fd1d6a3f5026e82dd845327b3f7f2b3b488b5baa27e634b128d12` | `D:\Tool\Bandizip\bdzsfx.x86.sfx` |
| Nested progress message pump | `sfx-cancel-message-pump-analyze_function-0x004183da.json` | `ev_c1c19c5c6e3dda0e3a8dd64bc5cd408ec450c54f9422022f896cd5548175995b` | `D:\Tool\Bandizip\bdzsfx.x86.sfx` |
| Terminal control enablement / Cancel-to-Close label | `sfx-cancel-terminal-ui-state-analyze_function-0x00417f72.json` | `ev_a7431e420e342ff1ee852cf6753154b2b930f04f155b4bec93e4a7ae3ee384b6` | `D:\Tool\Bandizip\bdzsfx.x86.sfx` |
| Dynamic `CreateProcessW` / `ShellExecuteW` resolver | `sfx-cancel-run-import-loader-analyze_function-0x00414cba.json` | `ev_3f823a07582b7002b5bef1f4250b3817a8d968b4ff91a9a280115966279209e1` | `D:\Tool\Bandizip\bdzsfx.x86.sfx` |

Supplementary `evidence/raw/sfx-cancel-static-instructions.txt` retains the exact interface attachments, released vtable slots, flag consumers, `0x0040a6c1` cancellation tail, failure `close(0)`, direct close/delete instructions and context initializer. It has no REA Evidence ID and is not runtime or source evidence.

## Replay and limits

`evidence/raw/sfx-cancel-session-snapshot.json` retains the independent x86 analysis cache; `evidence/raw/recipes/rea-sfx-cancel-followup.json` records its target-matching replay parameters. Individual canonical JSON and the static instruction slices remain the observations, rather than the cache alone.

No SFX, Bandizip, Arkview, Amsiman, Explorer, GUI or archive was executed. No registry, live configuration, credentials, user logs or services were read or changed. Extraction responsiveness, partial-file removal and native launch outcomes have not been exercised; no product code, build, package or installed binary was changed.
