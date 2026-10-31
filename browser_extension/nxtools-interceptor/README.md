# NxTools Browser Extension

这是 NxTools.Next 下载器配套的浏览器扩展。它保留旧版的 Chrome、Edge、Firefox 单源码扩展、站点开关、右键菜单和资源面板，并使用浏览器 Native Messaging 把普通下载交给 Rust 下载引擎。

## 功能

- 自动拦截 Chrome / Edge / Firefox 的下载任务。
- Native Messaging 使用精确扩展 ID 白名单，不开放本地 HTTP/WebSocket 端口。
- 嗅探当前页面的 m3u8、mpd、音视频和常见媒体分片资源。
- 捕获最近请求的 `Referer`、`User-Agent`、`Cookie` 等 Header。
- 只有 Rust 引擎持久化接受任务后才取消浏览器原下载；交接失败时浏览器继续下载。
- 同一浏览器下载使用稳定 handoff ID，扩展或原生宿主重连不会重复创建任务。
- 右键菜单支持“使用 NxTools 下载/下载当前页面”，只有原生端明确接受后才显示成功。
- 资源面板可将 1–32 个普通 HTTP/HTTPS 资源一次加入队列；每项使用稳定 handoff ID，部分失败可原地重试。
- Popup 中可查看桌面任务、页面资源、连接状态，并配置拦截、解析窗口、Header/Cookie。
- 捕获器优先使用网络、DOM 变更和 Performance 事件；完整背景图扫描只在打开捕获页或手动深扫时执行，避免持续遍历页面。
- 响应阶段的文件名、MIME、大小和请求上下文不会被后到的弱页面信息覆盖；Popup 只接收展示投影，不携带 Header/Cookie/Referrer。
- Popup 采用统一中性下载工作台：固定当前页面上下文、任务/捕获/媒体工具/设置四页、紧凑资源行（类型图标 + 文件名 + 类型·分辨率·大小）、勾选后才出现的批量条、按需出现的状态条。打开面板不会自动启动桌面端。
- 扩展消息按来源区分：网页内容脚本不能修改设置或操作桌面任务，其他扩展的消息直接忽略。

## 使用

1. 启动一次 NxTools.Next，使 PlatformHost 为 Chrome、Edge、Firefox 注册 `com.nxtools.next.browser`。
2. Chrome 优先使用下载设置 → 常规 → 浏览器扩展中的“Chrome 商店”按钮安装正式扩展；不需要填写扩展 ID。需要 Chrome / Edge 121 或更高版本。
3. 本地安装仍可在 Chrome / Edge 开启开发者模式，加载打包目录中的 `browser_extension/nxtools-interceptor`，或使用 NxTools 导出的签名 .crx 安装包；官方源码目录和本机导出包的身份会自动加入精确白名单。
4. Firefox 可从同一源码制作 XPI（或使用应用内“导出 Firefox (.xpi)”按钮）；固定扩展 ID 为 `nxtools-download-interceptor@nxtools.local`。
5. 加载后若提示连接失败，先确认 NxTools 正在运行；升级安装后请对扩展点“重新加载”。

### Chrome 网上应用店

- 正式安装地址：https://chromewebstore.google.com/detail/nxtools-download-intercep/eldfplghejhopcmgfkacimanhaojakph
- 商店 ID 已内置，普通用户无需填写；Native Messaging 同时保留官方本地目录和本机签名导出包的 ID。旧版本已保存的自定义 ID 继续兼容，但不再显示输入框。
- 发布者仍可在下载设置 → 常规 → 浏览器扩展中选择“导出 Chrome 商店 (.zip)”。该 ZIP 的根目录清单不含 `key`，不会改动本地加载用源码或读写签名私钥；不要把 CRX 当作商店上传包。
- 启动桌面端并完成原生注册后，重新打开扩展测试连接；原生注册失败仍会给出重试提示，不能把“设置已保存”当作白名单已经应用。

普通 HTTP/HTTPS、批量队列和 Blob 已接入新引擎。Blob 字节使用一次性有界 loopback 数据面，不会塞入 Native Messaging；HLS/DASH 与媒体合并仍按下载计划后续阶段接入。

## 代码结构

- `background.js`：后台入口，只注册监听；依赖按顺序列在入口 `BACKGROUND_DEPENDENCIES` 与 manifest `background.scripts`（两份列表必须一致，Firefox 无 `importScripts`）。
- `bg/`：后台模块（常量、状态、命名、请求上下文、设置、Native 传输、站点规则、资源缓存、媒体、标签页桥、捕获、交接、资源选择、弹窗状态、右键菜单、消息路由）。
- `content/`：内容脚本的无状态模块，挂在 `NxToolsContent` 命名空间，重复注入不会重复执行；`content-script.js` 持有全部页面状态。
- `popup/`：弹窗的格式化、DOM、渲染、操作模块，`popup.js` 只负责初始化。

## 开发验证

从仓库根运行 `node --test`（`tests` 下递归全部 `*.test.js`），以及 `tests/content-browser.cjs`、`tests/refine-browser.cjs`、`tests/layout-browser.cjs`、`tests/redesign-browser.cjs`。行为测试通过 `tests/harness/` 加载真实后台脚本（模拟扩展 API、模拟桌面端、虚拟时钟，覆盖 Chrome 与 Firefox 加载方式）；浏览器测试使用临时配置、合成 API 与网络阻断，不安装扩展或连接真实桌面。

运行 `node browser_extension/nxtools-interceptor/tests/build-preview.cjs` 可生成 `artifacts/ui-extension-redesign/` 的状态图库和可交互预览。预览运行生产页面/脚本，但使用测试 API；不等同于 Chrome/Edge/Firefox 安装版验收。测试夹具不由 manifest 或生产 popup 加载。
