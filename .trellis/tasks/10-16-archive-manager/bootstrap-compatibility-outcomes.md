# Bootstrap compatibility for ordinary updates

2026-11-16。普通新发布包复用已兼容的受保护 Bootstrap，不再因产品发布序列递增重复显示更新组件迁移确认。

## 行为

- `scripts/package.ps1` 独立记录最低 Bootstrap 维护序列82，并通过现有 Inno include/检测参数传递。
- 已完成82及之后维护、marker/登记/哈希一致且无未完成维护时，普通新包返回 `compatible`。
- marker缺失、低于兼容门槛、未完成维护、公钥轮换和身份不一致继续走原有维护或拒绝边界。
- 需要新的 Bootstrap 能力时显式提高兼容门槛；应用包签名、精确文件清单及防回退发布序列不变。
- 不改C#更新服务、不引入新状态文件或信任锚，不改真实安装状态。

## 验证

- 迁移架构回归92项通过，实际x86/x64检测入口覆盖包序列999、marker82、最低门槛82，以及旧marker、缺失marker、journal、篡改、无效marker、公钥轮换、缺失目录和访问拒绝。
- x86/x64边界控制：恢复按当前包发布序列判断时返回10 / `migration_required`；使用兼容门槛82时返回0 / `compatible`。
- 真实机器只读决策：Bootstrap marker86、模拟包序列999、兼容门槛82，返回 `skip`；未调用维护、服务或安装命令。
- 原Inno隔离runtime fixture的维护成功、取消、失败、篡改、修复成功/取消/失败/篡改及检测失败共9个相关场景通过。

## 已知限制

修复已进入[Release88](release88-package-outcomes.md)，兼容门槛82随正式安装器传入。本轮完整安装器runtime16项和BrowserHost替换/回滚5项通过；全量Rust/C#测试未重跑。真实覆盖安装和UAC验收未执行。外层包装器退出码未取得，产物签名、成员、精确清单和版本采用独立核验结果。归档保持 **Phase E — Format expansion and ZIP editing / in_progress**，B4保持暂停。
