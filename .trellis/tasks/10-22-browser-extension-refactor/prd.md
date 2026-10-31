# Browser extension refactor and frontend redesign

## Goal and user value
用户 2026-10-22 要求：按审查结论重构 NxTools 浏览器扩展，并同时优化、重新设计其前端外观。
目标是让下载接管在 Chrome/Edge/Firefox 上行为可靠（不再出现交接期间的浏览器“另存为”、
Firefox 后台不失效、失败不被报告为成功），让代码按职责可维护、可做行为测试，并交付一套新的
弹窗与页面注入外观。

## Background (verified 2026-10-22 review)
- 扩展目录 `browser_extension/nxtools-interceptor/`：MV3，`background.js` 4714 行/171 函数，
  `content-script.js` 2495 行，`page-hook.js` 1134 行，`popup.js` 1114 行；4 个策略模块已独立。
- 现有 44 个 Node 测试全部通过（`node --test tests/*.test.js`），但含 94 处源码正则断言；
  `native-lifecycle.test.js` 通过字符串标记截取 `background.js` 片段执行。
- 上一轮 UI 重设计（`10-12-browser-extension-ui-redesign`）已被用户认可（“效果可以”）；本任务
  明确授权再次重新设计外观，上一轮视觉方案只作基线而非约束。
- 旧版参考实现 `D:\Code\NxTools\browser_extension\nxtools-interceptor` 存在；交接/传输规则见
  `docs/reference-code/download.md` 浏览器扩展章节。

## Requirements

### R1 — Correctness defects (P0, each needs a red-capable test first)
- **R1.1** `background.js:4217` 的 `onDeterminingFilename` 监听器延后调用 `suggest` 却不返回
  `true`；Chrome 会立即按默认值继续，9581d83 的“交接期间不弹另存为”修复可能无效。监听器必须返回
  `true`，且每条路径恰好调用一次 `suggest`。
- **R1.2** `background.js:3` 的 `importScripts(...)` 在 Firefox 事件页中不存在；
  `BrowserExtensionPackageService.CreateFirefoxManifest`（`csharp/.../BrowserExtensionPackageService.cs:250`）
  只列 `background.js`，策略模块未加载。Firefox 包必须加载完整的后台脚本集合。
- **R1.3** `background.js:4512` `drop_target_download` 不检查 accepted，桌面拒绝时仍回复“已发送”。
  必须走与其他入口相同的 identity-bound accepted 判定。
- **R1.4** `chrome.runtime.onMessage` 路由不区分发送方；内容脚本可发 `popup_*` 消息，
  `popup_save_settings` 把任意字段原样写入 storage。路由必须按发送上下文（扩展页面 vs 标签页内容脚本）
  白名单消息类型，设置写入只接受已知键及合法值。
- **R1.5** `sendDownloadItem`（`background.js:2812`）用 `lastActiveTabId` 而非下载自身上下文查找资源；
  改为使用下载的 referrer/URL 解析标签页，无法确定时不借用其他标签页的资源。
- **R1.6** Service worker 挂起后内存中的请求头/重定向/Content-Disposition 缓存丢失：行为必须降级为
  “无上下文则按浏览器原样回退”，不得报告成功或抛出未处理异常。是否持久化由 design 决定且不得把
  Cookie/Header 写入 storage。

### R2 — Behavior test harness (prerequisite for R3)
- 提供可加载真实 `background.js`（及拆分后的模块）的模拟 Chrome/Firefox API 环境，以行为断言覆盖
  交接、浏览器下载拦截、右键、拖放、批量、Blob、连接生命周期、消息路由发送方检查。
- 用行为测试替换对应的源码正则断言；不删除任何安全/交接覆盖。

### R3 — Responsibility refactor (behavior-preserving)
- `background.js` 拆为按职责的经典脚本模块（见 design.md），入口只负责加载与注册监听。
- `content-script.js` 拆为悬浮入口、拖放区、Blob 桥、页面扫描、媒体控制模块，由 manifest 有序加载。
- `popup.js` 分离状态、渲染、操作（随 R4 外观重做一起完成）。
- 不引入 npm 依赖/打包器/运行时；每次搬移后全部测试保持绿色。

### R4 — Frontend redesign
- 重新设计弹窗（任务/捕获/媒体工具/设置）与页面注入面（媒体入口、资源选择面板、状态提示、拖放区），
  弹窗与 Shadow DOM 共用新 token。
- 保留全部现有功能、字段、设置 schema、站点开关、批量 32/部分失败重试、仅接受项取消勾选、媒体
  frame/index 消息、自动保存加显式重试、打开弹窗不启动桌面端。
- 可访问性不得低于现状：tab 角色与方向键、pressed 状态、预览键盘/Escape/焦点返回、原生进度条、
  reduced motion、可见实时反馈。
- 在 CSP 内实现：不使用内联样式/脚本，不扩大 CSP。

## Acceptance criteria
1. R1.1–R1.6 每项都有先红后绿的行为测试；Firefox 包测试证明后台脚本集合完整（C# 测试）。
2. 行为测试环境加载真实生产脚本；`node --test` 全绿，源码正则断言数量显著下降且无安全覆盖丢失。
3. `background.js` 入口 ≤ 200 行；无单个拆分模块超过约 900 行；`content-script.js` 同理拆分。
   manifest 权限、扩展 ID、key、CSP、Native 协议版本与消息类型不变（R1.4 新增发送方检查除外）。
4. 新外观在真实生产 DOM + 模拟 API 的浏览器测试中通过：四页默认/长内容/空/离线/加载/错误/禁用/
   忙碌/选择状态、键盘交互、几何边界、reduced motion；生成可交互预览与状态图库供用户目检。
   可量化外观目标：标签栏以上高度 ≤ 120px；捕获页默认视口内可见资源 ≥ 4 项（原约 2 项）；
   空闲时无常驻底部说明条；界面不再出现禁用的“合并”按钮。
5. 用户对新外观的视觉认可单独记录；截图/DOM 测试不等于安装版验收。
6. `tests/architecture/browser-native-messaging.ps1`、`spec-contracts.py` 通过；规范文档
   `.trellis/spec/browser-extension/*` 同步更新。

## Out of scope
- Rust 引擎/BrowserHost 协议变更、Native Messaging 注册表写入、真实浏览器安装扩展、导出签名包、
  打包发布、真实网页浏览/Cookie 捕获——均需用户另行授权。
- HLS/DASH 合并下载（仍按下载计划后续阶段）。
- 桌面端 GPUI 内的浏览器扩展设置页。

## Key decisions
- 2026-10-22 用户选定视觉方向 A：保留深色中性风格，精修层级、密度与状态（细节见 design.md R4）。
  仅新增一个低饱和 success 色；不做浅色主题，不与桌面端主题耦合。
- 常驻禁用的“合并”按钮从界面移除，合并消息与后台实现保留，待 HLS/DASH 阶段再接入。
- 范围包含审查方案的 P0–P4 全部内容，外加外观重设计。
