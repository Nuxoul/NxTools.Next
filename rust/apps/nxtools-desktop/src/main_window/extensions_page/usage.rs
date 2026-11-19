//! Read-only purpose/trigger projection. Saved intent is not live runtime verification.
//! No IO, capability inference, invocation, input state or native lifecycle owner.
use super::*;
use nxtools_extensions::{DeclarativeAction, Handler, InstalledPackage};

pub(super) const ENABLEMENT_NOTE: &str =
    "启用不等于自动订阅；自动事件与普通查询参与需在「自动化」中单独开启。";

pub(super) fn purpose(package: &InstalledPackage) -> &str {
    let description = package.manifest.description.trim();
    if description.is_empty() {
        "作者未提供用途说明；请根据下方入口和来源判断是否使用。"
    } else {
        description
    }
}

pub(super) fn has_unfilled_settings(package: &InstalledPackage) -> bool {
    use nxtools_extensions::SettingType;
    // Registry load already validates stored values, allowing an empty required field for setup.
    package.manifest.runtime.is_some()
        && package.manifest.settings.iter().any(|setting| {
            if !setting
                .validation
                .as_ref()
                .is_some_and(|rules| rules.required)
            {
                return false;
            }
            let default = match &setting.field {
                SettingType::Text { default }
                | SettingType::File { default }
                | SettingType::Folder { default } => default,
                _ => return false,
            };
            package
                .settings
                .get(&setting.key)
                .and_then(serde_json::Value::as_str)
                .unwrap_or(default)
                .trim()
                .is_empty()
        })
}

pub(super) fn readiness(package: &InstalledPackage) -> &'static str {
    if package.enabled
        && has_unfilled_settings(package)
        && extensions_settings::current_trust(package)
    {
        return "已启用 · 待完成必填设置，脚本暂不可运行";
    }
    match (
        package.manifest.runtime.is_some(),
        extensions_settings::current_trust(package),
        package.enabled,
    ) {
        (false, _, true) => "已启用 · 声明式动作，不启动插件脚本",
        (false, _, false) => "已停用 · 检查动作后可启用",
        (true, false, _) => "需确认信任 · 请在「用途与启用」检查来源和运行环境",
        (true, true, false) => "已停用 · 信任记录匹配当前内容，启用时仍需验证",
        (true, true, true) => "已启用 · 信任记录匹配当前内容，调用前仍需验证运行环境",
    }
}

pub(super) fn is_event(handler: &Handler) -> bool {
    matches!(
        (handler.feature.as_str(), handler.entry.as_str()),
        ("downloads", "created" | "completed" | "failed") | ("clipboard", "committed")
    )
}

pub(super) fn entry_label(handler: &Handler) -> &'static str {
    match (handler.feature.as_str(), handler.entry.as_str()) {
        ("quick_launch", "command") => "关键词命令",
        ("quick_launch", "provider") => "查询提供者",
        ("clipboard", "action") | ("downloads", "action") => "手动动作",
        ("clipboard", "committed") => "记录写入后",
        ("downloads", "resolve") => "链接解析",
        ("downloads", "created") => "任务创建后",
        ("downloads", "completed") => "任务完成后",
        ("downloads", "failed") => "任务失败后",
        ("tools", "action") => "命名工具动作",
        _ => "扩展入口",
    }
}

pub(super) fn handler_heading(handler: &Handler) -> String {
    format!(
        "{} · {} · {}",
        handler.title,
        workspace::feature_label(&handler.feature),
        entry_label(handler)
    )
}

