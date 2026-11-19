# Design

## Reference
ABDM AppComponent completion callbacks and BaseSingleDownloadComponent per-item override were inspected first. Legacy MainWindowViewModel.HandleDownloadCompleted is per-task, while Mister progress.rs completion and queue.rs completion actions are behavioral references only. None supplies durable batch identity. Next already coalesces terminals for400ms through CompletionActionCoordinator and presents through one passive NotificationCenter surface.

## Membership and grouping
Record the existing BrowserBatchSubmit correlation as runtime-only membership on newly accepted task state, expose it through the nonserialized DownloadCompletionSnapshot, and preserve it across state clones without persisting it. Duplicate pre-existing handoffs do not join a new batch. No new command/event protocol, SQLite column or persistent setting. Register exact member IDs in the original coordinator before terminal observation; attempt0→first dispatch1 is the same member, but a later retry attempt is standalone and does not replay the settled batch. Settle independent groups when all surviving original members are terminal, removed or cancelled. Removed/cancelled members do not count as success; an empty group emits no notice. Restart seeds historical terminals and never replays them. Standalone tasks retain the original400ms coalescing and passive completion behavior.

## Presentation
Prevent new per-task passive completion windows for known batch members; already-open windows retain their original keyed terminal transition. Emit one text-only summary per settled group using existing NotificationCenter and NotificationList. Standalone notices use the same action-free presentation. Configured completion sound, directory feedback, explicit directory operations and B5 power intentions retain their existing owners; there is no temporary-mute state or setting.

## Native lifecycle and diagnostics
Keep the sole passive popup, hidden creation, final placement then show and focus:false. Grouped feedback carries the original intake correlation through its summary and bounded platform commands; standalone feedback retains its own operation ID. Preserve completion command correlation, timeout, result and disconnect handling. Log group settlement only, without file paths, URLs or titles. No notification-owned expiry timer or second presentation owner.

## Acceptance
Production intake/terminal/notification regressions cover independent interleaved groups, summary exactly once, correct failures, removed/empty/historical no success authority and unchanged standalone behavior. Both standalone and grouped notices have no business actions; configured sounds and permanent preferences remain unchanged. Retain all B5 coordinator and completion-platform lifecycle tests. Native/visual acceptance remains separate.
