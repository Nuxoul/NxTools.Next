# B5 UI and coordinator outcomes

Acceptance: `not_accepted`.

## Official components and original owners

The original440×184 task Root and keyed task/window/generation remain. The Small shutdown Checkbox is retained. An official Small outlined Button/DropdownMenu in `Toolbar.content` offers Shutdown, Sleep, Hibernate, ExitApplication and no action, plus independent directory feedback. Checked `PopupMenuItem` uses its released public action/focus behavior. `Toolbar.child(Button)` is not used for this outlined rail.

The original Settings window uses `SettingItem::render` → `Form::vertical` → `Field` public slots. Each queue has a Small outlined action dropdown, Small directory Checkbox and outlined Save. Drafts remain in DownloadSettingsView until explicit Save; the original Queue command merges changed intention leaves against the baseline within SQLite. Failure retains draft/error. Config/Header/exact-rate/category drafts retain their separate original owners.

The original fixed/minimum450×200 countdown uses DialogContent/DialogFooter and the original titlebar. Heading and execute label identify the chosen action. Cancel/Escape/native close share correlation-scoped cancellation; executing native work owns the action slot. Cross-entity UI effects are next-turn, including immediate paths. Rendering does not query the store/network.

## Single coordinator

`CompletionActionCoordinator` remains the sole terminal-feedback, bounded case-insensitive max-three directory and action-countdown owner. Durable intention is distinct from current attempt/queue round and ephemeral authority. Old terminal records seed observed history, not a fresh round. Nonempty current membership requires actual Completed for every member; waiting/scheduled/failed/exhausted/cancelled/removal cannot satisfy success. Retry starts a fresh attempt; assignment of an old Completed row does not manufacture success.

Ready actions wait while accepted work remains nonterminal. Stable priority is **Shutdown > Hibernate > Sleep > ExitApplication**; callback arrival order does not decide. Directory feedback remains independent. Explicit Cancel consumes that completed attempt's authority without clearing persistent intention. Actual accepted work/retry/membership changes cancel pending countdown through the original event/revision. Drafts/probes/duplicates do not.

Execution rechecks authoritative facts, consumes authority once and retains the original native pending slot. No second countdown is created while that request is held. Shutdown keeps its bounded acknowledgement deadline; Sleep/Hibernate retain their slot through suspension until actual correlated terminal. Disconnect closes the matching executing window and visibly reports an unconfirmed outcome. Native failures retain exact codes, without fallback/retry. Stale callbacks cannot mutate current authority or presentation.

## Production TestPlatform evidence

`b5-desktop-pass5`: LIST22/RUN22/PASS22/FAIL0/SKIP0/not_run0, exits0/0. Selection is `cargo test --locked --offline -p nxtools-desktop --bin nxtools-desktop b5_ -- --list`, then identical target/filter with `-- --nocapture --test-threads=1` under the required single-lane environment.

Coverage includes actual task Checkbox/menu mouse input, first focused Enter/Space and repeated activation, explicit queue Save/SQLite failure retention, real Escape and stale cancel, progress close/reopen/reload and minimum bounds, successful local HTTP writer→typed action once, unsupported50/native87 propagation, held native reply with later successful work, Sleep/Hibernate beyond Shutdown deadline, original typed disconnect and late success, normal TestPlatform quit request plus separately joined resources.

The same selection covers real queue/SQLite failed retry, scheduled/waiting sibling, membership switch, cancel/remove, fresh successful round and engine reload without replay; production new-work events cancel immediately. Original coordinator and preservation selections are recorded separately in `verification.md`/`verification.json`, without adding overlapping counts.

Task persistence, queue invalidation, round/native-slot/probe retirement and native wait/disconnect have recorded behavioral RED → GREEN evidence. Compile prerequisites and test-window fixture lifetime corrections are not behavioral RED. First-pass supplementary lifecycle coverage is identified separately.

## Remaining layers

Physical Windows power/Exit, native OS-close/IME/DPI/Shell, human visual, installed/Release and whole-workspace acceptance are unperformed. Controlled typed success/quit is not physical power or process-exit acceptance. Independent coordinator acceptance remains pending.
