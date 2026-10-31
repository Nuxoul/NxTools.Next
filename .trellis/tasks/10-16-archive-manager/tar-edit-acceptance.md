# Step3 TAR editing — independent acceptance / repair required

2026-10-29. **Step3 remains PARTIAL / not accepted for sequence advancement.** Independent targeted/compatibility gates pass except the known historical native readmission case, but one new raw-header fidelity defect was confirmed and reproduced. Return only TAR-AC01 to the same implementation session; no Step4/HZ3–HZ5 dispatch until repair accepted. Original implementation report/failures and A–F status remain unchanged.

## TAR-AC01 — OPEN, low severity but explicit fidelity contract mismatch

`repo:rust/crates/nxtools-archive/src/tar_edit.rs#Member::change` at420–423 unconditionally re-encodes extension size/checksum after any member change;454 unconditionally re-encodes main header checksum, even when only an overriding PAX/GNU pathname changed and main header bytes did not need mutation. Valid numeric/checksum representations can therefore be normalized unnecessarily. Original spec tar-edit.md13/outcomes24 promise retained raw headers/extensions and only necessary dependent-field rewrite. Decoded member content/metadata equivalence does not satisfy that exact raw preservation promise.

Independent real frozen-worker probe uses CPython-generated local PAX path old.txt over main placeholder, patches valid six-octal-digit/NUL/space checksum fields (verified independently readable), then sends actual modify_tar Rename to new.txt through original worker native rendezvous. Worker complete, exact new name/content independently verified and source unchanged, but **untouched main header differs**: original checksum hex3031303134370020, candidate3030313031343700. Probe exits1 at raw-main-header equality. No final user file publication or product source edit occurred. Directory `header-probe-posix` retains source/candidate/wire/result. Initial `header-probe` with seven-digit/NUL representation passed because it matched writer canonical output; both positive/negative artifacts retained. One scratch script line-placement mistake was corrected before second execution, not a product/behavior failure.

Minimum repair: only alter/checksum headers whose underlying bytes actually change; only rewrite extension payload/size/checksum when affected path/size representation changes, preserving untouched representations otherwise. Add byte-preservation regressions at actual worker/candidate boundary for PAX-only and GNU-path-only rename, replacement with unaffected extensions, legitimate noncanonical numeric/checksum fields and unchanged records; still recompute checksum when bytes truly change and independently validate candidate. Do not simply drop the raw-fidelity promise or blanket-refuse normal readable headers. Do not rewrite other codecs/transactions. This one bounded readonly review is already consumed; no repeated broad review loop on repair.

## Independent execution already completed

Evidence:
`C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\tar-acceptance-20261029-005604`
Use actual scratch pointer `tar-acceptance-work.txt` for authoritative path; results.json retains exact commands/exits. Delivery52source hashes were rechecked before testing,728file opening baseline saved including checkpoint and installer. Final source drift check empty at this report. Native artifacts/oracles explicitly hashed; no implicit current target substitution.

- Archive exact delivery lib+41integration targets, with --no-fail-fast: **42targets /254PASS/0FAIL/0ignored**, one explicitly skipped100k performance case. Three standalone RAR/7z/TAR-stream full coordinator targets not selected.7TAR unit+11TAR integration included.
- Desktop archive: **321PASS/0FAIL/0ignored/1167filtered**,102.23s tests; original HZ118/icon8/WAC12 retained. Not wholeDesktop.
- Core93unit+7integration, IPC43, Protocol89PASS.
- Five-owner strict Clippy --tests -Dwarnings isolated-codecs and workspace check--tests PASS.
- Current C# serial incremental build PASS0warnings/errors; not clean rebuild.
- Exact `archive tar edit` LIST→sameRUN: **5/5/5PASS/SKIP0/FAIL0/not_run0**,10.68s run. Real native publications/cancel/held-source/native5/64/112/latecancel and independent CPython/bsdtar oracle.112 is injection, not real disk-fill.
- Original42native compatibility names LIST→41nonfailure sameRUN **41PASS/0SKIP**; known `archive job fixture actual readmission shortfall no output` separately **RUN1/FAIL1 source_unavailable**, combined41PASS/1FAIL/0not_run. No suppressed or skipped cases. Not previous56full suite.
- Scoped42Rust fmt/diff, spec116documents/14owners, GPUIrelease/editor/catalog/fixture selection PASS.
- Native worker frozen delivery path `C:\Users\Administrator\.pi-desktop\scratch\0c0e5320-11c4-40dd-bb50-f6a437188fc1\tar-edit\frozen\nxtools-archive-worker-final.exe`, SHA256 `00FAFA51E9CDE82E3F468829F061CF950234D60D233F6C7F0BB11E7ADA1A8191`; source-built worker from independent Cargo test targets matched same hash at readback. No Release approval.

Every Cargo checked idle shared lane, same-process CARGO_BUILD_JOBS1/CARGO_INCREMENTAL0/RUSTFLAGS=-C codegen-units=256, --locked --offline, serial tests. No build timeout/duplicate process kill; normal incremental results disclosed. No reference/product GUI, real Explorer, user archives, source reclaim, dependency/backend/vendor/package/install/system mutation/commit/push.

## Failure history and evidence limits

Read original plain/compressed/production-opt-in RED logs and mutation ignored-flush2FAIL, restored delivery source hash. Independent finish/flush tests pass without modifying product source. Readonly review found TAR-AC01 only; main validated scope and obtained direct worker reproduction. No general unsupported-extension requirement invented.

Old ZIP coordinator unified implementation run had Cancelled instead of SourceChanged, targeted rerun passed; independent complete selected-target run now passes that case. **Original failure remains unresolved instability evidence**, not root-caused/fixed by new pass. Historical native source_unavailable independently fails again; CLI helper_untrusted/FB-AC05/physical Explorer/DPI/IME/UIA/user/installed acceptance remain open. These do not replace the concrete new TAR fidelity blocker or imply entire product all-green.

Current source is inspected and tested only within declared ordinary TAR/restricted localPAX/GNU/single gzip/singleXZ scope. No universal POSIX metadata, all outer filters/resources, process-crash/restart/power-loss or HZ4/HZ5 acceptance. Step4 integration is not started. Checkpoint pre-acceptance173522bytes retained whole as suffix after status prepend; source/installer and worker preservation recorded separately.
