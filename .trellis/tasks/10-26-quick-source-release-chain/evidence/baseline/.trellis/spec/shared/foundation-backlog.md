# Source-backed quality gaps and Foundation handoff

This is a gap ledger, not a waiver. Required target is [UI contract](ui-design.md); evidence is current source or approved task. No product code was changed to hide gaps. Only existing owner plans may implement them after scope/visual approval.

| ID | Observed gap/evidence | Owner / observable acceptance |
| --- | --- | --- |
| F01 | Foundation2026-10-09 now maps approved palette and component theme; old source palette tests replaced with approved exact/semantic roles | Source gap corrected, native before/after screenshot/DPI/user visual acceptance pending |
| F02 | Shared24/28/32 controls,3/6/8 radii,settings40/header28 and common4/8 gaps now implemented; module/native measured sizes remain | Consolidated module pass plus real media/footer regression; remaining off-grid roles are explicit debt, not blanket exemptions |
| F03 | Caption/toggle/state/settings-nav/weekday/management actions now use focused GPUI KeyUp; local handlers preserve defer/generation | Actual changed-surface keyboard/disabled/single-dispatch gates pass; other custom actions and whole-app provider/DPI acceptance still not certified |
| F04 | System SPI preference + correlated C#/IPC/Rust global and all current desktop animation sites implemented | Unknown/failure/disconnect reduce; actual nested render and immediate generation-scoped dismiss tests pass. Live system toggle/reconnect/DPI remains pending |
| F05 | Verified GPUI0.2.2 has no rendered role/name/accesskit or WM_GETOBJECT/UIAutomation provider seam | Actual screen-reader names/roles blocked; requires separately scoped dependency/native-provider work, not tooltips or C# in another process pretending to inject a provider |
| F06 | Exact host/link feedback/timer/topmost/native alpha acceptance remains unchecked in existing link-panel PRD | Clipboard + Tools/link child: exact detector and generation/result regressions, actual transparent corner hit testing when authorized; preserve copy-only/opt-in defaults |
| F07 | Historical UI plan declares source slices done while later stage plan is planning and target differs | Standards fixes navigation/authority only; owning UI stages inventory actual finished behavior, do not restart completed features or interpret historical screenshots as new acceptance |
| F08 | Core load_or_create writes first defaults directly, unlike locked update_section sibling-sync/rename; error contexts in core/Finder/indexer include paths | Current observed paths, not new bug declaration. Future config/privacy owner verifies affected callsite/real failure before change; log raw context is never implicitly safe |
| F09 | Large orchestration modules remain in core/download/clipboard/desktop; no useful arbitrary line threshold | Future scoped testability/lifecycle seam only; no full-file split for aesthetics |
| F10 | Protected service rejects unregistered development/Validation client identity; live compatibility unverified | Existing SOL system task; don't weaken auth. Explicit authorized dev contract and real package/service gate separate |
| F11 | Real pointer/probe/release/DPI, protected LocalSystem idle/restart/base+delta/search/hash, actual registry/installer acceptance pending | Existing SOL/window/system owners; no claim from static/docs checks. Service/launch/installation needs separate authorization |
| F12 | Extension popup filter selected state only toggles .active; shadow-styles has no collected reduced-motion rule, unlike popup CSS | Existing extension/UI owner: semantic selection and separate popup/shadow motion plus live DOM keyboard/provider tests; no extension behavior edits here |
| F13 | C# PlatformLog.Write is bounded/TryWrite, but Dispose synchronously drains/joins | Observed shutdown distinction, not broad failure claim. Native owner requires causal regression before changing lifecycle; do not promise bounded disposal from sink admission alone |

Evidence: `repo:rust/apps/nxtools-desktop/src/theme.rs`, `repo:rust/apps/nxtools-desktop/src/ui/metrics.rs`, `repo:rust/apps/nxtools-desktop/src/ui/motion.rs`, `repo:rust/apps/nxtools-desktop/src/ui/components/state.rs#interactive_state`, `repo:rust/crates/nxtools-core/src/lib.rs#load_or_create`, `repo:rust/apps/nxtools-indexer/src/windows_service_host.rs`, [GPUI capability note](../../../docs/reference-code/gpui.md), [link requirements](../../tasks/10-02-link-detector-panel/prd.md).

## Closed test blocker (not release acceptance)
The two recorded engine-contract102/103 failures were causally closed on2026-10-09 by a test-only accepted-socket correction; one integrated103/103 run plus fixed focused rounds pass. [Dated report](../../../docs/audits/2026-10-08-sol-review-remediation.md) retains red evidence, source-only HEAD comparison and limits. Do not keep citing the old failures as current blockers or claim the package now contains changes.

## Foundation entry checklist
1. Read later approved stage design and exact owning phase order, current UI/source tests and references.
2. Freeze baseline behavior/state matrix and target/current mapping; name any justified exception explicitly and prove its constraint.
3. Follow the existing canonical stages with scoped behavior/budget tests; the user2026-10-09 amendment authorizes all remaining module/common implementation before consolidated acceptance, not only a representative Foundation surface.
4. Actual GPUI bounds/actions plus source token/lifecycle gates, compiled checks when authorized, visual/keyboard/mouse/DPI review; unavailable provider/native evidence stays pending.
5. Consolidate native visual/interaction acceptance after implementation; the user amendment supersedes the historical per-stage pause. Do not mark whole modules, Tools or provider support complete from representative source/screenshots.
