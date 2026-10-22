# 剪贴板文本整理

- ID: `example.clipboard-cleaner`
- Version: `1.0.0`
- Runtime: Windows with an already installed Python 3.10+.
- Standard library only: no pip, downloaded interpreter, SDK or dependency.

## Package contents

```text
clipboard-cleaner/
├── extension.json
├── main.py
├── test_main.py
└── README.md
```

`extension.json.files` lists exactly the other three shipped files, not the manifest
itself. Keep this folder independent; copying it does not require the NxTools source
checkout. `main.py` runs only when its manual action is invoked, and exits after one
completion. Importing the Python module for unit tests does not execute the action.

## Behavior / 行为

The only registration is `clipboard/action`, handler `clean`, titled
“整理文本并保留原文”. No automatic event, system clipboard listener, resident process,
network access, background thread, database access or Windows API is used.

The pure `clean_text(text, options)` function applies these steps in order:

1. Always replace CRLF and CR with LF. Other Unicode line separators are preserved.
2. If `trim_line_ends` is true, strip ASCII spaces and tabs from each line end.
3. If `collapse_blank_lines` is true, retain just the first line in each run of
   blank lines. Empty or entirely Unicode-whitespace lines count as blank. When
   line-end trimming is disabled, that first blank line retains its whitespace.
4. If `trim_outer` is true, use Python `str.strip()` on the entire result, including
   Unicode whitespace. This setting can remove indentation at the very beginning.

All three settings default to true. Missing keys use their defaults; non-boolean
values such as the string `"false"`, numeric 0/1, or null are errors, not coerced.
No Unicode normalization, case conversion or internal-space rewriting occurs.

Only changed, non-whitespace results request `clipboard.create_derived` once, with
`source_item_id` and the cleaned text. The original item is retained. This script
never deletes/overwrites items and never requests copying to the system clipboard.
Non-text selections, unchanged text and empty/whitespace-only results complete with
`not_handled`, without a write request. Empty results include a brief explanation.

Input and output text must each be at most **65,536 UTF-8 bytes (64 KiB)**. This is
not a character count. Oversized text fails explicitly, without truncation or text
in the error message. The host's separate 256 KiB JSONL frame limit also applies;
heavily escaped JSON can reach that limit even for text within 64 KiB.

## 安装与手动验收

1. 在 NxTools 左侧工作区“剪贴板”下面打开“扩展”。“扩展”是独立工作区入口，
   不是“剪贴板”的子页面，也不在“其他工具”中。
2. 将这四个文件打包为 ZIP，保留文件名和相对布局，清单位于根目录或单个外层文件夹内。
   在“导入扩展”中选择 ZIP，检查用途与来源，再明确确认导入；不要只导入 `main.py`。
3. 新包默认停用。在“用途与启用”查看手动入口，检查全部文件后确认信任。
   未配置 Python 时，信任窗口会检测 PATH 中已安装的解释器；找不到时可选择已有的
   Python 3.10+ `python.exe`。检测、信任和启用是独立步骤。
   不需要 pip，不下载解释器，不要求管理员运行。
4. **信任不自动启用。** 修改代码后必须重新导入并重新信任，再检查启用状态；
   更新后的包不能沿用旧代码的信任批准。清单和随包文件也属于内容身份。
5. 呼出剪贴板历史，在专门的测试文本条目菜单选择“扩展处理…”，再选择
   “整理文本并保留原文”。本插件只支持手动处理，不会在复制文本时自动触发。
6. 在当前窗口查看这次调用的状态和插件说明；宿主确认保存后可点“查看结果”。
   原条目保留；相同结果可能复用已有历史条目。查看结果不会复制或粘贴。
   本示例脚本也不会主动改写系统剪贴板，后续复制需明确操作。
7. 文本无需整理时不会新增条目。失败、取消或结果未确认时，先检查历史再决定是否重试；
   取消不是回滚。关闭反馈只关闭界面，可在扩展“最近活动”回看本次会话的手动调用。
8. 不需要启用任何自动事件。可在插件设置中分别关闭三个选项，保存后重测。

注意：README 也是清单列出的内容文件，更新此文件会改变包的内容身份。已安装包不会自动改变；
重新打包导入更新后需重新确认信任，再独立启用。

建议只用合成测试内容，例如下面这段 Python 字符串所表示的文本：

```python
"第一行  \r\n\r\n\r\n第二行\t"
```

默认整理结果应为 `"第一行\n\n第二行"`。也可先在单元测试中验证，不必操作真实历史。

## 信任与权限

**信任后的脚本具有当前用户权限，不是沙箱。** `permissions` 是作者用途声明，
不是操作系统访问限制，不阻止任意受信任脚本访问文件、网络或其他用户可访问资源。
本示例代码主动不使用这些能力，不能据此推论其他扩展安全。仅信任已审阅的来源。
宿主的超时、进程和输出限制是资源限制，不是权限隔离。无需管理员权限。

