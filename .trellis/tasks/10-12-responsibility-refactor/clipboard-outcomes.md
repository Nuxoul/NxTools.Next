# Clipboard responsibility refactor outcomes

## Scope and pre-edit evidence

Only `rust/crates/nxtools-clipboard/src/lib.rs`, new private `src/categories.rs` / `src/query.rs`, and this ledger are owned by this slice. `ClipboardHistory` keeps its one SQLite connection; no repository, manager, trait, dependency, native/UI callback, capture/retention change or new operation is introduced.

Pre-edit `lib.rs`: 4,442 lines, SHA-256 `3EACACB19D7E0EDB0FF0F57F40895D6D4097F0FB4D303E0ECDE936552586BC25`. Scoped `git status --short` showed no preexisting clipboard-crate edits; the owning task directory was already untracked and its other files are not touched. The baseline is the actual working-tree file, not a replacement from HEAD. Archive/storage files remain outside edit scope.

Read the spec index, shared architecture/engineering/verification, and all clipboard backend checklist topics. Read the complete `docs/reference-code/clipboard.md`; inspected recorded legacy `D:\Code\NxTools\Services\ClipboardRepository.cs:26-227,271-345,370-387,522-564` and `ClipboardHistoryService.cs:272-400`. The verified chain is history service -> repository recent/popup query, normal duplicate save, mark-used and cleanup. Custom source categories and Next's full usage-order cursor/known-total pagination are existing Next-specific behavior, not inferred legacy capabilities.

## Invariants fixed before extraction

- Same `ClipboardHistory` receiver and single connection, same serialized worker ownership. No new lifecycle/cancellation/placement/focus/native show-hide boundary; popup generation, native action success-before-usage and passive preview responsibilities are untouched.
- Existing category validation, case-insensitive names/source membership, built-in immutability, complete reorder validation and atomic replacement/deletion transactions survive verbatim.
- Content search remains text-only; Extended adds image path, serialized file paths, source process and window title. Do not trim/change query, SQL parameters or query limits.
- Filtered total stays independent of loaded rows; known totals avoid recounts. Offset/backward compatibility remains. Forward continuation uses immutable descending `(is_pinned, last_used_at_unix_ms, captured_at_unix_ms, id)` and one extra row for `has_more`; page sizes stay clamped to 1..=500.
- SQL/index/order/count/cursor/projection and error propagation are unchanged, including existing query-plan and benchmark checks. No temporary-table change or new query strategy.
- Capture identity includes typed payload/native-format equality. Usage, soft-delete, favorite/pin protection, max(7, retention-days) purge and remaining-reference asset cleanup remain owned by root history code.
- All public types/exports remain at crate root. Private modules extend the same inherent impl; only required sibling seams may become `pub(super)`, never crate/public.

## Test organization decision

Keep the existing inline `#[cfg(test)] mod tests` intact. It already collects the cross-responsibility SQLite/archive/storage/retention/cursor fixtures together; this slice separates production responsibilities without relocating or reindenting the entire test suite. Existing test module paths, names, attributes, bodies and source-gate lookup paths remain unchanged.

## Verification restrictions

No Cargo or rustfmt is run by this slice: parent coordinates one combined build. No live clipboard/history, Shell, service or disk scan is authorized. Function/range identity and scoped static diff checks are permitted; compiled behavior remains deferred to parent.

## Exact relocation ledger

All source paths below are under `rust/crates/nxtools-clipboard/src/`. SHA-256 values hash the original complete function declaration/body (UTF-8, LF, from declaration through closing brace, excluding trailing newline/preceding attributes). Every extracted function matches that baseline text exactly after removing only the newly necessary `pub(super)` visibility on three declarations. A separate whole-range comparison also proves that preceding attributes, SQL, parameters and intervening whitespace survived; the existing repeated `query_page` lint attribute is preserved, not silently cleaned up.

