# Feedback 6–7 — original built-in preview and modification capabilities

Status: production source and final parent TestWindow/lifecycle regressions PASS; overall feedback/HZ2 handoff remains PARTIAL due scopedfmt, outstanding HZ2 timing coverage and missing divider screenshot. No packaging/install/product/Explorer/physical visual acceptance.

## Baseline and reference chain

Round-start copies and SHA256 manifest: `C:/Users/Administrator/.pi-desktop/scratch/9d8169e5-656b-47ae-9113-af5789f5091c/feedback-preview-capabilities/` (`round-start-hashes.json`, five original owned files). Existing parallel dirty changes are retained. Runtime PID15556 was read-only observed at `C:/ProgramData/NxTools/NxTools.Next/Service/NxTools.exe`, SHA256 `DDD0DA472A50626CD76A2B9032E295617EADA6D4D96242669837188DA42D9F74`; file/product version resources were blank. Parent-provided package provenance identifies Release32 `73b6a27d-d4ad-4c8a-95da-b4eea608183e`; HZ1 accepted18 and this slice are later un-packaged source. This is not a claim that the running installation contains current workspace changes.

Read mandatory AGENTS, shared architecture/engineering/verification/UI selection and design, applicable Desktop/Archive/Protocol/IPC/PlatformHost owners, current0.8.0 release/table/API index, archive reference, browser request/preview contract/completion outcomes, phase checkpoint/HZ1 acceptance, Markdown editor provenance, Quick Launch preview reference. Actual original Archiver `gui/src/main.rs:697–714` remains temp extraction -> success-only external open, not the internal preview owner. Original Next chain reviewed: browser selection/fingerprint -> next-turn ArchiveApplication::preview_entry -> original EntrySession materialization/source/password/cache lease -> bounded preview::prepare/decode -> deferred preview_result -> shared MarkdownDocument or owned RenderImage -> resizable pane. Quick Launch `preview.rs` shares MarkdownDocument/cached_view, not its controller; `ui/preview_text.rs` sanitizes resource/HTML projection, `ui/markdown_document.rs` uses readonly BlockEditor + passive(true) + ImagePolicy::Placeholder. `ui/preview_image.rs::image_plan` includes separate auxiliary-window chrome/modes and is not appropriate to transplant into this existing in-window Contain pane. No new preview engine/decoder/controller/dependency/vendor patch.

Confirmed root causes: PreviewPane default false; discoverability only View -> 只读预览栏; UiState did not carry preview preference. The original content/decode/drawing chain was present but unreachable without opt-in and not remembered. Long-document wheel handling is tested at actual production viewport (programmatic offsets alone are not wheel evidence). Modification-only edit_reason was unconditionally rendered as a global Alert, misleading healthy readable non-ZIP formats.

## Component/API/shape decision

Requested shape: same activating archive window, original tree/details plus optional width-resizable right preview; visible standalone preview toggle, original doubleclick/Enter open. Exact released0.8.0 sources reviewed: `vendor/gpui-component-080/src/toolbar.rs` Toolbar.content vs child/ToolbarItem::sized; `button/button.rs` Button::outline/label/tooltip, Selectable::selected, toggled and official keyboard/focus; `resizable.rs` h_resizable/resizable_panel; `scroll/scrollable.rs` original viewport/gesture wrapper; `alert.rs` actual role and presentation. Toggle is official Medium/default32 outline Button inside existing Toolbar.content custom layout, so no Toolbar.child ghost().compact() adaptation. Existing toolbar64DIP icon commands are unchanged. Preview source/retry/close remain official Small outlined Buttons, Empty/Spinner and original resizable280DIP/200..600 pane. Official Dialog is used only on attempted unavailable modification, not as a persistent read error.

## Retained invariants

