# NxTools.Next development rules

## Packaging handoff

Before any package/build-release request, read [PACKAGING.md](PACKAGING.md). It records the
existing workstation PEM/ledger locations and the reusable `scripts/package-local.ps1` entry.
Use `-CheckOnly` for read-only discovery; never invent a key, reset a ledger or depend on prior
AI-session scratch for required publishing material. Private PEM bytes stay outside Git/artifacts.

Engineering and UI guidance starts at [.trellis/spec/index.md](.trellis/spec/index.md).
Read its applicable owner topics and shared contracts before code changes. The approved UI
target/current/exception matrix is [.trellis/spec/shared/ui-design.md](.trellis/spec/shared/ui-design.md);
historical component/reference measurements do not silently override the later approved brief.
For spec-only changes run `python tests/architecture/spec-contracts.py`; this checks documents,
not product behavior or native/release acceptance.

For EVERY Desktop UI task, first read the short mandatory
[component-first selection rules](.trellis/spec/shared/ui-component-selection.md), then the
[current official component table](docs/reference-code/gpui-kit-component-table.md) and the
[locked API index](docs/reference-code/gpui-kit-component-index.md). Prefer existing official
styled compound components and their public slots before custom ordinary controls. Record
the requested layout shape, exact released API, visible variant/size and parent adaptations.
`Toolbar.child(Button)` in locked0.8.0 still forces `ghost().compact()`; sizing alone does not preserve
outlined actions. Import/environment/chain/activity header commands need recognizable official
button boundaries. Extensions catalog presentation must follow the user-requested Other Tools
cell/tile form, not the rejected long prose list; do not invent a Card/Grid component or confuse
catalog cells with DataTable cells. Custom layout may arrange official components, not repaint
their substitutes. Missing review is not unsupported capability. Imports, passing tests and
AI-inspected screenshots do not certify visual/user acceptance. Run the local documentation
gate `python tests/architecture/gpui-component-catalog.py` when updating the catalog/rules.
The user-approved current graph is Kit/Base/Component/Assets0.8.0 with default gpui-pre0.3.9.
Read [the current release/API delta](docs/reference-code/gpui-kit-080.md) before the historical
0.7.1 catalog. Only notification-duration and Windows no-activate framework patches are carried;
GPUI Fast remains disabled. The separately approved2026-10-22 plan adopts the pinned Markdown
editor with only passive element.rs and three cull.rs lowbit substitutions; see
[editor provenance](docs/reference-code/markdown-editor.md). Run both
`python tests/architecture/gpui-release-contracts.py` and
`python tests/architecture/markdown-editor-contracts.py`.

## Reference-driven migration

Reference review is a blocking prerequisite, not an optional follow-up. For any feature that
already exists in NxTools, do not design, patch, or "improve" the new implementation from memory
or from the current NxTools.Next code first. The old implementation is the behavioral source of
truth unless the user explicitly requests a changed behavior.

When a feature has a checked-in phased plan, its phase names and order are canonical. Before
reporting or starting a “next phase,” reread that plan and use its exact phase title. Stabilization
or catch-up work remains part of the current phase; do not renumber it or pull later-phase work
forward unless the user explicitly approves the reorder and the plan is updated.

Before implementing or changing a migrated interaction:

1. Read the matching note in `docs/reference-code/` first.
2. Follow its recorded files in `D:\Code\NxTools` and the supplied reference projects.
3. Search source only to fill an index gap or verify changed code; update the note with every newly
   verified entry file, call chain and invariant before implementing.
4. Write down the layout, lifecycle, cancellation, placement, focus, and failure-state invariants that must survive the migration.
   Trace the implementation through constructors and base classes to the real native show/hide
   boundary; leaf view/controller files alone are not sufficient lifecycle evidence.
5. Add a red-capable regression test at the real boundary before changing code.
6. Preserve the architecture boundary: Rust owns state and layout, C# owns Windows APIs, and GPUI owns rendering.
7. Do not declare the slice complete from compilation or screenshots alone; verify its interaction lifecycle.

For every GPUI surface, an entity callback must not synchronously update an application/entity that
can write back into the currently leased entity, including cache-hit paths. Update local view state
in the current callback, schedule cross-entity work for the next application turn, and protect the
boundary with a lifecycle regression gate.

For clipboard hover preview specifically, keep one passive preview window, update/reposition it in place, hide it when inactive, and never synchronously re-enter an already leased GPUI entity.

