# TAR-AC01 independent re-acceptance — PASS

2026-10-29. **TAR-AC01 CLOSED; Step3 accepted at declared source/automated/owned-native boundaries.** Together with prior independent tar-edit-acceptance.md evidence, this permits ordered Step4 dispatch, not PhaseE/A–F/product/release acceptance. Original negative acceptance and implementation/repair reports remain unchanged. No repeated broad reviewer; only specific repair/source/tests rechecked by coordinator.

## Concrete repair verification

Only product change is tar_edit.rs (SHA25630F80D56C7921C6F3B5B9F332E90DFD28B0ED34DED00A5EF7BDC9AC90B4D3E6A). Member::change compares original main header bytes before checksum update; Extension::change_data rewrites size/checksum only when payload length changes. change_pax updates only affected path/size records and preserves unrelated raw records. New header initial checksum remains valid; actual payload-size changes still update dependent header fields. No codec/budget/safety/native handoff/ZIP/7z replacement change.

Independent current worker/candidate raw regressions compare ORIGINAL fixture bytes, not editor's mutated plan.4Rust tests drive5scenario groups and20fixtures: PAX/GNU-only rename same/longer path, replacement same/different size with unaffected PAX/GNU records, affected PAX size override, octal/base256 numeric and6octalNULspace/7octalNUL checksum forms, complete unmodified second member record. CPython+bsdtar independently parse names/content/uid/gid/mode/owner/time. All20source-unchanged/raw checks PASS. Original4FAIL log read and preserved, not newly replayed old product.

**Unmodified coordinator's original failed probe rerun:** same tar-header-probe.py code, test pointer/identity selects new frozen worker. Complete main header equalitytrue, source_unchanged=true, name/contentPASS; original/candidate checksum both3031303134370020. No false positive from comparing decoded metadata alone. Evidence original-probe/header-probe-posix/result.json.

## Independently executed bounded checks

Evidence root:
`C:\Users\Administrator\.pi-desktop\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\tar-ac01-acceptance-20261029-014011`

- Exact reported focused Archive command: **91PASS/0FAIL/0ignored/0filtered** across lib42+TAR eligibility1/matrix7/raw4/worker3/time6+streams12+ZIP10/worker1+7z4/worker1.13.69s command. No full42target or321Desktop rerun; earlier independent wide evidence remains its original date/snapshot.
- Archive owner strict Clippy isolated-codecs --tests -Dwarnings PASS; workspace check--tests PASS. Idle shared lane and same-command jobs1/incremental0/RUSTFLAGS codegen-units256, --locked --offline,serial tests, no timeout/duplicate build/process kills.
- Current C# normal serial incremental build0warnings/errors, not clean rebuild.
- Original TAR effects-reviewed exact LIST→sameRUN: **5selected/5RUN/5PASS/0SKIP/0FAIL/0not_run**. Real same-path operations/cancel/held parents/native5/64/latecancel/injected112 via existing fixtures. Not real disk-fill/Explorer/user acceptance.
- Scoped2Rust fmt, scoped diff and spec116documents/14owners PASS. Static gates are document/source evidence only.
- Original independent probe PASS. Ten primary commands all exit0; accounting.json records91/20/5 and exact names/exits.

Native repair-frozen worker explicitly used:
`C:\Users\Administrator\.pi-desktop\scratch\0c0e5320-11c4-40dd-bb50-f6a437188fc1\tar-ac01\frozen\nxtools-archive-worker.exe`
SHA25614089B5024EDFD53E7E07619C06231D27536441C20E3180F65CEE555CCE04384. Independent Cargo integration current worker matches hash; no signed Release or build-isolation claim.60file baseline unchanged through verification including protected Step3 files/Cargo/reports/installer; full checkpoint178121bytes saved before prepend and retained exact suffix. Final preservation check recorded separately.

## Sequence and limits

Step3 ordinary TAR/TGZ/TXZ editing is accepted only for previously disclosed qualified raw/header/local-PAX/GNU/single-outer shapes and rewrite cost/fidelity limits. No new format promise, globalPAX/sparse/links/unknown/volumes or POSIX/NTFS universal retention. Prior old ZIP coordinator instability, C#source_unavailable, CLI helper_untrusted, FB-AC05 and actual Explorer/DPI/IME/UIA/user/installed acceptance remain OPEN under prior reports. They were not rerun by this local repair acceptance and are not silently PASS.

Only this acceptance and sole checkpoint prefix are coordinator repository writes. No product/test mutation, package/sign/install/product/Explorer/system/Registry/service/ACL/UserChoice/recycle/realuser source/commit/push. Next is exactly Step4 editing entry/conflict interaction integration; HZ3/HZ4/HZ5 remain later gated sequence.