| Function | Baseline lib.rs lines | New categories.rs lines | Baseline function SHA-256 |
| --- | --- | --- | --- |
| categories | 861-871 | 12-22 | a2d8f83241185b637eb6684388bae62173ca0a0196ba6070fc34a87d3bbdb08f |
| create_category | 873-891 | 24-42 | ffc2acbd44345f5284ec8fbca99b8a483f787a1938637f1a39b87d8e54febc75 |
| rename_category | 893-911 | 44-62 | e3adc799f752895c36a796807bc89c8b1cb02f3d5b76b2b3304bee2dbbb5ed55 |
| delete_category | 913-931 | 64-82 | 698590cd0164d7e4e25b599351ccf106ab439de1c86b6a793a5e2b16df68a5c1 |
| reorder_categories | 933-958 | 84-109 | ee9def673fb9a60967801eaf1095fbfcf4691c7c4bed59f9f9fdfeac78fa7fa6 |
| category_sources | 960-977 | 111-128 | 8c69dee8aec7153a5e16cc246bffd0af5ebeb0a2a7e5905ff9e785c6e7cd42e1 |
| source_process_names | 979-990 | 130-141 | 5f4526edd43bb991a94fcc112f91a7ed0738c1f452e47c0faefc2c0439221a56 |
| add_category_source | 992-1005 | 143-156 | 1bf4e95df598a54e00e79bbcd40d7b8eba0fffb41feb50bf88373b595e36b818 |
| remove_category_source | 1007-1020 | 158-171 | 339c4d9cfd9a36598824874f02ac6df1e5ff85d26a3f5288a26347832d84e840 |
| replace_category_sources | 1022-1052 | 173-203 | 6a410571014329e984fdddc523092a88b45b2846abf5e29c02fccabf08653740 |
| ensure_custom_category | 1054-1065 | 205-216 | 04b9a4260e058c4f3bea09bd7a4c6c7db1bdc15a387c27f053e742383599fab8 |
| ensure_category_name_available | 1067-1086 | 218-237 | 7dfd53aa515b1356d3c7c87db29ac7fd67071f4511f08f1ec67688f9be6dbf2d |
| category_by_id | 1273-1285 | 239-251 | 0b4b638a14fa868a1dffe13c57f0b4ba9daacf14e24666629d2f36eb04825ae3 |
| map_category | 1880-1898 | 254-272 | 8eecf4c6c5536bdd468fc36db13d5ce5c906b316af24e74c8c6cbc9add62c3cb |
| seed_builtin_categories | 1900-1934 | 274-308 | 6fc0054e3f85482b7b2d339b1695302b69188adde8b780d95eaa5f83af85588c |
| validate_category_name | 2042-2049 | 310-317 | 4d481ed2db1d21cb3e69bbeb34c5282608ef800e61e01eea6ea201e79587584c |
| validate_category_source | 2051-2058 | 319-326 | c80f49bb1d6b94e869949d5e19866d733a47d8a3b608f381aa4b62c7103f313c |

| Function | Baseline lib.rs lines | New query.rs lines | Baseline function SHA-256 |
| --- | --- | --- | --- |
| recent | 759-761 | 10-12 | 947d9079c352744510ad83410795bf51eafb3d743c9c0994559ff78d403c893c |
| counts | 763-783 | 14-34 | 99944d38c862b1ef3f2e0640a972a3fe96adcadfc41e270d33998b3593d168f5 |
| recent_by_kind | 800-813 | 36-49 | 0d345f7496187f19aa888793f61795cd797c73dbf255e28d6da560b7d5a10d4c |
| search | 815-828 | 51-64 | 7fa6a15ace1e678055d2d1009a222b4a202ad08a260b442d5d9cf8d1de450257 |
| search_extended | 830-843 | 66-79 | a6f52726b87735776c647e6346f79e00dc949c0cf1682dc86d02b5471e997f8e |
| search_by_kind | 845-859 | 81-95 | 40432d227913f109ffaad7fae2a0197991bc13018c624503e18eb0c1e04f272d |
| query_category | 1088-1095 | 97-104 | f6220bfa140528de22e8b2a42c68b588b75ac3563e2af7c419931edaa78ec6f1 |
| query_category_with_scope | 1097-1138 | 106-147 | e8f4e2d51c24b3c62435faeaab32c42242288002519317d3033804795397d3fe |
| query_category_page | 1140-1149 | 149-158 | be3f0a386c74d7c81ebc164cea2bde7549eff7e5927e41650b913c93125a4582 |
| query_category_page_with_total | 1151-1169 | 160-178 | d4ebfcaf2eb1f0593ad4a497f7f0d78a1735115fb5f2b7e276da7c2e4ec72035 |
| query_category_page_after | 1171-1189 | 180-198 | 5b220b954a46b9f61ec1f022423d43f003293877b10ade50f975486ef1c4eb9c |
| query_category_page_with_cursor | 1192-1271 | 201-280 | 8002d61401a36a17da338e5d91d30730b96d5a8e04087536e2a46e53102e67b7 |
| query | 1287-1305 | 282-300 | 630cd18999009bdb526e8ef7d172fd59add69a03763f6e6d522472c28ff8549a |
| query_page | 1309-1384 | 304-379 | dd58a8833463a291e0809ff8627b99d7c8956293a463b3e63438904b4b6e8221 |
| query_items | 1387-1437 | 382-432 | c9d2ade8f4d1a9a0002fa797a59235590796f56bd1b7c024ab93983555c4f904 |
| query_items_after | 1439-1500 | 434-495 | bd511d4003c1c784648b544e6c528a55181089af8ea0b9974b08fa02919b85f0 |

