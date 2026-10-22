# 剪贴板文本工具

Python 3.10+ 标准库，一次性 JSONL v1 `clipboard/action`；没有自动 `committed` 监听器。

## 使用

运行下方命令生成 ZIP，在扩展管理导入 `clipboard-text-tools.zip` 并审查文件，独立信任 Python 解释器/代码，再明确启用。**导入、信任、启用相互独立，初始默认禁用且未信任。** 在剪贴板历史明确选择一个文本条目，从扩展动作中选择：

- `unique-lines`：按第一次出现的顺序去除完全重复的行，不排序、不trim、不做Unicode标准化。CRLF和CR统一为LF；保留第一次出现的空行。例：`乙/甲/乙`（三行）→ `乙/甲`。
- `pretty-json`：严格解析JSON，拒绝重复键、NaN/Infinity（包括溢出指数）、语法错误、非法Unicode；对象/数组嵌套最多32层。保留对象插入顺序及Unicode，2空格缩进。不是任意精度数字保真工具：小数由Python浮点解析，可能重新格式化/舍入；原文始终保留。
- `uri-decode`：仅做**一次**严格UTF-8百分号解码，`+`保持加号，不作表单空格、不递归、不打开URL。拒绝不完整/非十六进制转义、非法UTF-8和解码后的C0/C1控制字符（包括换行、NUL）。`a+b%20%E4%B8%AD` → `a+b 中`。

只读取 `context.selection` 的 `item_id/kind/text`。非文本或空文本不处理；输入/输出最多64KiB UTF-8，JSONL帧最多256KiB含LF。结果未变化或只有空白时不调用宿主；有变化且非空才调用**一次** `clipboard.create_derived {source_item_id,text}`。验证返回新条目ID，不覆盖/删除原条目，**没有 common.copy、clipboard.copy、粘贴或自动打开面板**。固定反馈消息不包含原文或转换后内容。

## 本地验证与打包（仓库根目录 PowerShell）

```powershell
$env:PYTHONDONTWRITEBYTECODE='1'
python -B docs/extensions/examples/clipboard-text-tools/test_main.py
python -B docs/extensions/examples/build-examples.py --build
```

main.py、包内protocol.py、测试和本文均列入files，独立包不需要SDK或pip依赖。测试仅用合成文本与假宿主，不访问真实剪贴板历史/用户配置。

权限声明仅供审查，不是沙箱；信任授予整用户权限。所有run/call ID严格匹配，重复JSON键拒绝，失败/取消/EOF不重试。派生请求已经提交后取消不保证回滚；宿主拥有取消进程、因果和相关ID生命周期诊断。stdout只含JSONL，stderr不记录内容；不读网络、不写日志。实际原生运行/菜单交互和用户验收仍需单独完成。
