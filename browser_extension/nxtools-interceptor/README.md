# NxTools Browser Extension

这是 NxTools 下载器原配浏览器扩展的 Next 版本，用于拦截浏览器下载请求、嗅探页面媒体资源，并通过受浏览器身份约束的 Native Messaging 交给 NxTools。

## 功能

- 自动拦截 Chrome / Edge / Firefox 的下载任务。
- 可选择立即下载，或接管后加入 NxTools 队列等待手动开始。
- 通过 `com.nxtools.next.browser` Native Messaging 主机通信，不再开放无鉴权本地 WebSocket。
- 嗅探当前页面的 m3u8、mpd、音视频和常见媒体分片资源。
- 捕获最近请求的 `Referer`、`User-Agent`、`Cookie` 等 Header。
- 只有 NxTools 返回持久化接受后才取消浏览器原下载；取消失败会撤销 NxTools 任务。
- Manifest V3 只用非阻塞 `webRequest` 采集 Header；普通安装不需要企业策略的
  `webRequestBlocking` 权限。
- 右键菜单支持“使用 NxTools 解析下载/当前页面”。
- Popup 中可查看桌面任务、页面资源、连接状态，并配置拦截、解析窗口、Header/Cookie。

## 使用

1. 安装并至少启动一次 NxTools.Next，让 PlatformHost 注册 Native Messaging 主机。
2. Chrome / Edge 可打开扩展管理页面并加载本目录。
3. 开启开发者模式。
4. 选择“加载已解压的扩展程序”。
5. 选择本目录：`browser_extension/nxtools-interceptor`。
6. Firefox 可在 NxTools 设置里导出 `.xpi`，再到 `about:debugging#/runtime/this-firefox` 临时加载。

扩展身份沿用原签名密钥：Chrome / Edge ID 为 `ajcjlgdnfcagknjcajhbiikagimgdnaf`，Firefox ID 为 `nxtools-download-interceptor@nxtools.local`。
