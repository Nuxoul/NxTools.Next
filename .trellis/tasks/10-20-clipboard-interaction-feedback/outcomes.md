# Clipboard interaction feedback outcome

## Delivered source behavior
- One official Small ghost Pause/Play icon Button at the Clipboard top header's far-right. Recording shows Pause; click requests runtime pause and shows Play; next click resumes. No duration Popover or separate status/button row. Immediate pending protection, native ACK-only applied state and explicit retryable failure remain; original C#/protocol capture gate, privacy/reconnect/no-backfill are unchanged.
- Ctrl+number / page shortcuts no longer require search-only focus. Number keydown locks the immutable item ID/query generation; matching keyup dispatches once even if Ctrl was released first. Capture-phase keyup prevents child handlers consuming it. Legacy empty-search1..9/0 maps to the first10; nonempty-search digits remain text. Editor/dialog/context-menu/IME/pending/query/repeat protections remain, as does existing Button Enter/Space ownership.
- Preview ordinary prose/Markdown defaults to official TextView without clicking Auto/Language. Confident code retains readonly Editor highlighting; optional source/language inspection remains. Original payload stays separate.
- History text summaries render safe Markdown through official TextView, with a mounted keyed parse cache reused across repaint and invalidated on source change. Only400characters are prepared; bounded ordinary/heading/multiline content heights24/40/96DIP and four rendered lines preserve outer list scrolling. Original complete text/native formats are hydrated by ID for actions. Images/HTML pass through existing safe projection; no implicit resources. Row body and rendered link clicks both invoke original item action, never link navigation.
- Header icon layout intercepts left MouseDown so it cannot enter native window drag; blank header keeps its original gesture. No framework/dependency/native/storage rewrite.

## Diagnosis and red evidence
- `clipboard-feedback-red2.log`: actual Ctrl-first number release and legacy bare digit did not start paste; default italic/inline-code/numbered-list preview had no official Markdown state; history lacked a Markdown renderer. The initial focus test itself used an invalid keyed-state lookup and is NOT product-red evidence; corrected actual focus traversal is covered in the final tests.
- `clipboard-feedback-link-red.log`: real rendered link click produced no PasteText (0passed/1failed). Official TextView intercepts link clicks, so a no-op on_link_click created a dead history hit region. Public callback now reuses original action with current modifier policy; a weak entity bridge satisfies its Send+Sync API without a self-cycle or leased dispatch.
- `clipboard-feedback-header-drag-red.log`: actual header handler was called1time on Pause press (expected0). Official Button avoids pointer focus but did not stop the press bubbling to native move. Final slot isolation regression passes icon0/blankheader1.
- Intermediate new-test API compile errors and fixture issues (RenderedText::as_str, Styled return, old single-line assertion, ACK not delivered to application, nonunique row selector) were corrected, not counted as behavior passes. A direct Rc link callback failed Send+Sync compilation and was replaced with the weak entity bridge; no warning suppression or upstream patch.

## Final verification
Workstation Cargo policy used for every Rust command: jobs1, incremental0, RUSTFLAGS=-C codegen-units=256. All-machine preflight observed another project's Cargo work at several points; waited without stopping/contacting it. No duplicate builds started.

- `cargo test -p nxtools-desktop --locked --offline ui::efficiency_tests:: -- --test-threads=1`:20passed/0failed/0ignored (`clipboard-feedback-closed-panel.log`). Assertions include typed PasteText/target/fulltail, Ctrl-first release/focus/legacy/nonemptysearch, generation/held/IME/pending, pause/appACK/resume/failure retry/duplicate exclusion, official assets/top-right geometry, Markdown rendered_text/cache invalidation/resource projection and row/link pointer actions/header drag isolation.
- Preview16/16 and history16/16 passed (`clipboard-feedback-final-uipreviewtests.log`, `clipboard-feedback-final-uihistory.log`); full final run below includes these again.
- Strict Desktop Clippy with tests and -Dwarnings passed (`clipboard-feedback-closed-clippy.log`).
- `cargo check --workspace --tests --locked --offline` passed (`clipboard-feedback-closed-workspace.log`).
- Full `cargo test -p nxtools-desktop --locked --offline -- --test-threads=1`: **866passed/0failed/0ignored/0filtered**,116.32seconds (`clipboard-feedback-closed-desktop.log`). Earlier integrated865run passed before the header regression was added;866is final evidence.
- Source gates, scoped format/diff, document/catalog consistency and unchanged Cargo.lock are recorded in `clipboard-feedback-closed-static.log` after final source changes. These gates establish source/document structure only.

## Boundary / remaining acceptance
No package/install/product launch/service operation/live clipboard/config/history mutation. No delegation/cross-session contact, commit or task archive. Existing unrelated dirty work remains. No C#/Protocol/currency/notification/QL/thumbnail-original changes in this correction. Cargo.lock is unchanged from feedback baseline.

Physical keyboard/Windows IME, actual native target paste consumption, installed version, pointer on real HWND, supported/mixed DPI, screen reader, screenshots and user visual acceptance were not performed. Virtual production GPUI/typed fake IPC/source gates do not certify those. The user-reported installed shortcut symptom is addressed by reproduced source gaps, not claimed reproduced on their installed binary. Full bounded list preparation has no new benchmark/soak claim.
