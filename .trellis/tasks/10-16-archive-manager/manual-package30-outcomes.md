# 用户验收包30 — 最新浏览器UI完整Release

2026-10-27，按用户“打包我验收”完成完整Release和安装器，未安装/启动产品。

## 交付身份

- `artifacts/installer/NxTools.Next-setup-0.7.0.exe`，sequence30，显示版本0.7.0。
- releaseID `6f9c2aea-f033-444e-99b7-ec912e5ff0a4`。
- 25,925,344字节，约24.72MiB。
- SHA256 `39FE76B7653280E735D77D8F30490E609A4EA34E5A4AD9A8F345AE10E62742F2`。
- manifest SHA256 `16fd223e71c69a9ac2125498c152bb0f591313f932c8424e346c996b10c8dc19`。
- 历史29号 `installfix` 后缀包仍保留，但不含本轮UI，不用于当前验收；旧包备份位于本次包装器目录。

## 包含的本轮修复

[名称列表/详细信息](browser-name-view-outcomes.md)和[图标/渲染修复](browser-render-repair-outcomes.md)：默认名称列表、独立模式持久化、实际视口宽度、原查看菜单、共享原生文件类型图标、UI字体、工具栏下拉几何和大小显示。完整重建Rust程序及C#主机/helper，桌面程序哈希与29号不同。继续包含[安装准备提权/日志修订](installer29-installfix-outcomes.md)，未放宽原身份及回滚检查。

## 实测证据

- 持久入口 `scripts/package-local.ps1 -SkipVerification`，原包装器/控制器实际退出0；预检提交内存16.92GiB、空闲构建通道，未降低6GiB门禁。低内存Cargo环境由入口初始化。
- 发布序号30，原密钥/账本；内部RSA3072-PSS/SHA256独立验签通过，未输出私钥。
- 77文件大小/哈希/精确集合、10构建产物、5归档批准、helper桌面哈希绑定、公共信任锚、浏览器扩展/许可源一致性和安装器host pin通过。
- 1940源文件构建前后哈希一致，独立复核仍一致；新图标模块、浏览器、应用路由和UI-state均在基线。
- 本次安装器9隔离场景、BrowserHost真实文件替换/回滚5隔离场景全部 `pass=True`；预期失败/取消场景非零码不是总体失败。另3个受保护准备场景仅历史修订已测，本次未重跑。
- 安装包Authenticode `NotSigned`，不是内部发布签名失败。
- 两个既有Release警告保留：SPACING_12 unused import、animated unused field。

## 边界

复用刚完成的67相关回归及Desktop测试严格Clippy，明确跳过全量验证矩阵，未新跑完整Desktop/CLI/格式/资源/大文件/性能集合。构建和合成场景不等于实际安装或视觉验收。未安装、启动产品、继续观察受保护部署、修改注册表/COM或推进后续功能。原错误码1根因未确认；整体UI和A–F最终验收状态不因本次打包转绿。

## 持久路径

- `.diagnostics/packaging/20261027-040516-cb07b264/`：status、原始package.log、previous-package、previous-installer。
- `.diagnostics/packaging/ui30-controller-20261027-040513/`：控制器状态、1940文件基线、空漂移清单、独立package-verification.json。
- 当前交付说明：[INSTALL.md](../../../artifacts/installer/INSTALL.md)，[evidence.json](../../../artifacts/installer/evidence.json)。