## Necessary access changes versus simplification

- Private `mod categories` and `mod query` are added at root lines 2-3. Initialization still owns connection/schema setup; its existing seed call is now qualified `categories::seed_builtin_categories` at lib.rs:574.
- `categories.rs:239` makes `category_by_id` `pub(super)` solely for the sibling query impl. `categories.rs:274` makes the seed helper `pub(super)` solely for root initialization. All other category helpers remain private.
- `query.rs:382` makes `query_items` `pub(super)` solely for the existing inline benchmark's direct query timing calls. Query routing/page/cursor helpers otherwise stay private. Parent-owned private `map_item` and history fields are directly accessible to descendant modules without new visibility/accessor changes.
- Actual semantic simplifications: **none**. This is cohesive responsibility extraction, not query optimization or error/transaction normalization. Existing public wrapper APIs are retained, not replaced with pass-through managers.
- Root production section falls from 2,077 to 1,277 lines. Final files: lib.rs 3,642 lines; categories.rs 326; query.rs 496. Inline tests remain at lib.rs:1278-3642, same `nxtools_clipboard::tests::*` path, 43 `#[test]` bodies, full section SHA-256 `9f558a08cf03df44d6e3f65778babf2fef4a942a13d1136f434713a6f9cabb7b` (UTF-8 LF, including cfg/module).

## Static outcomes and remaining run requirements

- PASS: all 33 relocated complete functions match working-tree baseline; whole-range comparison includes attributes/SQL/whitespace. Remaining root content exactly equals baseline after declared removals, two private module declarations and qualified seed call. No undeclared retained-code changes.
- PASS: `git diff --check -- rust/crates/nxtools-clipboard/src/lib.rs`; independent new-module whitespace/private-boundary checks. No rustfmt invocation.
- PASS: `tests/architecture/clipboard-lifecycle.ps1` (both command-result and reference/regression gates). No source-location update required: lib.rs retains usage/retention and inline identity/empty-capture regressions. `tests/architecture/clipboard-lifecycle.ps1:37,46,93-105,160` still resolves those real owners/tests. No gate assertions weakened or deleted.
- PASS: `python tests/architecture/spec-contracts.py`: 87 documents, 12 workspace owners; static navigation/evidence/contracts only.
- DEFERRED to parent: `cargo test -p nxtools-clipboard --lib` (all 43 existing tests including temp SQLite/cursor/10k+100k fixtures), `cargo clippy -p nxtools-clipboard --tests -- -D warnings`, combined workspace test compile and changed-file formatting check. Parent must coordinate one Cargo process with `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, `RUSTFLAGS=-C codegen-units=256`.
- One intermediate static range checker had an extra expected blank line before query.rs's final brace; corrected the checker expectation, not production code, then exact range comparison passed. No compiled baseline/suite claim is made.
- Post-extraction whole-file SHA-256: lib.rs `bd388cee5a71cf584feec840dccf10e88ea6ad58dd6bc0b7211c9fc71b7b55ff`; categories.rs `32e17bb6058f9828268215a8e7345e9695857f07afafab15cac1502b157dd91b`; query.rs `e0f438947b7065a6f221f946543e33cec6409773cfb3ddd2e0d739e6aad1acec`.
- No live clipboard/history, unrestricted filesystem scan, Shell, native process, service, packaging or installed acceptance was performed.
