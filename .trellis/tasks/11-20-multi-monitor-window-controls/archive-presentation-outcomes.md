# R3 archive progress foreground

## Behavior

- The original captioned Kit Root, job view and batch surface stay in place. First show and intentional reuse defer the existing typed `ActivateWindow` request. A retained, visible single-job completion or the final worker terminal of a visible batch requests foreground once on that same window. Intermediate batch terminals, progress ticks, minimized completion, auto-close success and background terminal do not request foreground. Existing decision-prompt restoration remains interactive and separate from ordinary updates.
- One pending activation per surface is registered before dispatch. It binds logical job, selected batch member, original GPUI window/HWND, attempt/generation and platform connection. Retirement, supersession, disconnect and the IPC router's terminal timeout clear that ownership. An old result cannot restore a window or consume a replacement receipt. Native confirmation and activation failures are independent of archive results, cleanup and result-location behavior.
- `archive.job.activation_accepted`, `archive.job.activation_started` and `archive.job.activation_finished` share the command UUID used by PlatformHost. The terminal log includes outcome, elapsed time and the native error code when supplied. No private request data or progress-tick logging is added.

## Boundaries

- The [archive reference](../../../docs/reference-code/archive.md#12-whole-product-native-reference) retains Bandizip's indexed owner-bound progress and terminal HWND lifecycle and the legacy NxTools archive-owner absence evidence. No additional native analysis or behavior equivalence is claimed.
- `rust/apps/nxtools-desktop/src/archive/jobs.rs:1468` consumes matching receipts through the existing ArchiveApplication notification path; `:2288` and `:2366` own presentation and retirement. No `application.rs`, dispatcher, protocol, IPC or C# edit is needed. `job_activation.rs:52`, `:92` and `:154` own admission, dispatch and confirmation; `:221` gates visible terminal presentation.

## Verification

Passed: scoped `rustfmt --edition 2024 --config skip_children=true --check` on `jobs.rs`, `job_tests.rs`, `job_activation.rs`; scoped tracked `git diff --check`; `python tests/architecture/gpui-release-contracts.py`; `python tests/architecture/markdown-editor-contracts.py`. These check formatting/whitespace and the locked GPUI/editor source boundaries, not behavior.

`r3_archive_` passes 3/3. The regressions use the actual ArchiveApplication, GPUI TestWindow and production bridge with only HWND/native results supplied synthetically:

- `r3_archive_create_show_reuse_and_visible_terminal_request_native_foreground` — show/reuse, confirmation/refusal, no tick dispatch, once-only retained terminal and same window (`job_tests.rs:5580`).
- `r3_archive_batch_foregrounds_only_its_retained_completion_and_ignores_retired_receipt` — one batch window, no intermediate activation, final sibling and old receipt after explicit reopen (`job_tests.rs:5675`).
- `r3_archive_timeout_disconnect_background_and_auto_close_do_not_reactivate` — timeout receipt/disconnect, retirement before deferred dispatch, connected background terminal and success auto-close (`job_tests.rs:5733`).

The integrated run also passed `progress_caption_close_backgrounds_without_sending_cancel` and `opt01_ac01_native_close_restores_displayed_prompt_and_reply`. Physical Windows foreground/readback (including refusal, minimize, cross-app interference, open-after and mixed DPI) and packaged acceptance were not exercised.
