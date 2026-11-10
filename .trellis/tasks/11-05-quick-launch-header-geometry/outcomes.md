# Quick Launch header and geometry outcomes

## Delivered

- Search header58DIP, official Large Input44DIP,26DIP text with34DIP line height,4DIP vertical input padding and24DIP search icon. The original single input frame, InputState/IME/focus,4DIP horizontal inset and28DIP recent-directory action remain.
- An established ordinary-query viewport keeps its size through intermediate local/index/script batches. Completed providers commit final geometry immediately. First current rows can reveal an initially collapsed viewport; no current rows keeps the search-only shell. Clear, hide, action/recent modes and explicit errors retain direct sizing.
- Current results remain immediately usable; geometry does not follow each intermediate publication. No shrink timer/state remains. Existing C# no-activate bounds and generation/session guards remain authoritative.
-52DIP result rows,32DIP icons, category/history ranking, middle-path ellipsis and compact attached-dialog input are unchanged.

## Reference

Inspected Listary6.3.5.94 launcher BAML→XAML:34DIP input/26DIP text with12DIP vertical margins,58DIP header; SearchMode.cs701–758 completes the provider merge before replacing Results. NxTools preserves its existing early-current-result publication while applying the reference's query-completion boundary to established viewport geometry.

Released Component0.8.0 Input::with_size(Large) supplies44DIP single-line height; Styled text and padding refinements apply after component defaults. No vendor patch or custom replacement input was added.

## Verification

| Check | Result |
| --- | --- |
| `cargo test -p nxtools-desktop quick_launch:: -- --test-threads=1` |190passed/0failed. Header/typed-text containment and input identity/focus; intermediate growth/contraction rejection; immediate final geometry; first-row reveal; original action, selection, preview, IME and stale-result protections. |
| `cargo clippy -p nxtools-desktop --tests -- -D warnings` |Passed. |
| `cargo check --workspace --tests` |Passed. |
| `cargo build -p nxtools-desktop` |Passed Debug preview build. |
| Quick Launch lifecycle gate |Passed. |
| GPUI component catalog/release, Markdown editor provenance, spec-contract gates |Passed. |
| Scoped rustfmt and whitespace |Passed. |

Rust commands used CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0 and RUSTFLAGS=-C codegen-units=256.

## Visual evidence

Startup-isolated `--ui-preview foundation` with NXTOOLS_PREVIEW_QUICK_LAYOUT renders synthetic production views with runtime/capture/indexer/platform disabled. Both owned preview processes were closed after capture. At100% scale the five-result client is620×356; inspected images show an unclipped search line, complete input frame/insets and larger header balanced against52DIP result rows.

- [Light client](assets/search-header-light.png)
- [Dark client](assets/search-header-dark.png)

## Limitations

`tests/architecture/ui-preview-isolation.ps1` stops at its download-specific line44 assertion about predecessor display and disabled URL/paste controls in downloads/browser_handoff_impl.rs and downloads/new_download_view.rs. Those files were not modified in this task. The isolated Quick Launch branch was directly inspected and exercised; the aggregate gate is not reported passed.

Installed-product rapid typing, physical IME, mixed-DPI/native live search and user visual acceptance remain separate from synthetic preview and compiled command tests. No Release package, installation, service operation, commit or push was performed; a whole-Desktop test run is not claimed for this slice.
