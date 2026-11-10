# Compact new-download intake

2026-11-05. Implemented; scoped automated verification passed.

## Result

- URL-only client is560x136 instead of560x176. The body contains one full-width official InputGroup with a leading link icon and trailing paste action; idle instructions and the separate label rail are absent.
- Continue and Cancel are adjacent at the right of the original fixed footer.
- Creating a manual blank window reads current clipboard text once and prefills one admitted HTTP/HTTPS link using the ABDM first-open extraction rule. Reopening an existing window only activates it.
- Prefill retains the original InputState and never starts a probe, task or transfer. Actual edits, edit-then-clear, timeout, disconnect and closed/reopened windows prevent stale writes. Supplied browser/Blob requests retain their original source and workflow.
- Manual paste, explicit Continue, confirmation fields, media options, scheduling, keyboard/IME, fixed-stage native handoff and theme ownership remain intact.

## Implementation

Rust owns selection and original-window/request/edit-revision eligibility. Additive no-payload `ReadClipboardText` uses the original protocol/result channel; C# reads Unicode text through the existing bounded STA runner, detached from the serial reader. Native failures retain fixed categories and HRESULT without logging clipboard contents. The shared portable web-link extractor performs no native read or history lookup.

The compact frame is official Kit0.8.0 Medium InputGroup32px with retained InputState and XSmall ghost paste Button24px. Palette, fonts, dependencies, vendor source and motion tokens are unchanged. Short official input-state color transitions settle before continuous-frame assertions; reduced motion and active-probe-only sustained animation remain enforced.

## Verification

| Selection | Passed | Failed / ignored or skipped |
| --- | ---: | ---: |
| Complete Desktop downloads module | 243 | 0 / 0 |
| Original new-download theme transition | 1 | 0 / 0 |
| Protocol library | 108 | 0 / 0 |
| Clipboard library | 48 | 0 / 0 |
| C# injected text-reader / bounded-STA fixtures | 2 | 0 / 0 |

The focused intake10 and motion1 checks are subsets of the243, not additional distinct passes. They verify the final source. C# uses identical preceding LIST and RUN selections; `not_run=0`. Its build reports zero warnings/errors. Desktop bin/tests and protocol/clipboard libraries pass strict Clippy with warnings denied.

Scoped rustfmt, whitespace, spec navigation/contracts, GPUI component catalog, locked release, pinned editor, download lifecycle and fixture-selection gates pass. Rust verification is offline, jobs1, incremental0, codegen-units256; final dev/test debug information is0. No verification filters are presented as a full-workspace result.

Evidence: `.diagnostics/download-intake/20261105-compact/`, including command outputs, selection/results, source gates and `verification-summary.json`.

## Limitations

- Physical clipboard-to-window interaction, screenshots/user visual acceptance, native DPI/IME/provider and installed behavior are not certified by these compiled GPUI and injected STA fixtures.
- The prior unfiltered workspace run remains failed on the unrelated download-menu text assertion. This task does not change that menu or claim whole-workspace acceptance.
- Packaging remains paused. No installer, installation, service operation, product launch, commit or push is performed by this task.

## Link-prefix asset follow-up (2026-11-09)

The leading official Link was outside Kit0.8.0's default asset subset. NxToolsAssets now composes the released selective icon source for Link, preserving the original InputGroup inset/frame/InputState rather than removing icon space. Final intake11 and original asset2 tests pass, including actual SVG nontransparent raster pixels and production prefix-click/Home insertion. [Server/reference delivery](../11-09-download-server-information/outcomes.md) retains current validation and installed/visual limits.
