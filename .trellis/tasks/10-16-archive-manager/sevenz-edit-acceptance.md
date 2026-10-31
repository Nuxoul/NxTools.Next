# Ordinary single-volume 7z editing — independent acceptance

2026-10-28. **Scoped source/automated/isolated-native acceptance: PASS. Overall archive/Phase E/A–F/release acceptance: PARTIAL.** This audit independently checked the attached delivery against current code and reran the bounded collections below. No new confirmed product defect was found in the inspected slice. The known native readmission failure was independently reproduced and remains OPEN; unexecuted fault/resource/crash/physical/user gates are not waived.

## 1. Scope and itemized decision

Canonical phase remains **Phase E — Format expansion and ZIP editing**. This accepts only the qualified ordinary single-volume7z route in [delivery outcomes](sevenz-edit-outcomes.md) and [bounded route](sevenz-edit-route.md), not TAR/RAR/volume editing, ZIP ReplaceContent, HZ3–HZ5 or product/release completion.

| Item | Independent decision and limits |
| --- | --- |
| Add regular files/directories/empty items | PASS at scoped worker/native boundaries. Original protected source IDs and CreationPlan; conflicts use original attempt-local ticket/apply-all. All-skipped warns with0commits, no rewrite. |
| Delete file/directory subtree/overlapping/all members | PASS. Original ordinals and component-ended descendants; overlap deduplicates, `a/` does not select `ab/`; all-delete produces independently readable empty7z. |
| ReplaceContent | PASS. Distinct typed replace_content, one existing regular ordinal + one protected source; zero-length source still opens/checks EOF/closes. Old member name/known metadata retained. Not Add or intermediate delete. ZIP replacement stays unavailable. |
| File and explicit/implicit directory subtree rename | PASS. Immutable original member identity, explicit source_prefix for selected implicit parent; all final paths/case/file-prefix/duplicate/self-descendant/stale checks precede encoding. |
| Format/state admission and fidelity | PASS for tested ordinary LZMA2 solid/non-solid, uniform AES and encrypted header. Strict original metadata allowlist prevents silent unknown-property loss; bounded repeated zero-only kDummy alignment is accepted, nonzero/unknown semantic fields refused. Supported old timestamps/attributes/kinds/content/empty directories and encryption are compared against fully decoded candidate; independent stock7Zip metadata oracle passed. Not exhaustive coder/filter/encryption acceptance. |
| Cost and UI/task lifecycle | PASS at TestWindow/worker/native-fixture level. Full non-solidLZMA2 reencode/coexistence warning, original menu/buttons/F2/Delete/forms/FIFO/generation/cancel/progress/result paths retained. Five fixed stages pass actual Rust→C#→Job event route; processed bytes are not saved bytes. No physical/visual approval implied. |
| Transaction and error precedence | PASS at tested boundaries. Full local and held-native readback verification precede actual old-reader closure and acknowledged edit_reads_complete. C# held source/target recheck, final cancel, FILE-only release and exactly one original native replacement; parents/candidate retained. Native5 failed-before-commit and64 unknown preserve their own facts; published late-cancel result survives. Approved path-publication TOCTOU is not ID-CAS. |

New added-file metadata is **not** full NTFS fidelity: original source contract supplies name/type/content and DOS-local time, not all UTC creation/access times/ACL/ADS. Compression bytes/method/layout/ratio change to disclosed non-solidLZMA2. Unsupported mixed/unpreservable encryption, special/anti/link/reparse/unknown metadata/SFX/tail/volume inputs remain fail-closed.

## 2. Independent source and artifact checks

Read the current checkpoint, original A–F plan, approved7z route, format contract, matching reference note/manifest and original failure history. Inspected strict raw/encoded header admission; ordinal/subtree planner; full old-stream consumption and typed first-error retention; local/held candidate verification; native source handoff and exact commit-failure/late-cancel precedence; original UI request/eligibility/progress routes. This was one bounded main-agent read-only inspection, not a delegated review loop.

- Delivery's48recorded source/test files matched before and after independent execution, including all45Rust paths. No product/test/formatter/manifest source edit was made by this audit.
- Five pinned reference repository commits and19recorded file hashes matched.
- Locked sevenz-rust2=0.23.0 registry source:77files exactly match the cached crate, whose SHA256 `0a4f883677093690e91fef8ae81fad8bce9e2c3a079b61054f05ce0d3ecf681e` matches Cargo.lock. No registry/vendor source patch was assumed. Initial absence of .cargo-checksum.json was not treated as verification; direct archive/source comparison supplied the evidence.
- Frozen worker and current target/debug worker both hash to **28AFDFEEC7BFC8E46C03800F08B2657F01D702B558C34DB21C550DD56D33A066**. Frozen file unchanged after native rerun. Commands reused valid incremental Cargo/MSBuild artifacts; this is not a clean-environment reproducibility or signed Release claim.
- Before this acceptance prepend, current checkpoint167161bytes retained delivery's original163934bytes exactly as suffix; both verified independently. Full pre-acceptance checkpoint snapshot retained for final prepend-only comparison.

## 3. Independently executed verification

