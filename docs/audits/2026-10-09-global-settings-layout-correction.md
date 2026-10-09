# Global settings layout correction after installed visual rejection

## User feedback and responsibility
User rejected the19:30 installed blacklist screenshot: inconsistent/tiny type, centered application names under left-aligned headers, misplaced checkbox centers and thin input beside taller actions. Palette-only mapping is not sufficient UI refinement. Previous source/token/no-overflow checks did not protect actual alignment or information hierarchy. This correction does not declare the entire desktop Zed-quality or the rejected candidate visually accepted.

## Scoped design and implementation
Only global settings rendering/local blacklist actions and regression/evidence changed. Retain720x480 client, existing palette/font family, titlebar/window lifecycle, domain rules, durable save/deferred publication and desired-vs-native-applied semantics. No Downloads/theme/vendor/C#/protocol/config-format changes.

- Replace horizontal equal thirds with128px left navigation and20px right-content inset. Navigation/body use12px text/18px line-height; blacklist title14px/20px, deliberate heading-description spacing.
- Table headers, rows and application editor share flexible application +96px Clipboard +80px Quick Launch +56px action columns. Executable names left-aligned with the application header, feature headers and16px check markers center-aligned,40px rows. Header remains outside the bounded160px scrolling body.
- Input, scope hit regions, add/update and footer actions32px tall. Input uses existing field_shell instead of small default sizing; explicit12px input/button text. Editor scope marks align with the table columns. Footer actions have consistent width/hierarchy.
- Existing focusable_content owns single pointer/KeyUp activation for names/navigation/scopes/remove; marks are rendering only, no nested Tab stops or vendor patches. Unchecked mark uses existing semantic muted border for legibility. No new palette values.
- Long names truncate within their flexible column; empty and many rows stay bounded; pending scopes/navigation do not dispatch. Draft/add/edit/remove/cancel/error/single-save and correlated consumer behavior retained.

## Verification and actual inspection
Before correction the production regression failed at the required explicit32px application field. After layout implementation14 owner tests passed except the old28px cancel expectation, then updated to the intentional32px contract. Added actual GPUI header/label/marker/editor centers (within1px), row vertical centers, input/button height/center, rail width/inset, long-name/many/empty/busy containment tests; no clipping assertions removed.

Full Desktop485/485 passed with0 failed/ignored, production/test strict Clippy, workspace --tests, fmt and Debug build passed. Token gate subsequently found3 literal radius values; equivalent RADIUS_CONTROL replacement retained visual behavior. After replacement owner15/15, strict test Clippy/fmt/Debug build reran passed; full485 belongs immediately preceding equivalent-token change. Eight source gates pass: ui-token-boundaries, ui-consistency, ui-state-lifecycle, desktop-shell-lifecycle, clipboard-ui-lifecycle, quick-launch-lifecycle, download-ui-lifecycle, ui-preview-isolation. Spec/diff checks run at final closeout.

Owned Debug synthetic native100%720x480 screenshot uses production NetworkSettingsView, no user config/platform/service startup. Browser actual image inspected; first new render showed aligned grid but insufficient unchecked-marker visibility, corrected with semantic border, then recaptured. Final `artifacts/ui-global-blacklist/global-blacklist-native-100.png`:17219 bytes,SHA256A3318DC84215BF94445601708DDBAA8458A3CDFD03A1CA0BA8BA8E551C3D5C0F. Browser gallery scales the image down; the native file retains720x480. Screenshot is layout evidence, not installed/mixed-DPI/accessibility acceptance.

Scratch logs: global-layout-red.log, global-layout-focused.log, global-layout-final-{suite,clippy,production,workspace,build,fmt}.log and global-layout-token-{focused,clippy,build,fmt}.log. Serialized Cargo jobs1/incremental0/codegen-units256; no duplicate build/timeouts.

## Delivery limits
No packaging/install/normal product runtime/service mutation/real Clipboard capture/DOpus navigation/user URL/commit/reset. Existing19:30 installer retains rejected old layout; only Debug preview/source is updated. Native screenshot/UI visual acceptance and125/150% installed evidence remain separate. Other desktop surfaces and pinned GPUI provider limitations are not declared complete by this local correction.
