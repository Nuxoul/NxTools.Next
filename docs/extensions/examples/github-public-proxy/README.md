# GitHub 公开下载代理示例

Python 3.10+ 标准库，一次性 JSONL v1 `downloads/resolve`。此包**只改写字符串，不发起 HTTP、不探测镜像、不读取或请求凭据**。

## 使用

1. 在仓库根目录运行下方测试和 ZIP 构建命令。
2. 在扩展管理中导入生成的 `github-public-proxy.zip`，检查声明和全部文件。**导入、信任 Python 解释器/代码、启用是三个独立步骤**；导入后默认禁用且未信任。
3. 在设置中检查 `proxy_prefix`。默认 `https://ghproxy.net/` **仅作示例**，不是推荐或经验证的国内可用服务；也没有速度、安全性或在线率承诺。自建服务可填 `https://mirror.example.org/public/`（普通 HTTPS 域名、443、普通路径、末尾 `/`）。保存前自行核实运营方。
4. 阅读风险后明确勾选 `third_party_opt_in`；默认 false 时返回清晰的 not_handled，继续原始路径。启用包后，符合匹配的下载解析将使用该设置。
5. 仅使用你已确认公开的无凭据资源，例如 `https://github.com/OWNER/REPO/releases/download/v1/file.zip`、`https://github.com/OWNER/REPO/archive/refs/heads/main.zip`、`https://raw.githubusercontent.com/OWNER/REPO/main/file.txt`。

## 风险和边界

- 第三方镜像能看到完整 URL、访问时间和下载内容，可能替换内容。必须从**独立上游渠道**核验校验和及签名；镜像给出的校验和不构成独立验证。
- 本任务提供的上游已审阅资料：[hunshcn/gh-proxy README](https://raw.githubusercontent.com/hunshcn/gh-proxy/master/README.md)。它描述将完整 URL 加到代理地址后面的用法，也提醒公开演示服务可能超载。此处只采用该前缀转换形式，**不据此认证 ghproxy.net 的运营关系、安全性或当前可用性**；本地测试不联网。
- 无网络意味着无法判断仓库可见性。public-only 是保守的路径/凭据限制，**不是远端公开状态证明**；私有、保密、带令牌资源不得使用。
- 只接受精确 `github.com` 的 release asset/archive 路径和精确 `raw.githubusercontent.com` 的 owner/repo/ref/file 路径；不接受网页、blob、API、子域或相似域名。禁止 HTTP、非443、用户信息、任何查询/片段（包括空 `?`）、控制字符、反斜线、百分号编码、非 ASCII/异常路径、点段、空路径段。此保守子集会拒绝部分合法地址，不尝试“修复”。
- 当前有效请求含任何请求头、Cookie、Referrer 时拒绝转换（不只检查 Authorization）；不调用 credentials.get。成功值显式 `headers:[]/cookie:null/referrer:null`，有效期60秒。代理域名不能再次匹配输入，不能递归包裹。
- 自定义前缀禁止用户信息、查询/片段、IP字面量（包括公网IP）、数字IPv4变体、本地域名以及上游源站；仅允许普通域名和保守路径。**不解析 DNS，不防御 DNS 重绑定、运营方重定向或网络侧攻击**。
- 权限声明只供信息审查，不是沙箱。信任即授予当前用户权限；不应运行未经审查的修改版。

## 本地验证与打包（仓库根目录 PowerShell）

```powershell
$env:PYTHONDONTWRITEBYTECODE='1'
python -B docs/extensions/examples/github-public-proxy/test_main.py
python -B docs/extensions/examples/build-examples.py --build
```

所有脚本、包内协议 helper、README 和测试均列入 files，无包外依赖。JSONL每帧含LF最多256KiB；输出只含协议，stderr不记录内容。失败、取消和不适用消息均固定且不携带 URL/凭据。宿主负责 run/correlation 生命周期及取消进程；本包不另写日志。实际镜像网络/国内连通性、原生运行环境和用户交互未在这些测试中验证。
