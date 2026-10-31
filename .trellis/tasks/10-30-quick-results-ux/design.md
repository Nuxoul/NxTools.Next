# Quick Launch result interaction contract

## Scope
Quick Launch work concerns search results and their behavior, not a launcher UI redesign. Archive workflows own the UI/UX repair scope. The search engine, ranking, indexing protocol and native lifecycle are unchanged by this slice.

## Result rows
Keep the existing 620 × 44/410 launcher, 40-DIP result row, 26-DIP icon lane, 48-DIP shortcut lane, external0/4-DIP gap and one uniform-list scroll owner. The result name and original subtitle/target remain the two text lines; Ctrl+1–9 stays in the trailing lane. Do not add a repeated row action button or duplicate the parent folder in path copy. Full names and targets remain available through the row's accessible description and supplementary Tooltip.

Secondary actions use the original right-click or bare Right path into the existing searchable nested action mode. No new PopupMenu/Command controller, action toolbar, hover-only command strip or native owner is introduced. Enter and modifiers continue through the original typed execution pipeline.

## References and lifecycle
Listary managed SearchMode.cs701–758, SearchItemList.cs39–63 and SearchWindowBase.cs268–286/323–348 establish committed result replacement, stationary-pointer selection protection and native show/focus ownership. SwiftList QuickSearchWindowInputHandler.cs38–50/96–139, QuickSearchWindowController.cs160–206 and QuickLookManager.cs210–279 confirm action/IME, persistent window placement and preview reuse. These are UI/managed boundaries, not Listary native search-engine source.

Same-query updates retain selected identity without recentering an already visible selection. Previous-query retained rows remain noninteractive, current available rows remain usable, and errors cannot enable old rows. Preserve the65ms coalescer,200-result cap, IME/escape/action stack, deferred cross-entity updates, one passive preview HWND and correlated native success-only close.

## Verification
Use the production result_ux tests and existing Quick Launch aggregate for absent repeated buttons, original path copy/shortcut rail, right-click/Right/Enter, identity/scroll, stale query and preview lifecycle. Source gates, compiled TestWindow behavior and real native/installed verification remain separate evidence levels.
