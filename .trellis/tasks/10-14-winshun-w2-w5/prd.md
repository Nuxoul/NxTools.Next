# W2–W5 approved search extensions

User approves W2 through W5 as listed. Canonical order: W2 no-result homophone fallback; W3 separate file/folder candidates; W4 useful recent-modified list; W5 fast-input execution guard. W6–W10/plugin runtime remain deferred. No package/install/service/real scan/user history modification/commit/push.

W2: ordinary searches complete first; only a zero-result final set can trigger <=8 bounded prepared pronunciation variants. Transform ordinary positive Han name terms, retain explicit/excluded/path/parent/drive grammar, max63ASCII spelling bytes. Reuse W1 shared query matcher and reviewed data. Same normal ranking/results remain unchanged; approximate results labeled explicitly using existing metadata. Existing generation/cancellation owns fallback.

W3: reproduce file candidates excluding all directories before limit, then retain bounded per-kind candidates across parallel/group/delta and final merge. Preserve strongest/exact results, directory/file-only scopes and deterministic in-kind order. No generic ranking framework or format changes.

W4: add explicit `recent:` Quick Launch query for recently modified indexed files, separate from existing recent-directory visit history. Ignore unknown/future timestamps, temp/lock/build/cache noise only in this view; bound per-folder/project representation using snapshot metadata, not filesystem reads; normal explicit searches retain every indexed file. Keep limits/delta/exclusions/cancellation.

W5: pending new-query results cannot execute old committed rows via Enter, shortcuts, pointer or action mode. Choose rejection (visible progress/retry), not automatic held execution. Preserve IME gate and current-session dispatch; red tests at actual InputEvent/action boundary. No duplicate keyboard owner.

Owners: Finder query/pinyin/ranking; Indexer snapshot candidates/recent filtering; Desktop existing query/search session/result presentation/input dispatch. Reuse official component metadata/status, no UI redesign/dependency changes. Define generation/stage/count/elapsed events before completion, no query/name/path payload logs.

Verification: red/green real Finder external merge and synthetic Indexer base/delta/group/cancellation; Desktop input/dispatch/presentation tests, full safe suites,strict lint/workspace--tests/fmt/source/spec gates. No Listary native source available; WinShun source read-only. Release/native latency and LocalSystem acceptance remain separate.
