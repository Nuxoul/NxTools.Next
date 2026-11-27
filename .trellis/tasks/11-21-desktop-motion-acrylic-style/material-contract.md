# 原生材质协议与所有者

## 管道会话与事实

原PlatformPipeClient从GetNamedPipeServerProcessId取得Desktop PID。每次握手建立新的非空material session GUID，断开即解除；调用方不能自行声明PID。EventWindow继续拥有系统偏好读取、最新状态单槽发布和既有250ms队列重试，不增加轮询管理器。

`system_material_capabilities_changed` data：`correlation_id: Guid`、`session_id: Guid`、`revision: u64`、`supported: bool`、`transparency_enabled: bool`、`high_contrast: bool`、`energy_saver: bool`、`remote_session: bool`、`native_error_code: i32`。revision在本会话事实变化时递增，首次快照不为零；连接本身不是事实就绪。

Windows读取系统透明注册表、SPI_GETHIGHCONTRAST、GetSystemPowerStatus及远程会话事实。缺失透明注册表值采用Windows缺省开启；读取错误保留原生错误并使用实底回退。支持范围为Windows10 1809及以后有DWM合成的桌面。Win10不要求DWM38支持。

## 应用与回执

`apply_window_material` data：`session_id: Guid`、`facts_revision: u64`、`window_handle: i64`、`generation: u64`、`style: solid|acrylic`、`surface: backdrop|transparent`、`dark: bool`。

原请求Envelope ID贯穿命令及诊断。C#唯一材质所有者核对当前会话、事实版本、真实Desktop peer PID、根非child HWND及本机交互会话，再执行写入。无效或陈旧请求在写入前失败；不设置位置、可见性、激活、owner或窗口样式。每个HWND代数单调增加，同一窗口的应用保持串行，断开清空旧代数。所有原生应用经原有有界后台执行机制，不阻塞串行管道reader或EventWindow。

成功的PlatformCommandResult.data为JSON字符串，字段：`session_id`、`facts_revision`、`window_handle`、`generation`、`style`、`surface`、`dark`、`effective: solid|acrylic|acrylic_opaque|transparent`、`fallback_reason: none|unsupported|transparency_disabled|high_contrast|energy_saver|remote_session|capability_read_failed|native_apply_failed`、`native_error_code: i32`。失败命令不授权展示。有效回执不等于像素或视觉验收。

普通Acrylic仅使用WCA19/Accent4/flags0/gradient0x01000000/animation0；Solid、透明特殊宿主和实底回退重置Accent0。支持DWM38的系统明确设置None1，DWM20明确写入应用深浅色。失败时尝试重置，只有已确认重置成功才返回acrylic_opaque/native_apply_failed；重置失败返回失败，保留错误。没有Accent读回接口，不编造effect getter。透明宿主不能设置普通Acrylic。

事实优先回退顺序：capability_read_failed、unsupported、high_contrast、transparency_disabled、energy_saver、remote_session。Solid身份和透明宿主不随这些事实改变，仍须原生重置成功。Acrylic回退仍保留Acrylic主题身份。

## 原连接提交顺序

`CommandRouter`在原连接内保留有界材质等待队列，沿原`PlatformCommandReceiver`单飞发送；普通命令和原取消通道继续运行。请求进入等待队列前登记原pending ID、连接代数和10秒IPC期限，不延长窗口所有者自己的期限。

原窗口所有者通过`retire_queued_window_material(id)`撤销尚未发送的旧请求，再提交最新事实；已发送请求不因本地取消而释放原生槽。真实终态沿原管道结果路由释放槽并唤醒下一请求。断开清理所有原连接ID，重连不重放。

`window_material_unavailable`及已发送请求的IPC期限到达都不能证明原生工作线程已结束；该连接停止材质出站，明确失败等待者，普通命令不受影响。迟到回执不能恢复出站或授权旧窗口，重新连接后重新取得事实。未发送请求单独超时只撤销自身，不污染仍正常的材质槽。

Desktop共享`Preparation`只保留请求/有效回执与当前事实、风格、深浅色的匹配，不拥有窗口、显示、焦点或计时器。提交失败不保留假pending；原生`stale_material_facts`等待更新快照，不原地重发。透明宿主策略随请求身份核对。

## 展示与渲染边界

Rust沿原prepare/ACK/frame/reveal链路等待当前会话、事实版本及窗口代数对应回执，关闭/取消/重连使旧回执失效。材质所有者不成为第二套位置或可见性管理器。普通首次窗口隐藏准备，复用保持原位置及实体；被动窗口保持NOACTIVATE，主搜索保持可激活。Root与角色投影在原延迟应用轮进行，不同步回写租用实体。

诊断为accepted/start/terminal及事实状态变化，包含请求GUID、会话/版本、HWND、代数、风格/有效模式/回退原因、耗时及原生错误，不包含窗口标题、输入、查询、剪贴板或路径，不逐帧记录。
