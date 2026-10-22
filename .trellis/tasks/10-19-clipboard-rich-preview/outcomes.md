# Clipboard Markdown / automatic code preview — source checkpoint

2026-10-19. User approved implementation and local task records. This session alone performed the work: no delegation/cross-session messages. Existing dirty tree and prior first-tier slice preserved. No commit/task archive/package/install/service/product launch/real clipboard or history operation.

## Delivered
- Existing passive hover preview classifies ordinary text, clear Markdown and distinctive code snippets locally. Ambiguous/conflicting/unsupported text remains literal. Not a compiler, formatter or universal language identifier.
- Official Kit0.7.1 TextView/TextViewState renders safe Markdown; official readonly Editor/EditorState shows code with syntax highlighting. Kept released component theme/chrome, no WebView/custom painted substitute.
- Small outlined source/auto and language controls;15manual choices: Rust,Python,JavaScript,TypeScript,TSX/JSX,C,C++,C#,Java,Go,JSON,SQL,Shell,HTML,CSS. Manual setting affects current view only; item replacement resets Auto/state/scroll. Deferred callbacks carry appearance generation.
- Full original ClipboardItem text remains unchanged for storage, edit, copy and paste. Lists remain lightweight summaries and do not run rich parsing. No C#/protocol/storage/capture changes; ClipboardApplication differs from this task's baseline only in constructor/set_item Window/Context plumbing.
- Rich parse input/output<=64KiB/1200lines,noNUL; AST<=32depth/8192nodes, pathological quote nesting rejected before parse. Image/ImageReference replaced with literal not-loaded labels; rawHTML escaped; projection reparsed to reject remaining resource nodes. No implicitHTTP/file resources, MDX/JS execution or formatting. Links are inert in passive preview. Unknown code-fence language removed from rendering projection to avoid raw names in highlighter diagnostics; known aliases canonicalized and unlabeled code blocks may use conservative local detection. Original source remains exact.
- Pending asynchronous TextView parsing shows original text rather than an empty preview. Long Markdown uses official internal scroll; manual menu height derives from available preview budget. Existing singlepassiveHWND/nativeACK/noactivate/finalbounds/frame/reuse/hovercancel owners retained.

## Official integration defects found and repaired
The locked released Kit highlighter does not guarantee every enabled grammar has useful captures. Actual regression found TSX lacked genericJS captures, C++ lacked inheritedC captures, C# capture query empty. Use public LanguageRegistry::language/register once: TSX merges existing TypeScript query+projectJSX captures; C++ merges existingCquery; C# original small project query. Upstream parsers, source and theme unchanged. Every manual choice now produces an actual tree and styled ranges; no imports-only/support claim.

## Verification actually performed
| Gate | Result |
| --- | --- |
| Initial new red-capable regression run | 0passed/5failed as expected before implementation |
| Final rich preview regressions | 10passed/0failed/0ignored,1.29seconds |
| Earlier focused original preview suite | 12passed/0failed |
| Existing preview presentation lifecycle | 12passed/0failed, including noactivate/bounds/show/failure/cancel/reuse |
| Full Desktop suite (final implementation) | 832passed/0failed/0ignored,118.72seconds |
| Strict Desktop Clippy incltests | pass,-Dwarnings |
| cargo check --workspace --tests --locked --offline | pass |
| Spec contracts after final contract append | pass,106documents/14owners |
| Scoped rustfmt / whitespace / new debug-marker and component-suppression scan | pass,exit0 |
| Clipboard UI/lifecycle architecture source gates | pass |

Actual GPUI tests assert parsed rendered_text (not merely selectors), real readonly rejection, actual grammar tree/styles for all15languages, pointer source/manual menu changes/entity preservation, longscroll-to-end/metadata bounds, fakeHTTP client requests=0. Fixtures only/synthetic content. Existing preview nativecommand/controller tests are not an actual live HWND/physical gesture.

Logs in session70da3267scratch: clipboard-rich-preview-red-executed.log; clipboard-rich-final-focused/clippy/workspace/desktop.log; clipboard-rich-boundary-*.log. AllCargo commands jobs1/incremental0/RUSTFLAGS=-C codegen-units=256; wait/preflight all-machine lane, no other process killed. Busy-lane attempts stopped before startingCargo. PowerShell compilerstderr NativeCommandError wrappers are not test failures when actual exit0.

Failures retained: initial offline index lacked tree-sitter0.26.13; wire/dependency failure not a TDDred. Then actual initial5tests all failed on missingclassification/safepath/realrenderer. OneC++CSSrulecollision fixed (exclude#include); Rust std::vsC++conflict fixed. Constructor/ElementId/getter test API mistakes corrected. First strictlint uselessconversion fixed. Subsequent all-language tests went red for TSXthenC++ before query inheritance fix; no suite run was hidden or called green before repair. FullDesktop ran once at final implementation and passed.

## Dependency/environment evidence
Kit stays exact0.7.1; markdown1.0.0 reused already-locked parser. Enabling selected official grammar features adds18lockedpackages/193lockfilelines; no existing package version changed. Tree-sitter0.26.13 officially exists/nonyanked but ordinary Cargo/indexrefresh saw0.26.11. Canonical crates.io API/index HTTP200 verified; existing local sparse metadata backed up to scratch/rich-index-backup then refreshed from official JSON (no config or dependency-source edits). Missing grammarcrateSHA256 checked againstCargo.lock before placing in normalCargo cache; oneTLS interruption resumed only missingTypeScript archive, no TLS verification bypass. Failed scratch isolation prototype never executed and changed no cache. Rust/grammar notices added; all new runtime/grammar crates reviewed MIT.

## Remaining acceptance / exclusions
No compiled/static blocker remains. Live passive preview pointer/keyboard/native primary-focus behavior with interactive controls, Light/Dark visual review at small/long/error states, mixedDPI,UIA/reader and installed runtime acceptance have not been performed. No screenshot is claimed approved. Code detection is conservative finite heuristics: very short/incomplete/mixed snippets may stay plain and manual selection remains available. Exceeding safetybudgets stays source. No automatic codeexecution/formatting, richlistrows, editor-save migration, remote image allowance or HTML documentpreview added.
