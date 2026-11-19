# 0.7.2 Release93

2026-11-18。产品和浏览器扩展升级至0.7.2，正式打包完成，原打包进程退出0，包装器状态`Completed`。使用冻结源码、独立缓存和输出目录；Bootstrap最低兼容门槛保持82。仅交付正常安装器，未生成外层ZIP。

## 产物

| 项目 | 结果 |
| --- | --- |
| 产品 / 扩展版本 | 0.7.2 / 0.7.2 |
| 发布序号 | 93 |
| Release ID | `a3b31996-202f-4189-a983-cdbce857fb77` |
| 安装器 | `artifacts/installer/NxTools.Next-setup-0.7.2.exe` |
| 大小 | 30,010,699 字节 / 28.62 MiB |
| SHA256 | `52A40AB991B46884469BF4995A79D92FC430B47128021B10A89EDC3C800BA65A` |
| Manifest SHA256 | `e4d0cf19da77adad3ac0c3036d2c12a87d3ee8d99404076ac8bbfb9cfee9c01e` |
| 发布公钥 SHA256 | `a7caa726f091b61620879c2935e37b85eac286714045fc71d158cad04c3beb68` |
| 内部发布签名 | RSA3072-PSS/SHA256，独立验证通过 |
| 签名成员 / 精确散装文件 | 107 / 109，全部匹配 |
| Bootstrap最低兼容门槛 | 82 |
| Windows数字文件版本 | Desktop / PlatformHost / ShellSelectionHelper / 安装器均0.7.2.0 |
| Windows Authenticode | `NotSigned` |

## 版本与兼容性

Rust工作区及14个第一方锁文件包、C#程序集、正常/维护安装器和扩展清单统一为0.7.2。与升级前基线逐项比对，第三方锁文件数据不变，扩展清单仅version改变，保留扩展公钥/ID、权限和脚本顺序。AppId、发布信任、协议/模式版本和Bootstrap兼容门槛不变。

## 本轮验证

- 版本一致性5项、package-local12项、源码分发5项、构建树清理8项、扩展Node325项及隔离浏览器签名7项通过；C#测试宿主构建0警告/0错误。版本回归已验证旧版本定义会失败。
- 冻结4,764个源码输入，复制前后内容一致，构建期间0漂移；缓存从本会话Release92独立缓存复制，未使用工作区target作为正式发布输出。
- 安装器runtime16项、BrowserHost替换/回滚5项全部通过，记录来自本轮`package.log`。
- 独立核验签名、公钥、107成员大小/哈希、精确109文件、94个扩展源文件、5个Rust构建输出、实际Windows版本、迁移Host绑定和兼容门槛82。
- 安装器哈希与本轮`status.json`及旁侧`.sha256`一致；归档对应源码ZIP完整可读。发布到默认路径后签名成员和安装器读回一致。
- `-SkipVerification`复用既有产品验证，未重复全量Rust/C#测试；既有产品结果见[Release92](release92-package-outcomes.md)及其引用记录，不计入本轮新执行。

持久证据：`.diagnostics/packaging/20261118-163004-3d7330f0-published93/`。原始`status.json`记录隔离构建路径，`published-verification.json`记录最终交付路径，`source-capture/`保留冻结输入与缓存证据；`source-capture/version-change/`保留版本基线、保留性比对和定向检查结果。前一0.7.1 Release92保留于`previous-package/`和`previous-installer/`。

## 已知限制

真实覆盖安装、UAC、服务/注册表操作和物理UI验收未执行；对应源码材料的独立离线重构建未执行。归档保持 **Phase E — Format expansion and ZIP editing / in_progress**，B4保持暂停；未提交或推送。
