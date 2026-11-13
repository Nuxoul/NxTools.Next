# Browser integration reliability and workflow polish

## Goal
Protect browser downloads that cannot be reconstructed, make connection failures actionable, and shorten explicit batch/media workflows without replacing the existing browser/engine boundary.

## Requirements
- Automatic downloads observed with a non-GET method remain in the browser without credential reads, task creation or cancellation. A redirected non-GET final request is not reconstructed as GET.
- Connection UI distinguishes missing/forbidden native host, startup failure, timeout, disconnection and rejected protocol/request with fixed payload-safe guidance. Passive refresh never reconnects after loss.
- Users can extract selected links or page links, review them in Capture, and explicitly send at most32 ordinary HTTP(S) items through existing stable batch identities. Extraction alone creates no download task.
- Media inspection runs only on explicit user action, is bounded, uses the original page/frame cookie context, and returns a summary rather than manifest contents. Failed inspection preserves the resource and permits retry. Protected media is explained; AES-128 is not mislabeled DRM.
- Preserve four tabs and existing graphite styling. Add homepage pause/status, remembered tab, resource name/domain search and collapsed other-page resources.
- Disabled capture/overlay must not enumerate mutation subtrees needlessly. Existing normal-mode timer duties remain unchanged.

## Acceptance
- Real Chrome/Firefox listener and runtime fixtures demonstrate non-GET retention, GET handoff, correlated diagnostic outcomes and no credential leakage.
- Extraction is bounded/deduplicated, rejects unsupported schemes, obeys source page lifetime and requires explicit batch confirmation; partial retry retains original accepted/pending identities.
- Production Popup/content DOM checks exercise search, keyboard/focus, review, pause, connection help, inspection pending/failure/retry and no overflow.
- All extension Node suites pass; relevant native log schema tests pass when changed. No installed browser/native registration/package operation is implied.

## Constraints
No new permission, dependency, transport, engine behavior, proxy interpreter, POST replay, remote delivery or automatic manifest fetch. Preserve unrelated worktree changes.
