# Implementation order

1. Verify Listary managed provider/history/comparer and launcher BAML references, preserving existing workspace changes.
2. Add failing Finder/SQLite regression; implement category ranking, identity/history projection and one final merge; run Finder tests and strict lint.
3. Add launcher boundary regressions; implement public-API path ellipsis, launcher-only metrics and generation-safe content size updates with retained preview/focus owners.
4. Converge both slices; run targeted desktop regressions, then full desktop tests, strict test-target lint, workspace test check and relevant source/document gates. Fix concrete in-scope failures without broad re-review.
5. Record outcome and any genuine physical/DPI/native/user acceptance limits. No commit, package, install or service operation is part of this request.
