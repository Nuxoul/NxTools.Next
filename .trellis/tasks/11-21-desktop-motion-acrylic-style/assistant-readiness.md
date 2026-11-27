# 悬浮助手透明宿主准备

## 参考与原生边界

旧版 `D:\Code\NxTools\Views\MainWindow.axaml.cs:2229–2274` 保留唯一悬浮窗口，隐藏后复用；禁用关闭并清理引用。`Views\FloatingAssistantWindow.axaml` 继承 Avalonia Window，透明、无装饰、不可调整大小、ShowActivated=false。构造与 `Opened/Closed` 在 `.axaml.cs:55–170` 拥有计时器、图片活动、位置、被动样式和清理；真实显示入口仍是 `Window.Show()`。Next 保留 HWND 并隐藏复用属于既定生命周期适配。

当前链路为 `FloatingAssistantApplication::sync -> send -> PlatformBridge::send_floating_assistant -> FloatingAssistantCoordinator.Dispatch`。原生 `Prepare` 创建独立输入层并调用 `Apply(false)`；`Update -> Apply(null)` 在原 `_visible` 为真时也调用 `ShowWindow(SW_SHOWNA)`；`Show -> Apply(true)` 显示渲染宿主和符合策略的输入层。`Hide/Retire` 分别隐藏和退休原会话，不等待材质准备。

## 保留约束

- 材质只面向当前 GPUI 渲染 HWND，使用 Transparent；不处理 C# 独立输入 HWND、来源主窗口或动作目标。
- 原 session/generation/connection、pending ID、五秒原生操作期限、尺寸读回和隐藏帧屏障保持原所有者。
- `Prepare/Update/Show` 在当前透明回执后沿原 ID 发送；材质等待计入同一五秒期限，不重置计时器。
- 等待期间的禁用、全屏隐藏、重试、退休和断开不能被迟到回调反转。隐藏和清理命令始终保留原通道。
- 所有跨窗口准备在下一应用轮注册，回调重新核对窗口、HWND、会话、代数、连接和当前 pending ID；不在租用实体内同步重入目标窗口。
- 不改变官方 Root/View、布局、图片解码、OLE/input 层、位置、焦点或动画所有者。

## 验证

合成回归通过原 IPC 路由验证透明请求先于原生准备，取消和超时拒绝迟到回执，已显示 Update 等待新事实而 Hide 不受阻塞。既有原生/frame 测试使用显式合成 HWND 和一致的确认回执，保持原断言。编译、生命周期执行及真实 GPUI/native 验收分别记录。