For download behavior and interaction, inspect AB Download Manager first, then the old NxTools
implementation for product-specific compatibility. Do not combine two competing interaction
frameworks when the reference uses one native gesture boundary; Windows Shell drag-and-drop is one
OLE loop owned by C#, initiated directly from the still-pressed GPUI pointer callback. Because C#
runs in PlatformHost instead of the visible GPUI process, the command must carry the source HWND;
C# must cancel that window's existing pointer capture and verify the initiating button is still
pressed before entering Ole32 `DoDragDrop` on a dedicated STA worker. A native drag may outlive a
single pointer callback and must never block the serial platform-command reader or the shared
Windows event thread; Shell open/reveal and hotkey/clipboard processing remain independently
responsive.

For Quick Launch preview, keyboard and pointer selection must feed the same generation-scoped preview
controller. Create the auxiliary preview HWND hidden, apply `WS_EX_NOACTIVATE` through the C# Windows
adapter, atomically apply its final device-pixel bounds, then show it; a hidden GPUI window may still
have `CW_USEDEFAULT` native bounds. After first creation, restore both the primary Windows HWND and
GPUI search input focus; do not make the primary search panel non-activating.

## Diagnostic contract

Every new user-facing feature and every cross-process operation must define its diagnostic events
before the slice is considered complete. Carry one correlation ID across Rust, IPC and C#; record
accepted/start, meaningful state transitions, terminal success or failure, elapsed time and native
error codes at the boundary that owns them. Never log clipboard contents, credentials, request
headers, cookies or other payload secrets.

Diagnostics must remain cheaper than the feature. Do not log animation frames, pointer movement,
download byte ticks or other high-frequency updates. Coalesce those streams into state changes,
bounded milestones or rate-limited repeated failures. Logging uses bounded/non-blocking sinks and
must never change the result of the operation it observes. Temporary probes use a unique
`DEBUG-...` marker and must be removed after the real interaction turns green; the low-noise
production lifecycle events remain.

When formatting PowerShell diagnostic output, assign the output of a statement-level `foreach`
loop to a variable before piping it to `Format-Table` or another cmdlet; direct `foreach { ... } |
...` composition is rejected by the PowerShell parser.

Do not use PowerShell automatic or preference variable names for script-local state. In particular,
avoid `$Host`, `$PID`, `$Error`, `$HOME` and case-only variants; use task-specific descriptive names.

