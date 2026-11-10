# B1 — Existing correctness

Approved implementation scope: REF-01, REF-02 and REF-03 in [taskbook](../../../docs/reference-improvement-taskbook.md). Preserve the dirty working-tree baseline, existing InputGroup/Link intake, server information, native icons and parallel ZIP extraction.

## REF-01
Ordinary production link-row activation consumes current ClipboardSettings.link_panel_copy_only and auto_open_browser. Copy-only wins for every target. Web activation without automatic opening copies rather than launching. Existing named Copy and explicit open actions remain distinct. Local file reveal/directory open, durable settings, focus, one reusable panel, generation retirement and deferred cross-entity callbacks survive. This request supersedes historical settings-independent ordinary-row behavior, not explicit-action semantics.

Acceptance: actual production ListItem pointer and Enter/Space; assert clipboard bytes and typed platform commands for settings combinations and local targets. Reuse/reopen, stale deferred callback, current settings and original timers/focus remain covered.

## REF-02
Resolve originating Cookie-store identity from supported download/request/tab evidence, including Firefox container/private and Chromium private stores. Query that store for each fallback URL; never fall back to another store when source is unavailable. Header master and Cookie subordinate gates skip all Cookie reads when disabled. Captured request context stays privileged; no additional Cookie persistence, permissions or sensitive diagnostics.

Acceptance: production background events, deterministic same-URL/different-store API fixtures, exact getAll parameters and Native Messaging payload Cookie. Cover redirects, closed/replaced tabs, unsupported APIs, gates, single/batch and actual host framing/roundtrip.

## REF-03
Distinguish rule exclusion, temporary pause, desktop unavailable/refused, timeout/disconnect and browser-cancel failure. Separate policy, in-process queued, durable acceptance and browser ownership-transfer results. Expose payload-safe actionable explanation in the existing popup.

Acceptance: actual extension/host fixtures; matching request and handoff identities, durable idempotency, no cancellation before acceptance, cancellation failure keeps browser item and original desktop-task compensation. No generic enqueue-success claim.

## Constraints and limits
No REF-04+ work, new rule types, limiter/completion/connection/volume changes, replacement transport/state/settings/scheduler, dependency/vendor changes, broad formatting or unrelated baseline fixes. Rust owns domain, GPUI rendering, C# native APIs. No package/install/product/reference-program launch, extension installation, registry/service changes, commit or push. Compiled/fixture evidence is separate from installed/native/physical/visual acceptance.
