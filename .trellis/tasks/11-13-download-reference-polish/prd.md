# Download reference correctness and completion improvements

## Goal
Apply the user-approved download improvements in order, using local ABDM and legacy NxTools first and Mister as a behavioral reference.

## Requirements
1. Reject a web page returned instead of an expected binary download without publishing it or damaging saved resume data; provide a browser action.
2. Distinguish disk backpressure from a slow network connection while preserving bounded memory, written-byte progress and durable checkpoints.
3. Report actual merge/finalization work after transfer; show determinate progress only where a real denominator exists.
4. Summarize downloads added together in action-free completion notices without changing configured feedback, success or power-action authority.
5. Persist source-page URL/title separately from download URL and Referer, retaining edited file names and legacy records.

## Constraints
- Preserve unrelated worktree changes and original Rust/GPUI/C# ownership.
- Reference roots: `D:\Code\ab-download-manager-master`, `D:\Code\NxTools`, `D:\Code\misterdownloadmanager-main\misterdownloadmanager-main`.
- Mister is GPL-3.0-or-later: behavioral reference only; no implementation, tests, comments or assets copied.
- No protocol expansion, tray changes, drag semantic changes, new dependencies, package/install/product launch or implicit commit.

## Acceptance
Each ordered child has a red-capable boundary regression and relevant green owner checks. Integrated HTTP/media/storage/message/IPC/Desktop/browser checks cover the changed data flow. Report compiled evidence separately from native, visual and performance acceptance.
