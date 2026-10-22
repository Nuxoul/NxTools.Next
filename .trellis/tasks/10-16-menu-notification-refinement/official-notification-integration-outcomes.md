# Official notification integration — source outcome (2026-10-17)

## Outcome and authority

Implemented the user-authorized adoption after the independent locked0.7.1 capability proof. The user accepted official fixed5seconds, whole-list hover/focus remaining pause, hover-only close and application-owned bounded admission/business guards. These supersede custom GroupBox/Alert notification cards and configurable1..60/default6 timing. No dependency, registry/vendor/framework or native protocol changes were needed. This is source/isolated Debug verification, **not a new Release package or user acceptance**.

## Delivered

- Actual released `Notification`, `NotificationList` and underlying `ToastStack` own card paint, stacked/expanded placement, countdown and enter/exit completion. `NotificationCenter` holds max3business records, IDs/generations/sources/actions only: no parallel expiry tasks, dwell/hover clock or custom card animation.
- Three reusable official slot IDs bound retained display entries, not merely the visible max_items. Same-invocation progress/result uses one business ID/new generation; unrelated invocations no longer overwrite one broad category ID. Extensions enable/disable/save/import, Quick Find, Clipboard and relevant Downloads terminal feedback carry invocation-scoped identity; existing explicit manual/native operation IDs remain.
- Public content slots contain scrollable text/details and at most2real outlined Small24Buttons. Official border/backing/icon/hover-close/stack grammar stays unchanged. Actions claim exact live generation/nonexiting/unclaimed once, then defer invocation on original source window; retired close callbacks cannot remove replacements.
- Global Settings truthfully displays **5 秒（官方通知默认）**, read-only, with group-pause explanation. Legacy Core `notification_duration_seconds` remains readable/preserved by unrelated saves but does not affect official timing; Cancel writes nothing.
- Existing single passive screen popup and C# source-monitor/DPI/style/readback ownership remain. Hidden final placement -> explicit correctly sized draw outside leases -> deferred reveal is preserved. Visible expansion/collapse uses keep_visible resize, not hide/reveal; pointer and keyboard get sufficient native viewport space. Current-generation content focus/measurement only allocates native space, never drives official focus/expiry/card offsets. Focused retirement and empty/reopen release stale expanded allocation.
- Position-only changes use official global placement while retaining the same notification entity/remaining clock. Native motion snapshots/errors plus unknown/startup/disconnect also reach GPUI reduce-motion. Contextual validation/trust/durable task/query state, original business effects and W10 deferral remain unchanged.

## Measured regressions and corrections

- `official-notification-integration-card-red.log`: before integration actual official-card count0vs1. Current real RoleAlert/card assertion passes; this is not an imports-only gate.
- Earlier independent-operation diagnostic showed two Extensions commits both replacing `extension-management`, expected2/actual1. Real consumer tests now require independent commit notifications while preserving same-invocation result replacement and duplicate-terminal guards.
- `official-integration-stack-final.log`: actual official action focus reached, native client stayed158->158. Replaced ineffective outer focus-event tracking with generation-scoped public-content focus sampling; mouse and keyboard viewport regressions pass, including blur/collapse and focused action retirement without a separate blur event.
- One bounded independent read-only review found position-only repush resetting countdown and missing allocation reconciliation. Added near-expiry regression before the fix: `official-integration-review-regressions-red.log` failed that assertion (24pass/1fail). Official global placement preserves entity/timer; `official-integration-review-regressions-green.log`:25pass/0fail, including actual mouse/keyboard/empty-reopen lifecycle. Focused-close test already passed before that reconciliation addition; do not misreport it as a measured red.
- Initial integration failures from normal-flow overlay layout, render-stage mouse listener, obsolete custom selectors and authoring mistakes were repaired at their actual boundaries. Overlay is absolute; native sizing listener registers in paint; tests observe real official roles/controls. Retired custom transition helper/tests are not evidence for official animations. Final mid-exit test toggles the actual GPUI official reduce-motion setting as well as existing application policy.

## Verification performed on final source

