# Installed feedback round 2

## Goal and approval
User reported preview-related Clipboard dismissal, first-row action visibility, missing pin/favorite identification, excessive text-preview whitespace, completed-download average rate, Explorer false failure/retained completion window and Link popup component inconsistency. Final scope presented and explicitly approved with `开始`.

## Requirements and acceptance
R1 Clipboard: reproduce real event/caller lifecycle in owned fixtures before fixing; passive hover/preview transitions must not hide primary panel. Preserve external click/real foreground loss/Escape, one passive HWND, generations/cancellation/deferred writes.
R2 History: selected first row alone never reveals actions; hover or action-keyboard focus may reveal them. Pin/favorite states remain visible independently and icon assets/paint work. Retain existing one-dispatch/paste/menu/Tab owners.
R3 Text preview: actual wrap/content/native dimensions agree; short/plain multiline/CJK/long text have bounded content-driven height and scrolling without large fabricated slack. No content logging or user history reads.
R4 Completed list speed: average actual transferred bytes / cumulative active-download time, excluding paused/waiting time. Persist reliable measurement; legacy records lacking duration show dash, never fabricate. HTTP/media/resume/retry/restart coverage uses isolated temporary/local transfer fixtures.
R5 Explorer: target-folder/file-selection confirmed separately from foreground. Confirmed target can succeed even if activation is denied; attempted launch/API acceptance alone isn't target confirmation. Originating completion window closes only after current correlated success. Failure/timeout/cancel/stale stays retryable. List failure text must not mention a completion window. No live Shell/service/registry operations.
R6 Link popup: inspect exact released compounds and use public Kit composition/default tiers without duplicating ordinary controls. Preserve identified-only capture, Open vs Copy, expand, one window/generation/timer and close-before-defer. Actual layout/action fixture and source selection record required, not imports only.

## Out of scope
Extension/Python runtime, W2+, dependency/private/upstream changes, installer/package/install, live clipboard/history/downloads/browser/Explorer/service, volume scans, commit/push/reset. Source fixtures aren't installed physical/mixed-DPI/reader/native acceptance. No unconditional click-close or suppress-all-errors/focus policy.

## Verification
Red-capable actual-caller tests before fixes; oneCargo, jobs1/incremental0/codegenunits256, command-local1.96.0 and21600s. Desktop full suite, Download lib/relevant serial contracts, exact audited safe C# filters/build, production/tests strict lint, workspace--tests/fmt, relevant source/spec gates. Document actual counts/logs and outstanding real acceptance.
