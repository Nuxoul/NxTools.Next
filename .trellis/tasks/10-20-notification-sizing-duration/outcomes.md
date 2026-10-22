# Notification sizing and duration — source outcome

## Delivered

### Natural content height
- Removed the project adapter's combined title/message/detail `54px` ceiling. Short notifications now use natural content height; the currency amount, reference date and provider fit initially without scrolling.
- Retained the official Notification/NotificationList/ToastStack card, theme, stack, close button, animation and hover/focus behavior. Width remains 336 DIP (360 DIP native popup including margins), rather than growing across the screen.
- Long text scrolls only after reaching its share of the existing 400-DIP stack capacity. The natural text body does not flex-compress, and official Small 24px action buttons remain outside the text scroller. Three expanded long cards and their actions fit the allocated native viewport.
- Window-local compatibility presentation additionally respects the real viewport and title inset. The production passive screen popup keeps its original prepare/readback/hidden draw/reveal and visible `keep_visible` resize owner; no C# or protocol changes.

### Configurable official countdown
- Restored editable **1–60 seconds, default 6** in Global Settings using the existing configuration field and atomic Apply/Cancel transaction. Existing valid saved values are reused, not reset to 6.
- The official default-32px NumberInput uses a 168px lane so both step buttons and the editable center fit. Pointer editing, Up/Down, +/- limits, busy disabling, invalid drafts, Cancel, reopen, successful save and failed-save preservation are covered.
- Startup and successful settings save publish the value for newly admitted timed notices only. Existing mounted entities, generations and remaining countdowns are not reset. Progress remains persistent.
- `vendor/gpui-component-071` is a released Apache-2.0 Component 0.7.1 snapshot selected by root Cargo patch. Only `src/notification.rs` differs from the release: optional duration/default None, `Notification::duration(Duration)`, and forwarding into the existing `ToastOptions.timeout`. No new timer, pause controller, card renderer, stack or animation implementation.
- Unconfigured official callers retain the upstream 5-second default. Product callers use their configured duration. `autohide(false)` still ignores duration; OS/system delivery is unchanged.

## Latest-version check and provenance

The user paused work to verify newer official versions before accepting a patch, then explicitly resumed after the result. Official crates.io metadata and 0.7.3/0.8.0 archives were downloaded into session scratch and checked against registry SHA-256 checksums. All three released `notification.rs` files (0.7.1, 0.7.3, 0.8.0) are byte-identical:

`09010c94024ee6e2580ecf557d97dcf276512a52be8cf298e0936088df0afeca`

All hardcode 5 seconds with no public duration/timeout setter. 0.8.0 also requires GPUI 0.3.9; it was not adopted. Kit remains exactly 0.7.1. Relative to the task baseline, Cargo.lock only changes Component 0.7.1 from registry source to the local patch; no package versions were upgraded.

All 259 copied release files were hash-compared: **only `src/notification.rs` changed**. The original Apache license is identical. The historical `vendor/gpui-component` 0.5.1 and registry source were untouched. Patch scope/provenance is recorded in `vendor/gpui-component-071/NXTOOLS-PATCH.md` and THIRD_PARTY_NOTICES.

References read: notification note, exact released official sources, actual Zed status toast and toast-layer lifecycle, and production C# NotificationWindowPlacement validation/preparation/readback. Zed's 10-second/single-owner implementation was behavior reference only, not transplanted. No matching Notification-named entry was found by filename in old NxTools; this is not a claim of exhaustive absence.

## Red/green evidence and repaired failures

- Initial production currency-render and screen-host regressions: **0 passed / 2 failed**, both reporting exactly **54px** actual text viewport. After repair both pass and assert the complete natural body is inside the initial visible text area.
- Added short-to-multiline growth, actual 100-line body scroll-to-end and three-long-card native-stack/action containment tests. All pass.
- Added small 300px window/top-position regression before the viewport adaptation: **0/1**, actual card height 376px beyond the viewport. After the scoped window-local budget fix, top and bottom cases pass.
- NumberInput step-boundary regression caught a real competing-default-step issue (60 became 61). `InputState::set_step(None)` leaves one clamped project callback; pointer/keyboard limits now pass. An earlier 95px min-content control width was adapted to 168px; pointer focus/editing is now tested.
- An over-specific settings right-edge test expected 700px despite the official row ending at 704px. Corrected it to assert actual parent-row containment, retaining the 32px control and reachable-footer checks.
- Initial integrated Desktop run: **849 passed / 2 failed**, 0 ignored, 94.77s. The failures asserted the superseded 112px notification cap and assumed an implicit 5-second product default. Updated only those assertions/fixtures: natural bounded height plus original action containment/click, and an explicit 5-second setting plus original replacement/progress semantics. Both focused regressions pass.
- New test authoring errors around dynamic selectors were corrected to the locked TestContext API's static selector contract. No product API or compiler-warning bypass was added.

## Final performed verification

All Rust commands used `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`, all-machine build-lane preflight and full-operation timeouts. No peer was contacted or killed.

| Check | Result |
| --- | --- |
| Final complete Desktop | **852 passed, 0 failed/ignored**, 94.76s |
| Final notification-related filter | **34 passed, 0 failed/ignored** |
| Global settings suite | **29 passed, 0 failed/ignored** |
| Core library | **59 passed, 0 failed/ignored** |
| Strict Desktop tests Clippy `-D warnings` | PASS |
| Workspace `cargo check --workspace --tests --locked --offline` | PASS |
| Scoped rustfmt / whitespace | PASS; final post-document rerun recorded below |
| Desktop shell, UI state, motion, Clipboard UI, Quick Launch lifecycle gates | All 5 PASS |
| Spec contracts / component catalog | PASS: 106 documents / 14 owners; 79 pages / 78 locked entries |
| Vendored source / license comparison | Only notification.rs changed; Apache license identical |

Countdown coverage exercises 1/6/60 seconds on the real official list, upstream unconfigured 5 seconds, whole-group remaining-time hover pause, settings/position changes without timer reset, stale replacement and persistent progress. The original clipboard/rich-preview/currency/Enter regressions remain in the full passing Desktop suite.

Logs are in session scratch: `notification-sizing-red.log`, `notification-small-viewport-red.log`, `notification-final-{focused,settings,core,clippy,workspace,desktop}.log`, `notification-closed-{focused,clippy,workspace,desktop}.log`, the two updated legacy-regression logs and `*.notification.log` architecture gate results.

## Limits / operations not performed

No framework upgrade, registry source edit, delegation/cross-session messages, commit, task archive, package, install, ordinary product launch, service operation, real clipboard/history/config access or live native action. Existing unrelated dirty changes and running products/services were preserved.

The screen-host regression uses production Rust layout/commands and synthetic native viewport acknowledgements, not a real Windows HWND/DPI validation in this slice. No new screenshots or visual approval, physical mouse/keyboard/Windows-IME, mixed-DPI/multi-monitor, reader/UIA or installed acceptance is claimed. Existing official hover-close and list-wide pause semantics remain. No source/test blocker remains after the final full run; release/native/user visual acceptance remains separate.

## Final hygiene

Final scoped rustfmt --check and git diff --check exit0; updated specification/component-catalog gates pass, and all five lifecycle gates pass again on final source. No production diagnostic probes or blanket warning suppression remain. No product behavior edits follow the passing full run.
