# 目标外壳原生圆角

## 所有权

Quick Launch/Clipboard主外壳请求8DIP，继续通过原SetWindowBounds发送设备像素。C#原adapter负责SetWindowRgn；GPUI绘制、实体、原bounds/topmost/material/frame/reveal/activation顺序和保存值不变。不修改THICKFRAME、Caption、WindowEdge、整窗opacity或DWM材质职责。

`WindowsPlatformAdapter.Corners.cs`保存明确请求过圆角的HWND半径及初始DPI，按当前实际外框尺寸/DPI重建区域。仅位置改变时不重建，最小化保留上次区域，最大化解除区域，还原后恢复；0清除策略，null保留。Windows拥有成功传入的HRGN，失败传入仍由调用者释放。

原EventWindow LOCATIONCHANGE/MOVESIZEEND/DESTROY回调在文件对话框过滤前传递根窗口事件；子对象、子ID、其他事件和零句柄不转发。回调只退休记录或提交原有界CenteredResizeWorkers。队列中每个HWND仍只有一个在途和一个合并后续工作；统一工作同时检查原居中resize与圆角，不以另一种Action覆盖未执行的居中resize。圆角更新不调用SetWindowPos、不阻塞事件线程、不轮询。断开/新peer及Dispose清空策略。

## 普通窗口材质圆角

普通窗口的圆角轮廓由 GPUI 绘制，沿用已认可的 UnderBorder（8 DIP、1 DIP 实线，背景与边框同一元素、一次补偿覆盖）；不使用系统圆角（DWMWCP_ROUND）。Acrylic 下 Root 表面向外扩 1 DIP 覆盖官方外框保留的环，自带边框色并隐藏官方外框边框（该边框是单独元素，会让弧线重复覆盖）。

C# `ApplyMaterialCorners` 在 Backdrop 材质成功后以 8DIP×DPI 设置 HRGN，只用于裁掉原生模糊在四个方角处的露出；区域标记为材质所有，Solid 时释放，显式 `SetWindowBounds` 策略（Quick Launch、Clipboard）不被覆盖。区域沿原有界几何工作随尺寸维护，最大化解除。失败只记录 `window.corners.failed`，不撤销已确认的材质。

验证：隐藏窗口真实 GPU 读回，普通 Acrylic 窗口与单元素 UnderBorder 参考在四角 24px 范围颜色最大差 0/255（旧结构 85/255），四条边中点像素与参考完全一致；覆盖差 24/255 仅为官方外框的轻阴影。夹具 `ordinary_acrylic_window_contour_matches_the_single_element_under_border_reference`（`--ignored`）。GDI 区域本身无抗锯齿，其边缘锯齿是否可接受仍待你在真实窗口确认。
## 诊断

原bounds请求correlation贯穿策略。原命令继续记录接受/终态；尺寸更新只报告failure/recovered状态变化，包含HWND、native_error_code及elapsed_ms，不记录位置/帧tick、标题或载荷。

## 已验证

- C#测试项目构建0警告/0错误。
- `--fixtures "window bounds corners"`：2项通过。严格wire与DPI算术；自有STATIC HWND的真实GetWindowRgn/GetRgnBox/PtInRegion；原bounds及用户式原生resize放大/缩小、负坐标移动、零重置和退休；原事件过滤→有界工作队列→实际区域更新；前台与隐藏状态保持。
- 自有隐藏framed HWND的WS_MAXIMIZE状态位使真实IsZoomed读回为真，验证区域解除/恢复；状态位随后恢复。此项是原生状态分支证据，不是物理最大化操作。
- 原材质13项fixture通过，保留会话、代数、回退、失败、退休、真实隐藏HWND及dispatcher边界。
- 原有界工作队列1项通过。原命令、几何事件与恢复重试共用同一提交动作，完成结果回原EventWindow队列；同一自有HWND同时挂起居中resize与圆角更新时，目标客户区和新区域均确认，工作队列排空后继续原夹具。

## 验收边界

当前Windows11、96DPI。DPI144/192仅为换算断言；未执行跨物理显示器DPI切换。测试不安装全局hook、不启动普通产品、不访问真实剪贴板或用户历史。真实GPUI边缘像素、拖动缩放/Snap、IME、被动窗口点击与用户视觉验收独立记录。
