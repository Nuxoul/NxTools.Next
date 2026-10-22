# 下载链接清单

Python 3.10+ 标准库，一次性 JSONL v1 `downloads/action`；两个明确的手动命令：

- `copy-lines`：复制原始链接，每行一个，保留任务选择顺序。
- `copy-markdown`：固定“下载 1”等标签，URL中 Markdown/HTML 特殊字符进行百分号编码，生成安全的链接目标。

## 使用

运行下方命令生成 ZIP，在扩展管理导入 `download-link-list.zip`，审查文件，单独信任解释器和代码，再明确启用。**导入不等于信任，信任不等于启用。** 在下载列表选择1–32个任务，从扩展动作中选择上述命令。此动作明确请求系统复制一次，但不自动粘贴。

只处理上下文 `task_ids`：非空、最多32个、无重复、规范带连字符非零 UUID；快照 task_id 必须逐字匹配请求。逐个 `downloads.get {task_id}`，使用真实 Desktop `task_snapshot` 的 `value.request.source_url`（原始 URL），**不使用 resolution 的改写地址**。全部验证后，才发送一个 `common.copy {text}`（没有 paths 或插件自造因果字段）。任何读取错误、身份不符、取消、无效URL、超限都不会复制部分清单；复制请求一旦发出，不论失败/EOF/结果丢失都不重试。不暂停、修改、提交或启动任务，不访问网络。

输入/输出文本最多64KiB UTF-8，帧最多256KiB含LF。仅接受无用户信息的HTTP(S) URL，拒绝控制/空白/反斜线；Markdown使用固定标签避免文件名注入。**原始 URL 本身仍可能包含敏感查询参数/签名；复制前后自行检查，不能把此包当作URL脱敏器。** 请求头/Cookie/Referrer和整个resolution均不输出，不复制到结果消息；任务快照可能包含这些字段，受信任脚本进程本身并非隔离沙箱。

权限只是信息声明，不限制整用户权限。测试仅使用假的宿主和合成任务，没有实际系统剪贴板或用户下载数据。

## 本地验证与打包（仓库根目录 PowerShell）

```powershell
$env:PYTHONDONTWRITEBYTECODE='1'
python -B docs/extensions/examples/download-link-list/test_main.py
python -B docs/extensions/examples/build-examples.py --build
```

包完整包含 main.py、protocol.py、test_main.py、README.md，无包外依赖。run ID 原样返回、call ID 严格匹配、拒绝重复JSON键；stdout仅JSONL，stderr不记录原文，终态消息固定且不带URL/路径/请求头。宿主拥有相关ID的接受/效果/终态诊断和超时；取消只能停止后续请求，不能撤销已完成的系统复制。实际原生复制/扩展菜单仍需独立交互验收。
