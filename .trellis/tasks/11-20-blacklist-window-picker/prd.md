# Blacklist window picker

## Goal
Global Application Blacklist retains manual process-name entry and gains a window-selection button. Hover outlines the target window and displays its executable name and dimensions; left click selects the owning process. Escape or the original button cancels.

## Acceptance
- One original editor and durable Apply/Cancel transaction; selection fills its process field without adding/saving a rule or changing scope checkboxes.
- Button and manual entry remain usable after success, cancellation and failure. Selection suppresses the confirming click into the target application, including its matching release.
- Passive click-through GPUI highlight does not activate target/overlay or change target window geometry. Device-pixel facts and negative virtual-screen origins remain coherent.
- One session per native adapter, request/connection/window/view qualification in Rust. Close, tab change, cancel, disconnect and timeout retire work; late results cannot mutate another draft or show an overlay.
- Source settings window remains available for the Cancel action; native target lookup excludes that window and the highlight surface.
- Correlated accepted/start/terminal elapsed/native-code diagnostics exclude names, paths, titles and pointer streams.

## Scope
Original network_settings application/view, narrow GPUI highlight, additive typed protocol, C# native session/adapter/pipe and isolated tests. No config/schema, blacklist semantics, dependency/theme, indexer, installation or packaging changes.