## NxTools JSONL v1

This is not JSON-RPC 2.0. stdin/stdout use UTF-8, one JSON object per LF-terminated
frame. Binary IO avoids Windows locale encoding and newline translation. Every
output uses `ensure_ascii=False` and an explicit flush. stdout contains protocol
frames only. No log files are written. stderr, if needed, contains only a fixed
error category, never selected text, requests, host error payloads or tracebacks.

The first frame must be a v1 `invoke` with a valid non-nil UUID `run_id`, matching
package/handler/feature, and `context.selection`. `entry` is not required in invoke.
Text selections need a positive signed-64-bit `item_id`, `kind="text"` and string
`text`; supported non-text selection kinds are `files` and `image`. The script
preserves the exact run ID spelling, does not generate UUIDs, and never rewrites
host-owned `causality`. Host diagnostics own correlation IDs, lifecycle timestamps
and native error codes; the script adds no diagnostic protocol fields or logs.

Typical exchange:

```text
host -> invoke
script -> host_call (call_id=create-derived, method=clipboard.create_derived)
host -> host_result (same protocol_version/run_id/call_id, success=true)
script -> complete (status=success), then exit
```

The script validates protocol_version/op/run_id/call_id/boolean success and the
returned positive item ID before reporting success. Failure produces one fixed
`error` completion. Malformed input with no obtainable valid run ID emits no
protocol frame, reports a fixed stderr category and exits nonzero. Semantic errors
with a valid run ID use a single error completion and normal process exit; the
completion status, not the process exit code alone, determines the result.

There is no retry. EOF, a malformed reply, or a received matching v1 `op="cancel"`
ends processing without another host call. A cancel received as the first frame
also issues no write. A request already sent may have committed before cancellation
or connection loss: an error is **not** a rollback guarantee. Inspect history
before manually retrying when the result is unknown.

Reading stdin is blocking, not a polling/busy loop. The **NxTools host** enforces
`runtime.timeout_ms=10000` for the entire invocation, including the host-result
wait, and terminates its owned process on timeout/cancellation. The current native
broker normally cancels by terminating the supervised process; the script also
recognizes a cooperative JSONL cancel if delivered. When killed externally it may
have no opportunity to send complete. Standalone execution without NxTools has no
independent input timer: do not run `main.py` interactively and leave stdin open
without frames. The tests enforce their own finite subprocess deadlines. No
cross-platform pipe timer or background worker is introduced for this example.

## Tests

From the `clipboard-cleaner` folder, run with an installed Python:

```powershell
python -m unittest -v test_main.py
```

To avoid Python bytecode cache files while keeping the package exactly four files:

```powershell
python -B -m unittest -v test_main.py
```

Tests use unittest, subprocess and an asyncio-controlled real subprocess roundtrip.
All protocol tests use synthetic selections and a fake host, never the real
clipboard, SQLite, history directory, network or credentials. The interactive test
waits for the actual flushed host_call before synthesizing host_result, then checks
complete and exit without requiring stdin EOF. Other cases cover all eight setting
combinations, Unicode, byte limits, malformed frames, identity/type failures,
host failure, EOF, cancel and at-most-one write/completion. Test output belongs to
the unittest runner, not the extension's protocol stdout.

## Compatibility and verification scope

Checked source contracts: `docs/extensions/PROTOCOL.md`,
`docs/extensions/examples/protocol.py`,
`rust/apps/nxtools-desktop/src/extensions/manual_actions.rs`, and
`rust/apps/nxtools-desktop/src/extensions/feature_adapters.rs` in NxTools.Next.
The required manual context, host method and result shape match those sources.
No NxTools host source change is needed or included.

Local execution results are reported with the delivered folder. Passing fake-host
tests does not certify installed GUI import/trust/interpreter selection, native
broker supervision or actual derived-history persistence. Follow the manual steps
above for those checks. Python 3.10 syntax compatibility can be checked statically;
execution under each installed Python version is a separate check.

## Recorded local results

- Windows / Python 3.13.2: `python -m unittest -v test_main.py` passed all 20 tests
  in 2.588 seconds after correcting the test's stderr LF/CRLF expectation.
- Both Python files passed `ast.parse(..., feature_version=(3, 10))` syntax checks.
- Standard-library/local imports, no absolute paths, and the exact four-file
  manifest/package listing were checked.
- Not executed: Python 3.10 interpreter runtime, installed NxTools GUI import and
  trust/enable flow, native timeout/cancel supervision, real history persistence.
  All subprocess results above came from synthetic inputs and a fake host.

