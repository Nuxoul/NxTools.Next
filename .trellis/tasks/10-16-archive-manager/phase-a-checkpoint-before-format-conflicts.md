# Phase A — Engine contracts and safety foundation: partial checkpoint

Date:2026-10-20. **Phase A in progress, NOT passed. Only this file owns current state.**
Canonical names/order stay in [implement.md](implement.md). No Phase B/A–F completion.

## Current checkpoint — C1/C2/C3/C5 small interoperability closure

**四项 scoped checks PASS；原实现通过，仅补齐断言/证据，未修改生产代码。
Independent acceptance pending; NOT Phase A/A–F completion.**
[Combined evidence, exact commands/oracles/counts](phase-a-small-interop-evidence.md).

| Item | Actual delivered status |
| --- | --- |
| C1 empty Gzip/XZ creation |PASS: zero-byte ordinary source, normal level; original runner/Job/actualworker/native create, independent CPython gzip/lzma empty decode, existing open/test/extract1file0bytes, exact empty native file and cleanup.8operations. |
| C2 actual7z x86BCJ+LZMA2 |PASS: existing7-Zip26.00 producer, saved commands/version/hash/actual coder proof;155byte archive/4096byte independent input. Original runner/Job/worker test+extract exact bytes/counts/cleanup. |
| C3 signature/ZIP aliases |PASS: unchanged independent ZIP.bin/.jar/.apk -> Zip and3files275bytes verified;7small random/damaged-signature/truncated/misleading compound cases retain typed UnsupportedFormat/CorruptArchive. No JAR/APK write/signature-authentication authority. |
| C5 solid optional metadata |PASS: original runner/Job/worker opens existing solid/non-solid fixtures; shared solid member packed_size=null; non-solid1446/1232match independent7Zip. Missing/null optional fields remain unknown, not0; no extra listing decode. |

- Exact Rust tests3PASS: new alias --exact,new optionalmetadata --exact,existing creationreadback
  (4normal/corrupt scenarios). Engine100k test was filtered, NOT run. Existing bounded7z working/
  header damage regression reused, not duplicated matrix. No production repair/red→green claimed.
- C#apphost0warnings/errors first; matchingisolatedworker after strictClippyisolated/default and
  archivefmt. C#exact LIST=RUN=PASS5,skip/fail/not_run0. Spec/context/selector/diff and fixture/
  production/history hashes verified. No fullworkspace/runtime/performance acceptance claim.

## Accepted prior scope / remaining work

- [B1/deliveredC4 independent PASS](zip-encoding-acceptance.md).
- [B2 password-only independent PASS](password-operation-acceptance.md).
- [B2 ZIP conflict independent PASS](zip-conflict-acceptance.md).
- [B3 original-budget ZIP risk independent PASS](zip-risk-acceptance.md).
- **[Latest ZIP preflight output-budget readmission independent PASS](zip-output-readmission-acceptance.md)**:
  explicit finite output-only approval -> one fresh cleanup-bound original admission. Old Risk,
  actual limits/source/publication and unchanged Job protections remain; not Desktop/UI integration.
- Other-format interaction/risk,fullB2/B3/SAFE04 and canonical cross-stage/native dependencies
  remain unfinished. No phase renumbering,exit-ledger rewrite or dependency waiver.
  NativeRecycle remains native_protection_unproven/NotAttempted.

## Preserved history and stop

[Prior checkpoint exact snapshot](phase-a-checkpoint-before-small-interop.md), SHA256
494A1AE731902F2092DAC6E3D31ADAF8627829AE8D0492052818CB4AD27AD485.
Historical reports/results/acceptances/ledger and Failed/Stopped facts remain byte-identical to
this slice's opening hashes. Production Rust/native code unchanged. Existing native-contracts
35497byte context warning retained. Only this file owns current state.

No100k/longpathmatrix/1GiB/4GiB+17ZIP64/performance,UI/FIFO/PhaseB,other-format interaction/resume,
new framework/dependency/system/sourceRecycle/delete,formalapp,package/install/sign/commit/push.
Stop for independent scoped acceptance; no next feature. No current implementation/check blocker.
