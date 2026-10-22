# Design boundary

Existing ui/preview.rs renders text literally and estimates plain line heights. Keep ClipboardApplication and C# native owner/lifecycle. Add a private preview_text module for bounded conservative detection and Markdown AST sanitization, plus official component state prepared on item/mode change. No storage, protocol or capture change.

## Component selection
User reference: existing compact passive hover preview, not a new editor/workbench. Locked Kit0.7.1 component::text::{TextView,TextViewState} for rendered Markdown and input::{Editor,EditorState} for readonly code. Code Editor retains released typography/frame; no appearance(false) repaint. Official Small outlined Button and Popover for automatic/source/manual language choices, directly composed (not Toolbar.child). Neutral Kit theme retained. Source/uncertain text remains original literal presentation. Public APIs source-verified in component input/editor.rs,text/compat.rs and Base input/state,text/state; Editor internally renders documentation using TextView::markdown. Kit has no public arbitrary-snippet language detector.
Locked0.7.1 actual grammar regression found TSX uses TS-only captures (no genericJS), C++ requires C/base query inheritance, and C# config has empty highlights. Public LanguageRegistry::register supplements TSX from existing TypeScript captures+project JSX captures, C++ from existing C captures, and C# from original project captures; parsers and official HighlightTheme remain upstream. No registry-source patch, dependency upgrade or custom painted editor; every selectable language produces real parse tree and styles.

## Preparation/safety
Local byte/line/depth budgets. Classify only full hydrated content, not400character summary. Explicit fences/shebang first; distinctive token combinations then confidence/tie rejection. Generic expressions/prose fallback. Markdown uses existing locked markdown1.0 AST to replace Image/ImageReference with literal labels and Html with escaped literal source before official renderer; links are inert in passive preview, no cx.open_url. No MDX, resource fetch or execution. Preserve exact original ClipboardItem text; manual mode changes only presentation.

## Expected files
Desktop Cargo.toml enables selected official grammar features; root workspace markdown dependency reuses existing locked version. ui/preview.rs + new preview_text files/tests; constructor call sites only if window-bound state requires them. docs/reference-code/clipboard-clipkeep-tiez.md records newly verified evidence before product edits. Dedicated task/spec outcome.

## Lifecycle and diagnostics
Existing preview.hover_ready correlation and native stage events retained. Record one prepared mode/language/fallback/count/elapsed transition per new item/mode, no content/path/URL logs or frame logs. Entity callbacks update local state then defer cross-owner work, including cached results. Retain same window/entity on item changes, reset internal scroll/selection, late hidden/cancelled preview cannot reappear. Native state/focus is not owned by renderer.

## Verification checkpoint
Initial5newtests executed red after dependency recovery; final10rich-preview regressions and fullDesktop832passed. StrictClippy/tests-Dwarnings,workspace--tests,architecture/fmt/spec gates passed. No native/product/screenshot/physical/DPI/reader acceptance performed. Dependency source unchanged; official sparse-index cache metadata was refreshed from canonical index with backup to scratch when ordinary Cargo saw stale versions, missingcrateSHA256checkedagainstCargo.lock. See outcomes.md.
