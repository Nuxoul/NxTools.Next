# 迟到指令补充 — 只读核查，不改变冻结交付

针对父会话消息 `96c04dd3-ca42-4b8c-ba3a-c64d6f39cf0b`，本补充不改生产／测试源码、不重跑测试、不修改既有 delivery-manifest／outcomes／历史日志，也不推进下一项。原 manifest SHA256 `bea18e4b467792ba69e46cb825d96b405a63e684634c1638e92ea64d27371924` 保持；补充证据独立在 `evidence/late-instruction/`。

## CreateOverwrite keyboard：UNRESOLVED / NOT ACCEPTED

完整失败 probe 已在实际产品测试文件保留，没有删断言、改成期望0、ignore 或移出回归。focused 6/7 和唯一完整 Desktop 1207/1208 的失败计数不变，不能把 probe 算成绿灯。父允许拆出为诊断 evidence，但这是可选方案；本次不再改动共享测试文件。

补充件复制该 probe 完整源码，并保存当前生产测试文件哈希。实际 focused Button 断言在 `job_tests.rs:1091`，随后明确 Space KeyDown/KeyUp；日志在 `:1102` 期望1而实际0，因此此前 focus 断言实际执行且通过。没有独立读屏/UIA/native 焦点验收，也没有虚构截图。

仅对任务前 dirty baseline 与当前 progress.rs 做只读逐字行比对（只去行首尾缩进）：CreateOverwrite 的原 DialogAction.child(Button) 行、on_ok、on_cancel、Conflict 决策回调行全部一致，精确内容保存在 `footer-compare.json`。locked0.8.0 的 Component Button 仅 when_some(on_click) 转发激活，外层 DialogAction div.on_click 派发 Confirm 的源码链保持。此为源码因果支持，**没有运行旧 UI／旧程序对照**。未为了旧基线复现回写／还原共享 progress.rs；先前交付中的受控 UI slot mutation 只验证对象展示测试红灯，不是旧键盘复现，已恢复哈希。

Create pointer、独立 missing-object cancel、Conflict Tab+Space 正向已实际运行通过；不能据此宣称 CreateOverwrite keyboard／默认 Enter／全部确认按钮／整体无障碍通过。probe 内失败后的组合 fallback 段未执行，独立 fallback 用例才是该项正向证据。

## Protocol 79/80：修正归因措辞

**本补充替代原 outcomes/manifest 中把此失败直接称为“无关”的绝对归因**。准确表述是：本项归档 display 测试6/6通过；Protocol 全套当次79/80，default-apps 负例失败保留；一次有界只读调用链和 own-delta 核查没有发现本项 display 修改进入 default-apps 分支，但这不是完整因果证明，不能仅因本项未改 lib.rs 就断定无关或宣称全套当前已绿。

原失败日志：`protocol-green.log` 的 panic 为 `lib.rs:2332:13`，断言 `serde_json::from_value::<PlatformCommand>({"command":"open_default_apps_settings","data":data}).is_err()`，79PASS/1FAIL。原断言未输出循环变量值，因此**不能从现有日志确定具体哪一个输入导致失败**，也不能补造那个输入。当前源测试四个输入是：`{"uri":"ms-settings:privacy"}`、`{"path":"C:\\x"}`、`null`、`[]`；完整当前片段已保存。

补充时只读当前 lib.rs SHA256 为 `b19d21a17d720365839374ed51394be717b395d549ea04fc5263f128c24a6384`，测试已在2342行，当前491–513行有专用 Deserialize Visitor，仅 deserialize_map/空对象。这与原失败日志的坐标／源码时点不能混用；**未保留原失败时 lib.rs 的完整哈希／快照，未运行当前负例或整套，不能宣称已修复**。当前外部变化只观察、不覆盖、不归因于本项。

只读关联：PlatformCommand 为按 command/data 标签派发的 enum，ArchiveJob 与 OpenDefaultAppsSettings 是不同类型分支；新 display 只在 ArchiveJobEvent.prompt 中出现，未修改此 enum、default-apps DTO/parser/test 或 serde 依赖。原本项13份 exact own-delta 无 lib.rs／Cargo manifest／lock 修改。这个有限证据支持分支隔离，但仍保留上面的状态与因果限制。

## 补充检查与停止

本次仅 Read/Grep、哈希、保存原内容副本和四行比较；没有 Cargo/dotnet／产品／服务／GUI／文件操作授权修改。补充清单分别哈希 probe、当前源码摘录、只读哈希、原失败日志副本和 footer 比较；原冻结81份证据不重写。现停止，父独立审查前不自 accept、不启动第7项。