- Original BrowserModel is sole selection/focus/anchor owner; local monotonic preview token invalidates ABA; cross-owner request/result/cancel stays deferred. No synchronous application writeback under leased browser.
- Same source/snapshot/generation/ordinal, one native/decode per browser and two global decode permits; actual cleanup/read/decode owns lease release. Selection/close/refresh/source/disconnect error handling and explicit open remain original.
- Text64KiB/1200lines/65536chars strict UTF8+BOM/control checks; image encoded8MiB, axis8192/pixels8Mi, decoder32MiB plus final RGBA32MiB and existing stricter JPEG/WebP admission/static frame. No global RSS guarantee. No script/HTML execution, network/local-resource loading, external fallback or edits.
- Additive UiState show_preview default true only when missing; explicit false survives original bounded atomic writer/restart; original v1/v2 details migration, geometry/history/widths/sort/corrupt preservation remain.
- Ready recognized read formats get exact modification-only copy: “此格式支持浏览和解压，暂不支持修改压缩包内容。” Unknown/not-ready formats do not gain a read promise. Ordinary ZIP structural/application/split checks remain fail-closed. Browsing/extraction/preview do not depend on write eligibility. Unsupported/corrupt/password read failures retain original warning/error state.
- Tab/icon16/20, WAC password, native drag/marquee, HZ1/installer unchanged. No screenshot identifies item5's two lines; no speculative divider repair.

## Execution evidence (raw logs retained)

All logs below are in the evidence directory above; each Cargo call checks no cargo/rustc, sets CARGO_BUILD_JOBS=1,CARGO_INCREMENTAL=0,RUSTFLAGS=-C codegen-units=256 in the same command, --locked --offline, serial test threads and1800s timeout.

| Log / exact test filter | Result |
| --- | --- |
| 01-feedback-red.log / feedback_ | Compiled behavior:61PASS/8FAIL,1377filtered. Own true reds: first-use false, no visible official preview button, original decoder payload cannot reach disabled pane, UiState show_preview missing. Initial capabilities fixture wrongly assumed a Role::Alert test snapshot and raced async ZIP inspection; these two failures are fixture issues, not product reds. Broad substring also selected parent's decimal/HZ2 expected reds2; not claimed as own findings. |
| 02-capabilities-red.log / archive::browser::tests::feedback_capabilities | Compiled2PASS/1FAIL: healthy SevenZ inserted persistent banner, real list top233.5DIP vs healthy ZIP189DIP. Original read failure and split fail-closed guards PASS. |
| 03-browser-green-attempt.log / archive::browser:: | Compile FAIL E0063: existing ui_state_migration_tests full UiState literal lacked additive show_preview; one fixture-only field added, no behavior assertion change. |
| 04-browser-green-attempt.log / archive::browser:: | Compile FAIL E0603 in parallel parent progress_feedback_tests:51 referencing private application::jobs; no preview failure executed. |

Final results, changed-file line locations and acceptance limits will be completed after the focused green runs. No source-string gate, compile failure or zero-selection is counted as behavioral RED.

## Parent convergence (2026-10-28)

The implementation delegate was stopped after repeated parent/concurrent integration compile blockers; no fabricated delegate GREEN. Parent completed fixes and verified `feedback-archive-final.log`:298archive-relatedPASS,0FAIL/ignored. Includes7named feedback_preview and4feedback_capabilities tests, original preview token/lease/decode/cancel/source/disconnect/drag regressions, icon8/WAC12/HZ118. Core93/IPC43/Protocol87, strict Clippy4owners--tests andworkspace--testsPASS.

Actual compiled integration first260PASS5FAIL exposed hidden-tab token not retired, default-preview original-connection and queue-observation compatibility, plus capability Dialog test-node visibility. Parent added immediate local preview suspension/deferred original cancel, atomically origin-bound passive enqueue in original IPC router, official accessible Label and exact opened-lease Release matching. Third run253PASS12FAIL identified old fixtures missing installed origin and obsolete same-token retention assertion; fixtures now install production-equivalent context and require fresh preview token on return. No production safety gate removed. Final298PASS.

Decimal display hints now65.5KB/8.4MB; actual64KiB/8MiB budgets unchanged. Final full results/remaining blockers: [feedback/HZ2 outcomes](feedback-hz2-outcomes.md). ScopedfmtFAIL retained; installed Release32 unchanged; no actual Explorer/DPI/user visual claim.
