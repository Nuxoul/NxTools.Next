# Current formal release — Release38 completed and independently verified

2026-10-30。[正式Release38交付](formal-release38-outcomes.md)：原正常入口、SkipVerification=false，sequence38／releaseID53692ed3-7f88-4678-88d8-3275441d4a71，安装器26,720,959bytes，SHA2567733A5504A37E10AC05CD265190D194BA463E6848883EF48837242203B4BC5A7。内部RSA3072-PSS/SHA256、77文件精确清单、10构建输出、5归档批准、helper绑定和18帧图标独立核验通过；Windows Authenticode仍NotSigned。

本次Rust2994PASS/0FAIL，1ignored/450filtered不计执行；实际Release门禁4PASS；完整匹配Debug C#600RUN＝500PASS/100SKIP/0FAIL/0not_run；额外实际Release原生回归4PASS；Inno隔离10PASS、BrowserHost替换5PASS。903源码输入验证期零漂移。未自动安装/启动产品；HZ5未启动，真实安装/物理用户验收仍独立。旧33–37占号及以下负面历史原样保留。持久日志 `.diagnostics/packaging/20261030-100128-5d8c4366/`。

---

# Latest formal attempt36 — C# screen-hit fixture blocked; no new installer

2026-10-30 08:12–08:47. User requested packaging again. Fresh original CheckOnly passed with29.86GiB committed headroom and idle lane. Normal `package-local.ps1` reserved36 and ran for2124.95seconds, exit1; no SkipVerification, signing fallback, automatic install or product launch. Persistent evidence: `.diagnostics/packaging/20261030-081206-5ec9bce7/` (`status.json`, stdout and stderr). Controller: `C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\formal-package-20261030-081204`.

- Architecture/fmt/strict Clippy and full Rust workspace tests completed; exact130 summary groups:2994PASS/0FAIL/1ignored/450filtered. Release Desktop/indexer/CLI/browserhost and isolated worker builds completed. Release host publishing/bindings and complete Release Tests build completed with0warnings/0errors. These are compiled outputs, not a signed delivered package.
- C# original `--fixtures`: `selected=600 run=23 pass=22 skip=0 fail=1 not_run=577`. Failure: `floating input fixture transparent hit and pass through`, before input surface construction, renderer cannot own its center pixel at140,140. Actual hitHWND9076C differs from owned rendererC0DA0; pixel272320, expected65C72D. One exact focused reproduction failed identically (owned renderer709DC); original and focused negatives retained.
- Read-only native attribution of actual point140,140: PID2736 `D:\Delta\delta-app.exe`, class `Zed::Window`, Default desktop, session1,2560x1440. This establishes the fixture's screen ownership prerequisite is not satisfied; it does not prove a product transparent-input defect or the exact external z-order cause. No user window was moved/closed, no activation/global input delivered, no test/assertion changed or waived. An isolated/unobstructed screen environment is required before this validation can pass; do not keep reserving/rebuilding to hunt a green result.
- Current archive source baseline drift is empty. No remaining Cargo/rustc/clippy/ISCC process was observed after failure. Old installer remains26,213,543bytes/SHA25685B850532C013FD2CEE4945875CB5DF4BE16599C8286385980876DD1FA5281D7 (actual rehash). No new formal installer/signature produced; do not deliver sequence32 as this build.
- Final CheckOnly: ledger36/package32/next37,28.59GiB,Readytrue. Sequence36 is consumed; recompute from original ledger on any later attempt, never reuse33–36. Remaining gates: actual complete C# fixture verification, formal signing, installer and manifest/hash validation. HZ5 remains not started.

The following earlier memory-blocked record is historical and retained unchanged.

---

# Formal Release packaging — blocked before next reservation

2026-10-30. User explicitly requests the normal **formal Release installer**, not an experience/unsigned package. Existing release key, original monotonic ledger, full verification, signing and installer workflow remain required. No automatic installation or product launch. Archive implementation stays **Phase E — Format expansion and ZIP editing**; HZ5 has not started.

## Actual publishing state