Rust1.96, serialized `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256` throughout. No concurrent Cargo or packaging.

| Check | Result / session scratch log |
| --- | --- |
| Full Desktop after final product and test changes | **799passed,0failed/ignored**,88.78s; `official-integration-handoff-desktop.log` |
| Core | **58passed,0failed/ignored**; `official-integration-exact-final-core.log` |
| Protocol | **48passed,0failed/ignored**; `official-integration-exact-final-protocol.log` |
| Scoped Desktop tests Clippy `-D warnings` | PASS; `official-integration-handoff-strict.log` |
| Workspace `--tests` / fmt `--all -- --check` | PASS; `official-integration-handoff-workspace.log`, `official-integration-handoff-fmt.log` |
| Final production Debug build | PASS; `official-integration-final-debug.log`; existing unused `ClipboardApplication::extension_adapter` warning remains unsuppressed; later edits are test/docs only |
| C# safe fixtures via actual Tests.exe `--fixtures` | **256selected/run/pass,0skip/fail**; `official-integration-exact-final-csharp-fixtures.log`; no C# product changes in this integration |
| Eight source gates | PASS: Clipboard UI, UI state, preview isolation, motion, Desktop shell, Download UI, Quick Launch lifecycle, PlatformHost test selection; `official-integration-exact-final-gate-*.log`; final updated motion gate also passes |

Final handoff checks PASS: spec94documents/13owners, catalog79pages/78locked entries, contexts13implementation/19check unique existing files, all8source gates and scoped whitespace. PNG dimensions match360x158/306/158; collapsed/restored have identical SHA256. Installer15 hash/size unchanged; zero Cargo/rustc survivors and final fixture PID retired. Logs: `official-integration-handoff-{spec,catalog,context}.log` and `official-integration-handoff-gate-*.log`. The existing locked-index context-size warning remains (48,744bytes vs32,768limit); bounded source reads were used without changing limits. These document/source checks do not certify native/visual/user acceptance.

## Real owned GPUI -> production C# fixture

Final Debug fixture: `official-integration-native-601c9e26265d4226b892725d5b8d566b`, synthetic source HWND136232/PID26080. Three actual official cards on one passive HWND. Captured native client heights **158 -> 306 -> 158**, width360/96DPI; outer/client equality and absence of native nonclient frame bits asserted. **One reveal, two visible keep_visible adjustments, one final hide**, foreground unchanged. Initial three hidden preparation commands settle first-scene sizing before the one reveal; no hide/reveal during interaction. Exact fixture process is retired by the owned-path runner. `fixture.log`: runtime=false/capture=false/indexer=false/platform=false.

Production pointer input is synthesized only inside the isolated fixture, not a physical user mouse. Keyboard expansion/retirement/blur and empty/reopen use production GPUI TestPlatform plus fake native viewport acknowledgements. These are separate evidence tiers, not a claim of real OS keyboard navigation into a nonactivating installed popup. Official animation real-time versus virtual-clock differences are respected; mechanical geometry tests use released reduce-motion.

[Three actual HWND captures and native evidence](../../../artifacts/official-notification-integration-ui/index.html) are archived. Browser screenshot viewing succeeded in this turn, and the three-image page was directly inspected: expanded three-card separation, both first-card actions contained, collapsed/restored shape consistent. No user approval or complete motion-frame/physical input/DPI/reader acceptance is implied.

## Release and remaining acceptance boundary

Latest delivered installer remains sequence15,18,498,970bytes, SHA256 `B373A3BD3DE3A672CA5E6B99967F2893F33563FF74EB9A8F6A3DC11E999EB506`, unchanged. It contains the previous custom implementation, **not this integration**. No release sequence reservation, packaging/install, ordinary product launch, deployment/service operation, real user clipboard/config/history/plugin effect, commit/push/archive was performed. Existing dirty user work is retained.

No remaining source/test blocker. Installed physical pointer/keyboard, mixed-DPI/multi-monitor, reader/provider and user visual acceptance remain explicitly unperformed. Future Release packaging requires a separate request and next valid sequence; sequence15 must not be presented as containing this source.