/// Describe only the actual entry boundary. The manifest does not declare input/output types.
pub(super) fn trigger(package: &InstalledPackage, handler: &Handler) -> String {
    match (handler.feature.as_str(), handler.entry.as_str()) {
        ("clipboard", "action") => format!("剪贴板历史 → 条目菜单 → 扩展处理 → {}", handler.title),
        ("downloads", "action") => format!("下载 → 任务菜单 → 扩展动作 → {}", handler.title),
        ("quick_launch", "command") => format!(
            "快速启动 → 输入关键词（可在空格后附查询）：{}",
            keywords(package, handler)
        ),
        ("quick_launch", "provider") => {
            let keywords = package.keywords(handler);
            if keywords.is_empty() {
                "快速启动 → 普通查询；需先在「自动化」开启此提供者。".into()
            } else {
                format!(
                    "快速启动 → 关键词：{}；普通查询另需在「自动化」开启。",
                    keywords.join("、")
                )
            }
        }
        ("downloads", "resolve") => {
            if handler.url_matches.is_empty() {
                "下载 → 链接解析；未声明自动匹配规则，需在「下载解析链」指定此入口。".into()
            } else {
                let rules = handler
                    .url_matches
                    .iter()
                    .map(|rule| {
                        format!(
                            "主机 {}{}，路径 {}",
                            rule.host,
                            if rule.include_subdomains {
                                "（含子域名）"
                            } else {
                                ""
                            },
                            rule.path
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("；");
                format!("下载 → HTTP(S) 链接解析；声明匹配：{rules}。也可由「下载解析链」指定。")
            }
        }
        ("tools", "action") => {
            let titles = match &handler.action {
                DeclarativeAction::Items { items } => items
                    .iter()
                    .map(|item| item.title.as_str())
                    .collect::<Vec<_>>()
                    .join("、"),
                _ => handler.title.clone(),
            };
            format!("此扩展 → 命令与工具 → 选择命名动作：{titles}")
        }
        ("downloads", "created") => "自动触发：下载任务创建后；在「自动化」独立订阅。".into(),
        ("downloads", "completed") => "自动触发：下载任务完成后；在「自动化」独立订阅。".into(),
        ("downloads", "failed") => "自动触发：下载任务失败后；在「自动化」独立订阅。".into(),
        ("clipboard", "committed") => "自动触发：剪贴板记录写入后；在「自动化」独立订阅。".into(),
        _ => "作者声明的扩展入口；请检查「来源与能力」。".into(),
    }
}

fn keywords(package: &InstalledPackage, handler: &Handler) -> String {
    let keywords = package.keywords(handler);
    if keywords.is_empty() {
        "未配置关键词（在「命令与工具」修改）".into()
    } else {
        keywords.join("、")
    }
}

pub(super) fn handler_state(package: &InstalledPackage, handler: &Handler, paused: bool) -> String {
    let gate = if package.manifest.runtime.is_some() && !extensions_settings::current_trust(package)
    {
        "需确认信任"
    } else if !package.enabled {
        "扩展已停用"
    } else if package.manifest.runtime.is_some() {
        "扩展已启用，调用前验证运行环境"
    } else {
        "扩展已启用"
    };
    if is_event(handler) {
        let intent = if paused {
            "自动事件已暂停；在「自动化」重试"
        } else if package.enabled_events.contains(&handler.id) {
            "已订阅自动事件"
        } else {
            "未订阅自动事件"
        };
        format!("{intent} · {gate}")
    } else if handler.feature == "quick_launch" && handler.entry == "provider" {
        let intent = if package.enabled_providers.contains(&handler.id) {
            "已加入普通查询"
        } else {
            "未加入普通查询"
        };
        format!("{intent} · {gate}")
    } else {
        gate.into()
    }
}

/// The selector contains the actual displayed copy, not a parallel test-only label.
/// This lets production-render regressions reject a purpose/usage omission or wrong entry text.
pub(super) fn copy(selector: impl Into<String>, text: impl Into<String>) -> gpui_kit::AnyElement {
    let text = text.into();
    div()
        .debug_selector(|| format!("{}:{text}", selector.into()))
        .w_full()
        .min_w_0()
        .flex_shrink_0()
        .whitespace_normal()
        .text_sm()
        .child(text)
        .into_any_element()
}

impl ExtensionManagerView {
    pub(super) fn package_usage(
        &self,
        package: &InstalledPackage,
        cx: &Context<Self>,
    ) -> SettingGroup {
        let snapshot = package.clone();
        let mut items = vec![
            SettingItem::render(move |_, _, _| {
                div()
                    .w_full()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(copy("extension-detail-purpose", purpose(&snapshot)))
                    .child(copy("extension-detail-readiness", readiness(&snapshot)))
            })
            .keywords([
                "用途".into(),
                "说明".into(),
                package.manifest.description.clone(),
            ]),
        ];
        for handler in &package.manifest.handlers {
            let heading = handler_heading(handler);
            let trigger = trigger(package, handler);
            let state = handler_state(
                package,
                handler,
                self.application
                    .read(cx)
                    .event_paused(&package.manifest.id, &handler.id),
            );
            let selector = format!("extension-detail-trigger-{}", handler.id);
            let search = vec![heading.clone(), trigger.clone(), state.clone()];
            items.push(
                SettingItem::render(move |_, _, _| {
                    div()
                        .w_full()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(heading.clone())
                        .child(copy(selector.clone(), trigger.clone()))
                        .child(copy(format!("{selector}-state"), state.clone()))
                })
                .keywords(search),
            );
        }
        items.push(
            SettingItem::render(|_, _, _| copy("extension-enablement-note", ENABLEMENT_NOTE))
                .keywords(["启用", "自动化", "订阅", "普通查询"]),
        );
        SettingGroup::new().title("用途与使用方式").items(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn package() -> InstalledPackage {
        serde_json::from_value(serde_json::json!({
            "manifest":{"schema_version":1,"id":"usage.fixture","name":"Demo only","version":"1",
                "handlers":[{"id":"action","title":"整理文本","feature":"clipboard","entry":"action",
                    "action":{"type":"script","entry_file":"fixture.py"}}]},
            "content_hash":"fixture","source":"demo fixture"
        })).expect("read-only fixture; never admitted for execution")
    }

    #[test]
    fn extension_usage_copy_uses_actual_entries_and_independent_intents() {
        let mut package = package();
        assert!(purpose(&package).contains("作者未提供用途说明"));
        let mut handler = package.manifest.handlers[0].clone();
        assert_eq!(
            trigger(&package, &handler),
            "剪贴板历史 → 条目菜单 → 扩展处理 → 整理文本"
        );
        handler.feature = "downloads".into();
        handler.entry = "completed".into();
        package.enabled = true;
        assert!(handler_state(&package, &handler, false).contains("未订阅自动事件"));
        package.enabled_events.insert(handler.id.clone());
        assert!(handler_state(&package, &handler, false).contains("已订阅自动事件"));
        assert!(handler_state(&package, &handler, true).contains("已暂停"));
        assert!(trigger(&package, &handler).contains("下载任务完成后"));
        handler.feature = "quick_launch".into();
        handler.entry = "provider".into();
        assert!(handler_state(&package, &handler, false).contains("未加入普通查询"));
        package.enabled_providers.insert(handler.id.clone());
        assert!(handler_state(&package, &handler, false).contains("已加入普通查询"));
        handler.entry = "command".into();
        package
            .aliases
            .insert(handler.id.clone(), vec!["clean".into()]);
        assert!(trigger(&package, &handler).contains("clean"));
        handler.feature = "tools".into();
        handler.entry = "action".into();
        handler.action = DeclarativeAction::Items {
            items: vec![nxtools_extensions::FixedItem {
                id: "named".into(),
                title: "查看说明".into(),
                action: Box::new(DeclarativeAction::Folder {
                    path: "C:\\Synthetic".into(),
                }),
            }],
        };
        assert!(trigger(&package, &handler).contains("命令与工具 → 选择命名动作：查看说明"));
        package.manifest.runtime = Some(nxtools_extensions::ScriptRuntime::once(
            nxtools_extensions::ScriptProgram::Python,
        ));
        assert!(readiness(&package).contains("需确认信任"));
        assert!(!readiness(&package).contains("已验证"));
    }

    #[test]
    fn extension_usage_resolver_describes_public_host_path_not_invented_output() {
        let package = package();
        let mut handler = package.manifest.handlers[0].clone();
        handler.feature = "downloads".into();
        handler.entry = "resolve".into();
        assert!(trigger(&package, &handler).contains("未声明自动匹配规则"));
        handler.url_matches = vec![nxtools_extensions::UrlMatch {
            host: "example.test".into(),
            include_subdomains: true,
            path: "/watch/*".into(),
            priority: 0,
        }];
        let text = trigger(&package, &handler);
        assert!(text.contains("主机 example.test（含子域名），路径 /watch/*"));
        assert!(!text.contains("视频") && !text.contains("输出"));
    }
}
