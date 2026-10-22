# Approved round-2 boundary and execution

## Owners
ClipboardApplication owns preview/HWND/platform event routing; ClipboardPanel owns local selection/hover/input; ui/history.rs owns row presentation, ui/preview.rs content/layout. DownloadEngine/model/store own transfer statistics; Desktop list only presents durable result. C# ExplorerShellActivation owns target observation/native result; Rust artifact_actions owns correlated current-window close. LinkPanelView renders existing timer/session/controller via released components.

## Planned files and invariant
Expect history.rs/tests for action/status rails; ui/preview.rs and clipboard*.rs/tests for actual content size/event ownership; Download task/store/transfer stats and Desktop list projection/tests for cumulative active time; ExplorerShellActivation/native safe fixtures and Rust artifact_actions/tests for result-driven close/copy; Link panel/polish tests for compound/tier use. Each change necessary for listed behavior; not arbitrary refactor. Read recorded legacy/ABDM before corresponding code changes. Add new verified reference chain/invariants before implementation. Baseline dirty snapshot saved in session scratch feedback-round2-status.txt/diff.

## Sequence
Reference/owner and test seam -> red -> smallest fix -> focused green -> full integrated safe tests/strict/check/fmt -> docs. Parent coordinates Cargo only. Independent implementations disjoint, no recursive delegation or duplicate edits. Optional review max1; no broad review restart.

## Diagnostics
Keep correlation/session/generation/request/stage/terminal/elapsed/native-code events, no clipboard text/query/path/title/URL. No pointer/frame/byte-tick logging. Target confirmation and activation outcomes distinct. Transfer stats updated with existing progress/terminal persistence, not new high-frequency logs.

## Components
Complete official Kit0.7.1 index read. History ListItem/Toolbar/Icon preserve original controller and input. Link independent HWND is not replaceable by root-internal Popover/Notification; inspect released List/Alert/DialogContent/StatusBar before final choice. Exact public APIs/decision and actual bounds to be appended after review; unreviewed isn't unsupported.