- Previous attempts reserved sequences **33, 34, 35**, all failed; those numbers remain consumed.
- 33: `.diagnostics/packaging/20261029-230437-36d1e12a/`, obsolete QL-11/12 source-contract tokens rejected current official Input/rem layout.
- 34: `.diagnostics/packaging/20261029-233022-b09d5e9c/`, Windows PowerShell converted ordinary compiler stderr into a terminating native-command error; not evidence of a compiler defect.
- 35: `.diagnostics/packaging/20261029-234930-b2ea185d/`, actual Cargo exit101: TAR readmission chain lacked its explicitly required owned fixture root. Full verification was not skipped.
- Latest `package-local.ps1 -CheckOnly`: ledger35, package32, calculated next36, no active build, committed-memory headroom **0.81GiB**, Ready=false (required6GiB).
- The normal formal entry immediately repeated its own preflight: **0.78GiB**, rejected before reservation/build. **36 was not reserved**; no new persistent packaging run was created. No new Release installer or signed payload was produced.
- Existing installer remains sequence32/release ID `73b6a27d-d4ad-4c8a-95da-b4eea608183e`, 26,213,543bytes, SHA256 `85B850532C013FD2CEE4945875CB5DF4BE16599C8286385980876DD1FA5281D7`. Its hash was rechecked, not inferred from the filename. It does not contain current unpackaged changes.

## Bounded repairs and verification

1. Updated only obsolete QL-11/12 architecture source expectations to existing official Input/Field and rem row APIs. Complete lifecycle gate and three independent negative controls passed; original editor geometry tests remained.
2. Applied exact rustfmt output to three network-settings files. Production-only unused import/classification-field warnings were corrected with `cfg(test)` at their actual test-only owners; no lint suppression, UI behavior or resource limit changes. Earlier full workspace strict Clippy/fmt and focused floating-assistant43/editor2/network3 passed.
3. `package-local.ps1` now starts the original publisher as an OS child, captures stdout/stderr separately, waits and records the actual exit code. Two failing regressions preceded the repair; all **11 isolated wrapper tests passed**, including ordinary stderr+exit0 and stderr+exit7. Manifest verification remains mandatory. `PACKAGING.md` documents the separate logs.
4. Created new owned scratch TAR fixtures with the checked-in independent CPython/libarchive generator and installed stock7Zip oracle. Rebuilt current Debug fixture host. Original TAR chain **1 test / all17 scenarios passed**, and declaration contract **2 passed**, with no scenario filter.
5. Full workspace default-parallel run then failed two Core storage-management fixtures at `StorageOwner::new`: six independent tests compete with the original process-wide four-owner ceiling. Serial focused execution passed **6/6**. The ceiling and product code were not changed.
6. Full serial run exposed libtest progress prefix mixed into a CLI child JSON line. Only added `--format=terse` to that test child's arguments in `rust/apps/nxtools-cli/src/output_tests.rs`; all original JSON/identity/exit assertions and36 actual synthetic pipe cases remain. CLI library **15/15 passed** after repair.
7. Complete `cargo test --workspace --jobs 1` with `RUST_TEST_THREADS=1` and explicit TAR fixture environment passed: **2994 passed, 0 failed, 1 ignored, 450 filtered**, across130 result-summary groups (including doc-test groups). These framework filtered/ignored counts are not claimed as executed tests. Earlier failed logs remain retained.
8. Built current isolated-codecs Debug worker and saved an immutable owned test copy, SHA256 `CC3A863FA34ADD66EC71F5C3CAECEF13213E4168264903865AB427B6FD7285FD`, so later default-feature builds cannot overwrite the fixture worker. This is a test preparation build, not a formal Release payload.

Every Rust command used the shared low-memory initializer (jobs1, incremental0, codegen-units256 and profile controls); an occupied unrelated build lane was respected. No process-name kills, system-wide memory cleaner, key replacement, ledger rollback, assertion/budget weakening, SkipVerification, unsigned fallback, installation or HZ5/source reclamation occurred.

## Evidence and remaining gate

Current owned fixture evidence:
`C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\formal-fixtures-08fe94657edf4d7fbcd6fc87fc6128d8`.
It retains independent fixture manifest, host/worker/oracle identities, targeted tests, both failed full-workspace logs, final serial workspace log/counts and formal environment. The controller records environment, source baseline, previous artifacts, command/exit and no installation.
Latest preflight-only controller record:
`C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\formal-package-20261030-010213` (source drift empty).

Resume only after a fresh `package-local.ps1 -CheckOnly` reports adequate headroom and idle lane. Recompute next sequence from the real ledger/package floor; do not reuse33–35 or hardcode36. Supply the explicit current owned fixture environment and serial test environment, then call the original normal entry without SkipVerification. Complete Release/C# fixture verification, production signing and installer/hash verification still remain. No new formal signature, installer, installed service or physical UI acceptance is claimed.
