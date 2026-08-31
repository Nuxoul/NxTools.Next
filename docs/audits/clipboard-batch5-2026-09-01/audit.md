# 批次 5(剪贴板保护性迁移)回归基准索引

日期:2026-09-01
收口提交:`e58458d`。全部对照在同一台机器(Windows 10.0.22631,2560×1440,100% DPI)捕获,
弹窗对照统一为 316×449 窗口几何,设置页对照统一为 1076×509;实例均使用独立
`NXTOOLS_BASELINE_DATA_ROOT` 数据根并在捕获后退出,未触碰用户真实数据。

## 迁移前后对照对(固定基准)

| 切片 | 迁移前 | 迁移后 | 内容 |
| --- | --- | --- | --- |
| CL-01 主窗口设置 | `clipboard-settings-cl-01-2026-09-01/01-…pre-migration-baseline.png`(工作树 `36db4aa` 渲染) | `02-…post-migration-current.png` | 私有 5px 圆角双栏 → 共享方角 `settings_split_surface` |
| CL-02 弹窗令牌与菜单 | `clipboard-popup-cl-02-2026-09-01/01-…pre-migration-baseline.png` | `02-…post-migration-current.png` | 66 处旧常量退役;菜单迁入共享壳层 |
| CL-03 历史行外壳 | `clipboard-history-cl-03-2026-09-01/01-…pre-migration-baseline.png`(3 条真实历史) | `02-…post-migration-current.png` | 手写行外壳 → 共享 `compact_result_row` |
| CL-04 分类栏 | 复用 CL-03 迁移后图(同提交 d5e434e,12px text_xs) | `clipboard-category-cl-04-2026-09-01/03-…post-CL-04.png` | 分类条/按钮/把手 → Label/Meta 令牌 |
| CL-05 预览窗 | 确认性切片,无渲染变化 | — | 共享 PreviewShell seam + 96 项资源上限进静态契约 |
| 批次收口 | — | `clipboard-batch5-2026-09-01/final-clipboard-popup-batch5-close.png` | `e58458d` 批次收口实拍 |

## 已验证的保护项(不得回退)

- 弹窗 300×440 默认尺寸与 8px 安全边距;分类横向滚轮(`category_wheel_is_horizontal_clamped_and_uses_vertical_wheel_input`)与拖动排序(`category_drop_reorders_before_target_or_at_end`)。
- 图片比例算法与 270px 预览上限(`image_preview_preserves_aspect_ratio_without_upscaling` 等);96 项图片/图标资源 LRU(`image_resource_cache_evicts_the_oldest_entry_beyond_capacity`)。
- 面板/预览/粘贴生命周期与 Rust/C#/GPUI 职责边界未动;粘贴、捕获、分类持久化行为无变化。

## 静态门禁(每次回归必跑)

- `tests/architecture/ui-token-boundaries.ps1`(令牌 + 退役旧令牌规则)
- `tests/architecture/clipboard-ui-lifecycle.ps1`(CL-01~05 全部契约)
- `tests/architecture/ui-state-lifecycle.ps1`(共享空/加载/错误状态)

## 证据边界

- 空历史数据无法呈现右键菜单与历史行悬停操作的像素对照;菜单呈现由共享组件的 DL-05 视觉门禁
  与源码契约门禁锁定,待后续有真实使用数据时可补拍。
- 125%/150% DPI 截图矩阵与 Release 包人工验收保留给用户,不因批次收口而标记完成。
