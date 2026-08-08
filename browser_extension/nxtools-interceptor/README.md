# NxTools Browser Extension

这是 NxTools.Next 下载器配套的浏览器扩展。它保留旧版的 Chrome、Edge、Firefox 单源码扩展、站点开关、右键菜单和资源面板，并使用浏览器 Native Messaging 把普通下载交给 Rust 下载引擎。

## 功能

- 自动拦截 Chrome / Edge / Firefox 的下载任务。
- Native Messaging 使用精确扩展 ID 白名单，不开放本地 HTTP/WebSocket 端口。
- 嗅探当前页面的 m3u8、mpd、音视频和常见媒体分片资源。
- 捕获最近请求的 `Referer`、`User-Agent`、`Cookie` 等 Header。
- 只有 Rust 引擎持久化接受任务后才取消浏览器原下载；交接失败时浏览器继续下载。
- 同一浏览器下载使用稳定 handoff ID，扩展或原生宿主重连不会重复创建任务。
- 右键菜单支持“使用 NxTools 解析下载/当前页面”。
- Popup 中可查看桌面任务、页面资源、连接状态，并配置拦截、解析窗口、Header/Cookie。

## 使用

1. 启动一次 NxTools.Next，使 PlatformHost 为 Chrome、Edge、Firefox 注册 `com.nxtools.next.browser`。
2. Chrome / Edge 开启开发者模式，选择“加载已解压的扩展程序”。
3. 选择打包目录中的 `browser_extension/nxtools-interceptor`。
4. Firefox 可从同一源码制作 XPI；固定扩展 ID 为 `nxtools-download-interceptor@nxtools.local`。

当前阶段只把普通 HTTP/HTTPS 下载接入新引擎。Blob、HLS/DASH 与媒体合并会按下载计划后续阶段接入；扩展不会把大 Blob 塞入 Native Messaging。
