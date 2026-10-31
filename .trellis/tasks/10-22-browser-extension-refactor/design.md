# Design — browser extension refactor and redesign

## Invariants carried over (must not change)
- 交接顺序：identity-matching accepted → 取消浏览器下载成功 → erase；任何失败保留浏览器下载
  （`.trellis/spec/browser-extension/contracts.md`）。
- Native 传输按需连接、空闲 1s 释放、旧端口回调不能修改新端口、诊断不启动宿主；弹窗快照不复活桌面端。
- 256 KiB 请求上限、协议版本 2、`requestId` 与日志关联 ID 分离、批量 32、逐项部分失败。
- 弹窗投影不含 Header/Cookie/Referrer；日志不含 URL query/路径/标题/Header/Cookie/Blob 字节。
- manifest 权限、host 权限、`key`、gecko ID、CSP 不扩大。

## R1 defect designs
- **R1.1 suggest**：监听器 `(item, suggest) => { void interceptBrowserDownload(item, { suggest: once(suggest) }); return true; }`。
  `once` 包装保证单次调用；所有终止路径（未启用、站点停用、非 HTTP、失败、成功）都调用一次。
  `onCreated` 回退分支不变。
- **R1.2 Firefox background**：源 manifest 同时声明 `background.service_worker: "background.js"` 与
  `background.scripts: [<完整有序列表>]`（Chrome 121+ 忽略 scripts，Firefox 忽略 service_worker）。
  `background.js` 入口在 `typeof importScripts === "function"` 时才调用 `importScripts(<同一列表>)`。
  `CreateFirefoxManifest` 改为保留源 manifest 的 `scripts` 列表并删除 `service_worker`，不再硬编码
  `["background.js"]`。一致性测试：manifest `scripts` 列表 == `importScripts` 列表 == 磁盘文件。
  需确认最低 Chrome 版本对双键的容忍（实现前查官方文档并记录在 research/）。
- **R1.3 drop target**：`drop_target_download` 复用统一的 `handoffUrl()`，只有
  `acceptedHandoffResponse` 为真才返回 ok。
- **R1.4 sender trust**：路由表 `{ type, origin: "extension_page" | "content", handler }`。
  `extension_page` 判定：`sender.id === chrome.runtime.id && !sender.tab && sender.url` 以扩展 origin 开头。
  `content` 判定：`sender.id === chrome.runtime.id && sender.tab?.id > 0`。不匹配返回 `{ok:false}` 并记录
  固定事件名的诊断。`popup_save_settings` 经 `sanitizeSettings()`：仅 `DEFAULT_SETTINGS` 键、类型匹配、
  枚举/数值范围校验（drop target 96..360 × 48..220）。
- **R1.5 download context**：`resolveTabIdForDownload(item)` 依次用 `item.tabId`（若浏览器提供）、
  referrer 匹配的标签页；无法确定返回 0，不使用 `lastActiveTabId`。
- **R1.6 worker suspension**：不持久化 Header/Cookie。仅把 Content-Disposition 文件名与重定向映射
  （不含 query 的规范化键）写入 `chrome.storage.session`（若可用），带 TTL 与上限；缺失上下文时照常交接
  （桌面端自行请求），失败按浏览器回退。

## R2 test harness
- `tests/harness/chrome-api.cjs`：可编程的 `chrome.*` 假实现（runtime/connectNative 端口、downloads、
  tabs、storage.local/session、webRequest、contextMenus、scripting、declarativeNetRequest），事件可触发、
  调用可记录。
- `tests/harness/load-background.cjs`：在 `vm` 上下文中按 manifest `background.scripts` 顺序加载真实文件，
  暴露受控时钟。不再用字符串标记截取源码。
- 行为测试按场景分文件：`handoff.behavior.test.js`、`download-intercept.behavior.test.js`、
  `router.behavior.test.js`、`native-transport.behavior.test.js`、`firefox-manifest.test.js`。
- 源码正则断言仅保留真正的结构约束（如“禁止内联样式”、“manifest 权限集合”）。

## R3 module layout (classic scripts, shared worker global scope)
```text
bg/constants.js         常量与默认设置
bg/naming.js            URL 规范化、文件名、MIME/扩展名推断
bg/settings.js          设置读写、sanitizeSettings、站点开关
bg/native-transport.js  连接、请求、待决表、空闲释放、诊断日志
bg/request-context.js   Header/重定向/Content-Disposition 缓存（R1.6）
bg/resource-cache.js    资源索引、容量、TTL、持久化投影
bg/site-adapters.js     YouTube/Bilibili/Douyin/DASH 规则
bg/handoff.js           单个/批量/Blob/浏览器下载/URL 交接
bg/tab-bridge.js        注入、frame 枚举、发消息、扫描
bg/media.js             媒体状态、媒体操作、截图、移动 UA
bg/popup-state.js       弹窗投影
bg/router.js            消息路由表与发送方检查
background.js           入口：importScripts + 注册监听
```
每个模块用 `const NxBg<Name> = (() => { ... return {...}; })();` 暴露显式接口，Node 测试也可单独加载。
`content-script.js` 同法拆为 `content/overlay.js`、`content/drop-target.js`、`content/blob-bridge.js`、
`content/scanner.js`、`content/media-control.js` 与入口，manifest `content_scripts.js` 有序列出，
`injectContentBridge` 的 `scripting.executeScript` 文件列表与 manifest 一致（测试保证）。
搬移顺序：纯函数模块 → 缓存 → 传输 → 交接 → 路由 → 入口，每步测试全绿后再进行下一步。

