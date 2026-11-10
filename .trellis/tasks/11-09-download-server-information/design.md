# Design

## Data flow

Existing routed HTTP probe → selected successful response and its stage duration → optional typed DownloadProbe observation → existing task/store transaction → full/selected task projection → existing DownloadTaskWindow information row.

Response observation is immutable per completed probe. Redirect final host and peer belong to the same observed response. An overdue companion is dropped and cannot overwrite it. HLS/DASH manifest redirects must not combine one response's peer/time with another response's host: capture actual manifest observation when available, otherwise identify the earlier observed probe endpoint accurately. Route is captured from the existing operation's immutable network policy; no reread of mutable global config.

The value is an HTTP probe response duration including DNS/TCP/TLS/server response, not connection RTT. No heuristic quality colors/near-medium-far thresholds. IP unavailable is valid; System route is explicitly a policy. Missing historical data remains 未记录/待探测 according to task state, not0ms. Browser/Blob/no-network tasks do not fabricate an HTTP endpoint.

## Rendering and lifecycle

Keep original task window/entity, existing information layout and released styled components. A compact labelled metadata row uses existing layout/typography, with bounded/truncated main text and an official Tooltip for fuller explanation if needed. No extra window or timer. Layout, focus, pause/cancel, completion and reopen use the current owner. Entity callbacks update local state only; cross-entity work stays deferred.

## Diagnostics

Extend existing correlated probe-completed diagnostic with route kind, peer availability and observed response milliseconds, or one fixed accepted-to-terminal event at the same owning boundary. Do not log host/IP/URL/Header/cookies. Reading/rendering stored observations produces no diagnostics.

## Validation

Add red-capable tests at real HTTP/SQLite/projection and production GPUI boundaries before production changes. Preserve concurrent probe deadlines and request counts. Run one Cargo lane with workstation limits; scoped regressions before broad suites. No deployed application or reference program is launched.
