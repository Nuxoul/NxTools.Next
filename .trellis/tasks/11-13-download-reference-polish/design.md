# Design

## Ordered ownership
| Order | Child | Behavior owner |
| --- | --- | --- |
| 1 | `11-13-download-web-page` | Existing HTTP response/write boundary and typed fault, original task window/native browser command |
| 2 | `11-13-download-disk-backpressure` | Existing segmented receive/write and checkpoint owners |
| 3 | `11-13-download-finishing-progress` | Existing HLS/DASH/media process progress and engine projection |
| 4 | `11-13-download-grouped-completion` | Existing batch intake, terminal observation and completion presentation |
| 5 | `11-13-download-page-provenance` | Existing browser request decoder, download request envelope, SQLite and selected details |

Children execute sequentially. They do not replace the scheduler, create another notification manager or grant success authority from failed/empty groups. Later child designs are completed against the then-current source before activation.

## Shared invariants
Task/attempt/artifact/plan identity remains stable. No response byte becomes durable progress before write/sync; final publication remains no-replace and precedes Completed. Pause/cancel drains actual writers. Cross-entity presentation actions are deferred and generation-scoped; closing a task window is not cancellation. Logs use existing correlation IDs, stages, elapsed/native codes and bounded state transitions without URL, paths, headers, Cookie or child output.

## UI composition
Preserve current task-window dimensions and keyed same-HWND progress/completion lifecycle. Add actions through existing official Kit0.8.0 Button/Alert/DescriptionList slots; inspect exact released variants, size and parent adaptations per child. Batch feedback uses the existing official notification owner rather than a second window/controller.

## Verification
Real localhost HTTP, temporary SQLite/artifacts and fixture child processes test behavior. Production GPUI action/bounds/lifecycle tests cover presentation without implying native/user acceptance. Serialize Cargo with workstation memory variables. No release packaging is authorized.