## R4 frontend redesign
- 结构层面（已确定）：`popup.js` 拆为 `popup/state.js`（消息与状态）、`popup/render-*.js`（四页渲染，
  只用 `textContent`/元素构建，移除 `innerHTML` 字符串拼接）、`popup/actions.js`（忙碌/重试/反馈）。
  token 文件继续由弹窗与 `shadow-styles.js` 共用。
- 视觉方向（用户 2026-10-22 选定 A）：**保留深色中性风格，精修层级、密度与状态**。
  - 色板：沿用 `popup-tokens.css` 现有 token 与值；唯一新增 `--nt-success`（低饱和绿，仅用于“已连接/已接受”
    状态点与成功反馈），弹窗与 `shadow-styles.js` 同步。不引入渐变、彩色强调色或浅色主题。
  - 字体与尺寸：沿用 Segoe UI Variable/YaHei UI，正文 13/20、辅助 12/18、标题 16/22；圆角 6/浮层 8。
    弹窗宽 456px、内容区 420..445px 不变。
  - 依据 2026-10-12 图库实际截图（`artifacts/ui-extension-redesign/capture.png` 等）确定的精修项：
    1. 顶部压缩：品牌行与连接状态合为一行，去掉“浏览器下载工作台”副标题；页面上下文行保留但降高。
       目标：标签栏以上的高度从约 165px 降到 ≤ 120px。
    2. 捕获页工具区从 4 层收为 2 层：第 1 层“筛选 + 计数 + 清空”，第 2 层批量操作条只在有勾选时出现
       （吸顶）。“当前页面 / 勾选加入队列·预览不下载”说明文字移除，改为空状态里的一次性说明。
    3. 资源行改为两行紧凑行：文件名 + 一行元数据（类型·分辨率·大小）；“预览/发送下载”放在行右侧同一行，
       始终可见（不依赖悬停，保证键盘可达）；MIME 与来源（webRequest 等）移到展开预览里。目标：默认视口内
       可见资源从约 2 项提升到 ≥ 4 项。
    4. 缩略图占位从“视频预览”文字改为按类型的线性 SVG 图标（视频/音频/图片/流媒体/文件）。
    5. 常驻禁用的“合并·后续”按钮从工具栏移除；`popup_merge_resources` 消息与后台实现保留不动。
    6. 底部常驻说明“本地交接 就绪·打开面板不会…”改为仅在有操作反馈时出现的状态条（成功/失败/进行中），
       空闲时不占位；“打开面板不启动桌面端”的说明移到设置页“本地连接”分组。
    7. 任务页：进度条与操作按钮合到同一行（进度条左、按钮右），去掉独立按钮行；状态文字使用语义色
       （下载中中性、暂停次要、失败 danger、完成 success）。
    8. 设置页：保留三组分组与自动保存+显式重试；开关行降高到 48px，说明文字统一 12/18。
    9. 注入面：媒体入口 28px 与资源选择面板沿用新的行密度与图标；拖放区与状态提示仅同步 token。
  - 动效：只保留 120ms 的 hover/pressed 与展开过渡，全部受 `prefers-reduced-motion` 约束；无空闲动画。
  - 实现细化（2026-10-22 Phase F 开工前）：
    - 新 token：`--nt-success: #7fbf9a`（在 #18181b 上对比度 ≥ 4.5:1，`--nt-success-soft: #22302a`）。
    - 状态条（`#messageText`）保留 `role=status`/`aria-live`，改为 `.status-footer[hidden]` 空闲隐藏，
      语气由 `data-tone=status|success|error` 驱动；成功提示 4s 后自动收起，错误保持到下次操作。
    - 合并按钮：从 DOM 移除 `#mergeResourcesButton`，弹窗不再发送 `popup_merge_resources`；后台消息与
      实现不动（router/content 白名单不变）。
    - 资源行网格：`[勾选 16][图标 28][文件名 + 元数据][预览][发送]`，行高 ≈ 52px；扩展名/类型图标为
      内联 SVG 由 DOM API 构建（CSP 安全，无外部资源）。
    - 弹窗渲染全部改为 `document.createElement` + `textContent`，`escapeHtml`/`innerHTML` 从 popup 移除。
    - `popup.js` 按职责拆为 `popup/format.js`（纯格式化）、`popup/dom.js`（元素引用与构建工具）、
      `popup/render.js`（四页渲染）、`popup/actions.js`（消息、忙碌、反馈、事件绑定），入口 `popup.js`
      只做初始化；均为经典脚本按 popup.html 顺序加载（弹窗页面只加载一次，无重复注入问题）。

## Compatibility and rollback
- 每个阶段独立提交点；R3 纯搬移可整段回退。manifest 仅新增 `background.scripts` 与内容脚本文件列表。
- 已安装扩展升级后需在浏览器中“重新加载”；扩展 ID 不变，存储 schema 不变（仅新增 session 存储键）。
