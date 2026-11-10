# Compact new download intake and clipboard prefill

## Goal

Match the user's compact URL-entry reference without changing the existing probe/confirmation/download workflow.

## Requirements

- Remove the idle numbered HTTP/HTTPS guidance entirely.
- Keep one full-width URL input with a leading link icon and trailing paste action in one official frame.
- Align Continue and Cancel together at the right of the fixed footer.
- Reduce the URL-only client from560x176 to560x136. Keep ordinary confirmation/media budgets and the existing hidden fixed-stage handoff.
- On creation of a manual blank window, read clipboard text once. Follow ABDM's single-link rule: fill exactly one valid HTTP/HTTPS URL; do not choose among multiple URLs or fill non-link text.
- Prefill only; never probe, create a task or start a transfer automatically. Existing browser/Blob/prefilled requests and manual paste retain their behavior.
- Reuse the original application/window/input owners. Native clipboard access belongs to C#; Rust owns URL policy. No clipboard monitor, history lookup or polling.
- A delayed read must not replace an edited, cleared, probing, closed or reopened draft. Duplicate Open only activates the existing window.
- Automatic read failure is quiet and leaves manual entry/paste usable. No clipboard payload in diagnostics.

## Reference and component selection

ABDM: EnterNewURLPage.kt::LaunchedEffect -> BaseEnterNewURLComponent.onPageOpen -> ClipboardUtil.read -> StringUrlExtractor.extract; exactly one link is selected once. CustomWindow forwards lifecycle to Compose Window/onCloseRequest. Legacy AddDownloadWindow constructor/WindowAnimationService.Attach/OnOpened/RequestClose/OnClosed retain explicit probe, cancellation and cleanup; Avalonia native base is not available.

Official Kit0.8.0: InputGroup::input/addon with retained InputState and default Medium32px frame, leading InputGroupAddon Icon and trailing XSmall ghost InputGroupButton24px. Public addon padding fits the32px frame. Original primary Button and outlined Cancel remain outside Toolbar.child adaptations. Existing Form/Field stays on the confirmed surface. Palette, fonts, focus/IME and original callbacks remain unchanged.

## Change boundary

Production: new_download_view.rs for compact composition/URL edit revision; browser_handoff_impl.rs and a private clipboard-prefill module for one-shot request/result lifecycle; original downloads.rs for pending state/routing; additive no-payload ReadClipboardText command and C# bounded STA reader/dispatch. Existing Clipboard web-URL extraction is shared rather than duplicated. Tests exercise original application Open, actual rendered bounds, typed command/result and real URL input edits.

No download-engine behavior, download-menu test, archive implementation, dependency, vendor source, packaging, install, service, user archive or system clipboard mutation is in scope.

## Acceptance criteria

- Production compact bounds136px; one input frame, no idle guidance, right-aligned Continue/Cancel, all hit targets reachable in Light/Dark and14/16/18base-font fixtures.
- Exactly one clipboard request per newly created blank window; matching single URL fills the original InputState without starting work.
- Invalid/empty/multiple links, edited-then-cleared drafts, late/stale results, close/reopen, disconnect, failure and duplicate Open retain correct ownership.
- Browser/Blob, manual paste, probing status, confirmation handoff, input/IME/keyboard and media/footer regressions remain intact.
- Native reader/parser/result tests use synthetic injection and the existing bounded STA runner; no live clipboard writes are required.
- Relevant Rust/C# tests, strict scoped lint, formatting, lifecycle and locked component/editor/spec gates pass. Physical/native DPI, installed and user visual acceptance are reported separately.
