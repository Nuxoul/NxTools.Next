# Implementation plan

阶段名与顺序为权威顺序；稳定化工作归属当前阶段，不重新编号、不提前做后续阶段。

## Phase A — Baseline and references
1. 在 session scratch 记录扩展目录与 `BrowserExtensionPackageService.cs` 的哈希基线，以及工作区现有脏文件列表。
2. 复核 `docs/reference-code/download.md` 浏览器扩展章节与旧版 `D:\Code\NxTools\browser_extension`；
   把 Chrome `onDeterminingFilename` 返回值约定、MV3 `background.scripts`/`service_worker` 双键兼容性的
   官方文档证据写入 `research/platform-facts.md`。

## Phase B — Behavior harness (R2)
3. 建 `tests/harness/chrome-api.cjs`、`tests/harness/load-background.cjs`，加载当前未拆分的真实 `background.js`。
4. 写 R1.1–R1.6 的行为测试，确认它们在当前代码上为红；把 `native-lifecycle` 改为 harness 版本并保持绿。

## Phase C — Defect fixes (R1)
5. R1.1 suggest 返回值与单次调用。
6. R1.2 manifest 双键、条件 `importScripts`、`CreateFirefoxManifest` 保留脚本列表；C# 测试。
7. R1.3 拖放 accepted 判定；R1.4 路由发送方检查与 `sanitizeSettings`；R1.5 下载标签页解析；R1.6 降级/会话存储。
8. 全部测试转绿；更新 `tests/architecture/browser-native-messaging.ps1` 的 `importScripts` 断言。

## Phase D — Background split (R3)
9. 按 design.md 顺序逐模块搬移，每步运行 `node --test`；入口 ≤ 200 行。
10. manifest `background.scripts` / `importScripts` / 磁盘文件一致性测试。

## Phase E — Content script split (R3)
11. 拆 `content-script.js`，manifest 与 `injectContentBridge` 文件列表一致；运行 layout/redesign 浏览器测试。

## Phase F — Frontend redesign (R4)
12. 视觉方向已定（A，见 design.md R4）；先改测试夹具与几何断言（标签栏以上 ≤120px、默认视口资源 ≥4 项、
    空闲无底部状态条），确认在现有界面上为红。
13. 拆 `popup.js` 并实现新弹窗 HTML/CSS/token 与 `shadow-styles.js` 注入面样式。
14. 更新 `tests/layout-browser.cjs`、`tests/redesign-browser.cjs`、`tests/popup-fixture.js`；
    运行 `tests/build-preview.cjs` 生成预览与状态图库，逐张实际查看。
15. 请用户目检并记录视觉认可。

## Phase G — Spec and wrap-up
16. 更新 `.trellis/spec/browser-extension/{index,contracts,ui-guidelines}.md` 与扩展 README。
17. 运行全部校验，记录 `outcomes.md`（实际检查与未覆盖的安装版/真实浏览器限制）。

## Validation commands (repo root, PowerShell)
```powershell
$extensionTests = (Get-ChildItem browser_extension/nxtools-interceptor/tests -Recurse -Filter '*.test.js').FullName; node --test $extensionTests
node browser_extension/nxtools-interceptor/tests/layout-browser.cjs
node browser_extension/nxtools-interceptor/tests/redesign-browser.cjs
node browser_extension/nxtools-interceptor/tests/build-preview.cjs
powershell -File tests/architecture/browser-native-messaging.ps1
dotnet test csharp/NxTools.PlatformHost.Tests   # 仅 Phase C 步骤 6；按 verification.md 的测试选择规则
python tests/architecture/spec-contracts.py
```
不需要 Cargo（Rust 未改动）。若 Phase C 发现必须改 Rust 宿主，先停下请用户授权并遵守 CARGO_* 限制。

## Risky files / rollback points
- `background.js` 拆分（Phase D）：每模块一个提交点。
- `manifest.json`：只允许新增 `background.scripts` 与内容脚本文件；权限/CSP/key 变化即回退。
- `BrowserExtensionPackageService.cs`：仅 Firefox manifest 生成逻辑。

## Not authorized by this task
安装扩展、注册 Native Messaging、导出签名包、打包发布、真实网页/Cookie、提交与推送（除非用户另行要求）。