Evidence root: `C:\Users\Administrator\.pi-desktop\scratch\70da3267-15fc-4e10-b17c-2d9c0bf62e48\sevenz-acceptance`.
Per-command JSON/logs, accounting-final.json, desktop-name-comparison.json, dependency/source/ref hashes, native-accounting.json and checkpoint suffix records retain scope and identity. Rust commands inherited jobs1/incremental0/RUSTFLAGS=-C codegen-units=256, --locked --offline, serial tests, natural-idle lane checks and full-operation timeouts. No peer process terminated or duplicate build started.

| Check actually rerun here | Result |
| --- | --- |
| Archive isolated-codecs --lib |35PASS/0FAIL/0ignored; includes strict header/plan and existing creation tests |
| Actual 7z edit safety + real-worker contracts |4+1PASS; overlap with unified count below |
| Archive exact lib+38integration targets |**236PASS/0FAIL/0ignored across39targets**; same delivery catalog. Explicitly filtered one100kperformance test, excluded three native coordinator chains; not full-suite acceptance. |
| Desktop archive filter |**319PASS/0FAIL/0ignored**; exact319names equal delivery, missing0/extra0. Protected HZ1 18/icon-layout8/WAC12 all retained. Independent extension_http target matched0tests and is not counted. |
| Core/IPC/Protocol --lib |**93/43/89PASS**,0FAIL/ignored. Delivery Core7integration checks were inspected in historical evidence, not rerun here. |
| Five-owner strict Clippy --tests -Dwarnings with isolated-codecs |PASS exit0 |
| cargo check --workspace --tests |PASS exit0 |
| C# tests-project build --no-restore |PASS,0warnings/0errors |
| Exact reviewed7z C# list→same run |**selected9/run9/pass9/skip0/fail0/not_run0**,3wire+6native; actual fixed worker, independent content/encryption/metadata oracles and phase chain |
| Known native readmission case list→same run |**selected1/run1/pass0/skip0/fail1/not_run0**; source_unavailable, unchanged failure |
|45path scoped rustfmt/diff |PASS; no formatter writes by audit |
| Spec/catalog/GPUI-release/editor/test-selection gates |PASS (spec115docs/14owners; historical catalog79/78; exact carried vendor/editor deltas). Source checks only. |

Counts overlap and must not be summed. Original broader56compatibility logs were independently recounted:55PASS/1FAIL; this audit **did not rerun the other55cases**. Its fresh native execution consists of9sevenzedits and1known failure, not a new56case all-green result.

### Harness/environment corrections retained, not product failures

First native attempt incorrectly launched `dotnet Tests.dll`; owned-package protection correctly requires the exact Tests.exe apphost and rejected before worker admission (`Invalid owned package`). Initial wrapper stderr handling also prevented complete summary retention. Logs retained; scratch wrapper corrected to apphost and merged native output, then exact LIST/RUN9 passed. No product guard weakened.

First Archive unified run stopped with145PASS/2FAIL at TAR-stream tests because explicit NXTOOLS_TAR_STREAM_FIXTURES was missing; subsequent targets/layers/workspace did not run in that chain. Read actual fixture contract, hash-verified and copied delivery's tiny synthetic fixtures into this session scratch, supplied explicit path, and reran the identical39target catalog to236PASS. No test/budget/assertion changed; original failed log remains.

## 4. Remaining blockers / non-accepted boundaries

1. **Known readmission failure OPEN:** `archive job fixture actual readmission shortfall no output` freshly returns source_unavailable before expected resource_limit/OutputShortfall. Prior independent acceptance already attributes the path to encoded-source admission vs1byte output allowance and native mapping. This audit inspected the same chain and reproduced the result; no budget or source policy was relaxed. Not a new7zedit regression, but broader native coverage is not green.
2. **Three native coordinator chains NOT ACCEPTED:** rar_readmission_chain, sevenz_readmission_chain, tar_stream_readmission_chain were not run to acceptance here. Passing direct contracts does not replace coordinator/process evidence.
3. **Edit-specific finish/flush injection NOT RUN:** code checks writer.finish/flush and complete candidate/readback/native flush; existing creation fault tests pass, but no direct local edit encoder finish/flush storage failure was injected. This is an open verification item, not proof of an implementation failure or full SAFE05 closure.
4. Exhaustive coder/filter/encryption cross-product, huge archives/disk-fill/commit-memory stress, real child hard-kill/restart/power-loss recovery and full added-source metadata fidelity remain unverified.
5. Historical CLI helper_untrusted remains unrerun/unaccepted; FB-AC05 two-line screenshot complaint remains OPEN. No real Explorer target/physical keyboard-pointer/IME/DPI/Light-Dark screenshot/UIA/user visual, installed protected tuple, signing/package/update/service acceptance.

## 5. Final disposition / preservation

**Accept this qualified single-volume7z edit implementation at source, scoped automated and isolated-native evidence levels. Keep overall archive/Phase E/A–F/first-release acceptance PARTIAL.** No new confirmed in-scope defect requiring a source fix was found; open items above remain explicit and are not converted into PASS.

Repository writes by this audit are only this acceptance document and a prepend to the sole checkpoint. Old outcomes/failed logs/plans/task metadata/product/test sources remain intact. No package/sign/install/product/Explorer/Registry/service/ACL/UserChoice/source-recycle/commit/push/delegation operation, no next-phase or TAR/HZ3–HZ5 implementation. Final source and exact-checkpoint-suffix verification recorded after the documentation write.
