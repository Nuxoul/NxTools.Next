# Desktop errors and presentation

Evidence: `repo:rust/apps/nxtools-desktop/src/network_settings.rs`, `repo:rust/apps/nxtools-desktop/src/clipboard.rs`, `repo:rust/apps/nxtools-desktop/src/quick_launch/actions.rs`, `repo:rust/apps/nxtools-desktop/src/downloads/new_download_view.rs`.

Map typed owner failure to local actionable state; distinguish no data/loading/failed, saved desired/applied Windows status and accepted/applied client. Superseded query/retired generation is quiet; failure preserves last-good/draft/retry. Copy KeepOpen policy runs at acceptance and completion; matching paste-only closes. Confirmation viewport timeout never releases clipped form. Clear all pending/timestamps on terminal path; no nested cross-owner update to show an error. Raw path/query/native errors are not diagnostic-safe. [Engineering](../../shared/engineering.md), [state matrix](../../shared/ui-design.md).
