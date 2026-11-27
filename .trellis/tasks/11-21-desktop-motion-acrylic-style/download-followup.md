# 下载进度投影收尾

## 行为边界

连续TaskChanged事件刷新紧凑任务快照时，不得因快照序号领先当前消费序号而略过后续排队的TaskProgress，也不得清除其他仍有效任务的已接收进度。主动权威刷新、会话变化、事件缺口和SnapshotRequired仍清空失效进度并重建水位；旧序号仍拒绝。

所有者为Desktop现有DownloadApplication事件/快照投影，不修改引擎、持久化格式、传输、IPC或窗口展示状态机。原窗口、视图、输入、取消及终态身份保持，快照/事件仍使用同一会话代数和序号。

## 参考与调用链

- ABDM `shared/app/.../singledownloadpage/BaseSingleDownloadComponent.kt:107–134`：同一downloadId的monitor flow更新原状态，完成才遵循原关闭/完成窗口政策；无普通状态通知重置进度所有者。
- 旧NxTools `ViewModels/DownloadTaskViewModel.cs:599–625`：UI线程发布原任务状态和派生进度属性；状态通知不替换任务实体。原生show/hide与窗口构造链不在本次修改范围。
- Next `downloads.rs::handle_engine_event -> refresh_snapshot -> apply_task_window_directive`，`task_windows_impl.rs::update_task_window -> DownloadTaskWindowView::replace_snapshot`。当前紧凑快照没有实时分段进度，普通事件不能把该快照水位当成整个事件流已消费的证明。

## 回归

独立事件投影测试通过真实应用事件入口、引擎快照加载和当前任务集合验证排队进度与缓存保留；既有主动快照清空、旧序号拒绝、会话/缺口重同步回归保留。原真实HTTP重平衡、同一窗口/输入、真实字节区间和最终输出测试作为原场景验证，不放宽8秒期限。

事件投影3项、原真实HTTP重平衡1项及完整下载组363项通过。连续事件仅推进已消费水位；缓存只保留当前已知非终态、字节/总长兼容的进度。权威刷新、旧序号和重试重置回归保持通过。原HTTP场景继续要求同一窗口/输入、真实分段区间、最终输出及原8秒期限。