On this workstation, every Rust verification and Release-package wrapper (including a background
PowerShell process) must use `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, and
`RUSTFLAGS=-C codegen-units=256`. The checked-in Windows wrappers initialize these values themselves;
direct Cargo commands must still inherit them. Release packaging must pass the committed-memory
headroom preflight before starting Cargo. Setting the variables only for an earlier command is not
enough; default package concurrency can exhaust Windows commit memory and make unrelated `rustc`
targets exit with `0xc0000409`. When the user declines packaging or building because of current
memory pressure, do not start Cargo verification either unless the user separately authorizes it;
use source/static gates and report the deferred compiled verification explicitly.

Give long Cargo verification and packaging commands a timeout sized for the full operation. Use the
tool's resumable yield/poll mechanism for progress instead of a short process timeout; after any
wrapper timeout, inspect surviving Cargo/rustc processes and never start a duplicate build.

## Indexer release gate

`Ready` and successful search are not sufficient evidence for an indexer release. After changing
MFT, USN, service-session or cache code, run `tests/performance/indexer-idle-cpu.ps1` against the
packaged LocalSystem service with `-RestartService -AssertRegression`, then run the real service
search smoke test. A service restart must restore a versioned memory-mapped base plus its persisted
delta; it must not rebuild all volumes merely because the desktop reconnects. State explicitly
which reference source was inspected, and never imply that Listary's native engine source was
available when only its managed boundary could be observed.

A bounded `RebuildRequired` decision while a last-good snapshot remains searchable is maintenance,
not a user-facing failure. Keep its reason in the maintenance event/log and reserve `last_error`
for actual poll, compaction, persistence, or snapshot-worker failures.

Before overwriting the default Release package, stop only a running desktop whose executable path
exactly matches `artifacts\win-x64\NxTools.exe`, and let the package script stop/restart only the
installed indexer service whose registered binary matches that same package. Never terminate an
unrelated process merely because its process name is similar.

The protected desktop, PlatformHost, and indexer form one atomic release. Startup may redirect to
the protected copy only when all three protected binaries match their packaged counterparts, and a
service smoke test must reject a protected indexer whose approved/runtime hash differs from the
current package.
<!-- TRELLIS:START -->
# Trellis Instructions

These instructions are for AI assistants working in this project.

This project is managed by Trellis. The working knowledge you need lives under `.trellis/`:

- `.trellis/workflow.md` — development phases, when to create tasks, skill routing
- `.trellis/spec/` — package- and layer-scoped coding guidelines (read before writing code in a given layer)
- `.trellis/workspace/` — per-developer journals and session traces
- `.trellis/tasks/` — active and archived tasks (PRDs, research, jsonl context)

If a Trellis command is available on your platform (e.g. `/trellis:finish-work`, `/trellis:continue`), prefer it over manual steps. Not every platform exposes every command.

If you're using Codex or another agent-capable tool, additional project-scoped helpers may live in:
- `.agents/skills/` — reusable Trellis skills
- `.codex/agents/` — optional custom subagents

Managed by Trellis. Edits outside this block are preserved; edits inside may be overwritten by a future `trellis update`.

<!-- TRELLIS:END -->

=== 范围约束(约束你提议什么修法,不约束你找什么)===
凡是这里真的有问题,都要报——包括听起来罕见但本项目确实会产生的情况。
然后把修法收在范围内:
1. 这不是一篇安全攻防论文。可以校验,禁止过度防御。除非本项目另有说明,默认操作者是
   自己机器上的合作者;如果它真有对手,它会写明,以那个范围为准。
2. 不要加哈希/校验和/指纹,除非它替代了一个实质上更贵的操作,并且结果会改变下一步做什么。
3. 禁止防御性脚手架:不为这里不会发生的情况加 feature flag、迁移框架、兼容层、包装层。
4. 禁止钻牛角尖:冷门编码、符号链接竞态、RTL 文本、毫秒级竞态一律不在范围内,
   除非该情况经由本项目**受支持的用法**可达——它的文档示例、它公开的接口、它真实的
   数据。可达即可,不需要你复现出来;但"理论上构造得出"不算。
5. 该判断的地方就判断,不要换成评分表、检查清单,或对已经定论的东西再跑一遍校验。
6. 以上都不覆盖用户、本项目自己的约定、或更高优先级规则明确要求的安全、迁移、校验与
   审阅。那些是被要求的,是活儿本身,不算范围外。
7. 交付物的文字不是答辩记录。成立的就直说;注意事项集中到专门一节
   (Limitations、已知问题),别在每段末尾都缀一句免责;更不要把指令**或过程**
   写进成品:「别提 X」意味着 X 不存在(不是写「我们不讨论 X」);中间报错、
   被放弃的方案、改稿痕迹同样不是内容。围绕最强的结果组织——论文是发布会,
   不是工作汇报。不占优的数字,是取舍就说成取舍,不是就平实陈述,都不写成认输;
   数字本身照样留在表里。
8. 推进本身是交付的一部分。几个方案都合理时,选一个并写明取舍——只有分歧
   真的大、或选错难回头时才升级为提问。不挡住目标的细节,记下来接着走,
   不是先停下解决它。为了不选错而停在原地,本身就是选错。
9. 不留任何生成/纠偏痕迹:最终产出(代码、注释、docstring、commit message、
   PR 描述、总结)只呈现正确的最终结果,当作一开始就是这么写的——不提中间报错、
   试错纠偏、被放弃的方案、"为什么没做 X";也不出现 AI 辅助的标记,比如
   "Generated by / Co-authored-by: Claude/Codex" 署名、模型腔过渡语
   ("值得注意的是"、"综上所述"、"首先...其次...最后")、"以下是..."式开场白,
   措辞详略贴合项目和文档已有风格。
已经见过的形状,供你校准。是例子不是清单——一个真问题不会因为"长得像其中一条"就被驳回:
  H  为了比对两个表格的差异,给每一行都算哈希——直接比单元格就能回答
  H  写下一堆校验和文件,而没有任何代码会去读它们
  E  给一个没有用户、没有部署的应用做账号安全加固
  R  用一整夜对自己的补丁反复审计,而功能一行没写
  R  一个对任何提交都给不通过的审阅者
  O  一层守卫的理由是上一层守卫,而不是需求
另有两种长得像上面、但不是的。这些要报:
  ✓  用摘要比对来跳过重读一个你已经有的大文件
  ✓  本项目自己的文档示例就会产生的那种"听起来罕见"的输入
跑任何检查之前先回答:这次运行会检测出什么具体的失败?真出现了我下一步会做什么不同的事?
答不上来就别跑。
对的就说对。不要为了交差硬找问题。
