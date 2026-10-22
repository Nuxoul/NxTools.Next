# 独立 Python 扩展示例

这三个包都是 Python **3.10+ 标准库**的一次性 JSONL v1 扩展，不依赖pip、常驻服务或包外SDK。每个目录完整包含 `extension.json` 和全部 manifest `files`（main、包内protocol helper、README、测试）。现有 `download-resolver/` 和根目录 `clipboard-cleaner/` 示例保持不变；本构建器只打包下表三个新包。

| 包 | 功能 | 效果边界 |
| --- | --- | --- |
| [github-public-proxy](github-public-proxy/README.md) | GitHub公开release/archive/raw链接添加可配置HTTPS前缀 | 默认未同意第三方；纯URL改写，不发HTTP，不读取/转发凭据 |
| [download-link-list](download-link-list/README.md) | 复制选中任务的原始链接清单/Markdown | 1–32个任务逐个读取，完整校验后仅一次common.copy，不修改/启动下载 |
| [clipboard-text-tools](clipboard-text-tools/README.md) | 原序去重行、严格JSON格式化、百分号解码 | 选中文本≤64KiB，仅有变化且非空才一次create_derived，原文保留，不复制/粘贴 |

## 正式程序内置

修正版正式程序直接内嵌这三个包，首次启动会自动登记到“扩展”页，无需手动导入ZIP；旧配置首次升级也会补齐缺少的条目。它们默认**未信任、未启用**，仍须核实Python环境并明确授权；不会安装Python、联网或自动执行。已有同ID包与其设置／授权优先，不自动覆盖；初始化后删除的包不会在下次启动恢复。独立ZIP继续用于明确导入或恢复。

## 构建和验证（仓库根目录 PowerShell）

```powershell
$env:PYTHONDONTWRITEBYTECODE='1'
python -B docs/extensions/examples/test_examples.py -v
# 也可单独测试任一包：
python -B docs/extensions/examples/github-public-proxy/test_main.py
python -B docs/extensions/examples/download-link-list/test_main.py
python -B docs/extensions/examples/clipboard-text-tools/test_main.py
# 只有明确 --build 才写ZIP，不运行插件或修改安装器：
python -B docs/extensions/examples/build-examples.py --build
```

输出仅到 `artifacts/extension-examples/{github-public-proxy,download-link-list,clipboard-text-tools}.zip`，每包根部直接是 `extension.json`，只写manifest及files，固定排序、1980时间戳、普通文件权限和ZIP_STORED，重复构建字节一致。已存在且字节相同则保留；不同则拒绝覆盖，以免破坏既有产物（需要更新时自行备份/移走自己拥有的旧示例ZIP）。不会把整个脏工作树或缓存打包。

Python测试全部采用合成数据/隔离子进程/假宿主，不访问真实剪贴板、下载、配置、网络或服务。`rust/crates/nxtools-extensions/tests/bundled_examples.rs`另有真实 ImportPreview 目录/自制临时ZIP导入与内容寻址暂存测试：三个manifest、全部handler和设置、默认禁用/未信任、不可从可变目录直接安装、源ZIP移除后快照仍可验证；不执行插件或触碰真实用户配置。Rust测试由维护者按仓库串行/内存策略单独执行，本Python命令不触发Cargo。

## 导入与使用

1. 先阅读对应README及源码，在扩展管理导入生成的单个ZIP，检查声明。
2. 选择/核实Python运行环境，**独立明确同意信任代码和解释器**。
3. 再明确启用扩展。导入不授予信任，信任不自动启用；代码/运行环境改变需要重新批准。
4. 下载清单/文本工具从已选择条目的扩展手动动作触发；GitHub解析器启用后还必须在设置中勾选`third_party_opt_in`。没有自动剪贴板监听器。

**权限声明只是信息，不是沙箱。** 整包信任授予普通用户权限，不能将这些示例当作恶意代码隔离机制。JSONL帧最多256KiB（含LF），拒绝重复键、非法UTF-8、错误run/call ID、非布尔success等；stdout仅协议、stderr不输出原文；终态固定不回显URL/文本/主机错误。调用ID逐次匹配，效果请求最多一次、不重试。取消/EOF/宿主失败不能撤销已经提交的复制或派生；宿主还负责进程截止时间和相关ID的接受/状态/终态诊断，示例不重复输出载荷日志。

`https://ghproxy.net/`只是配置示例，**不是国内可用性、安全性、速度或在线率背书**。镜像可见URL和内容，必须从独立上游核验签名/校验和。仅使用已确认公开的资源；无网络的路径限制不能证明远端公开状态。自定义域名DNS/重定向不在本例验证范围。下载清单中的原始URL仍可能带敏感查询参数，分享前请检查。详见每个包的安全限制。

协议以[PROTOCOL.md](../PROTOCOL.md)为准。本切片不更改产品UI/宿主协议，也不代表原生运行、国内网络或用户交互已验收。
