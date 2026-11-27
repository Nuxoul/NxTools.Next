//! Debug-only synthetic real-HWND fixture; scratch transport, no product runtime/IPC/user data.
use super::*;
use gpui_kit::InputEvent as _;

struct Source {
    center: Entity<NotificationCenter>,
}
impl Render for Source {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .relative()
            .bg(crate::theme::Theme::active(cx).canvas)
            .text_color(crate::theme::Theme::active(cx).text)
            .p_4()
            .child("隔离屏幕通知验证 · 此窗口内部不应出现通知")
            .child(super::super::presentation::render(
                self.center.clone(),
                true,
                48.0,
                window,
                cx,
            ))
    }
}
pub(crate) fn open(cx: &mut App) -> anyhow::Result<()> {
    let scratch = std::path::PathBuf::from(std::env::var("PI_SCRATCH_DIR")?).canonicalize()?;
    let directory =
        std::path::PathBuf::from(std::env::var("NXTOOLS_PREVIEW_SCREEN_NOTIFICATION_DIR")?)
            .canonicalize()?;
    anyhow::ensure!(
        directory.starts_with(&scratch) && directory != scratch,
        "owned scratch fixture directory required"
    );
    super::preview_cadence::init(cx);
    let reduced_fixture =
        std::env::var("NXTOOLS_PREVIEW_NOTIFICATION_REDUCED").as_deref() == Ok("1");
    if std::env::var("NXTOOLS_PREVIEW_NOTIFICATION_CADENCE").as_deref() == Ok("1") {
        cx.set_reduce_motion(reduced_fixture);
        cx.set_global(crate::ui::motion::UiMotionPreference::Known {
            reduced_motion: reduced_fixture,
        });
    }
    let position: NotificationPosition = serde_json::from_value(serde_json::Value::String(
        std::env::var("NXTOOLS_PREVIEW_SCREEN_NOTIFICATION_POSITION")?,
    ))?;
    let center = cx.new(|_| NotificationCenter::default());
    let rendered = center.clone();
    let source: AnyWindowHandle = cx
        .open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    point(px(500.0), px(200.0)),
                    size(px(620.0), px(320.0)),
                ))),
                focus: false,
                show: true,
                ..Default::default()
            },
            move |window, cx| {
                window.set_window_title("NxTools.ScreenNotificationFixture");
                println!(
                    "ui.screen_fixture.source hwnd={} pid={}",
                    crate::ui::native_window_handle(window).unwrap_or_default(),
                    std::process::id()
                );
                let content = cx.new(|_| Source { center: rendered });
                cx.new(|cx| gpui_kit::base::Root::new(content, window, cx))
            },
        )?
        .into();
    let (bridge, endpoint) = nxtools_ipc::platform_bridge(16);
    endpoint.set_connected_for_contract_test(true);
    attach(center.clone(), Rc::new(bridge), source, cx);
    center.update(cx, |center, cx| {
        center.set_position(position, cx);
        center.push(
            AppNotification::new(
                "screen-fixture",
                NotificationLevel::Success,
                "扩展处理完成",
                "已保存 1 个结果，原文保留",
            )
            .actions(vec![
                NotificationAction::new("查看结果", |_, _| {}),
                NotificationAction::new("详情", |_, _| {}),
            ])
            .duration(None),
            cx,
        );
    });
    let stack_fixture = std::env::var("NXTOOLS_PREVIEW_OFFICIAL_STACK").as_deref() == Ok("1");
    if stack_fixture {
        center.update(cx, |center, cx| {
            center.show_progress("stack-second", "第二个操作", "这条不应覆盖前一条", cx);
            center.show_progress("stack-third", "第三个操作", "悬停展开官方堆叠", cx);
        });
    }
    let exit_fixture =
        std::env::var("NXTOOLS_PREVIEW_SCREEN_NOTIFICATION_EXIT").as_deref() == Ok("1");
    if exit_fixture && !reduced_fixture {
        cx.set_global(crate::ui::motion::UiMotionPreference::Known {
            reduced_motion: false,
        });
    }
    cx.spawn(async move |cx| {
        let deadline = Instant::now() + Duration::from_secs(90);
        let mut waiting = None;
        let mut was_visible = false;
        let mut exit_started = false;
        let mut material_facts_loaded = false;
        while Instant::now() < deadline {
            if !material_facts_loaded
                && let Ok(bytes) = std::fs::read(directory.join("material.facts.json"))
                && let Ok(facts) = serde_json::from_slice::<
                    nxtools_protocol::window_material::SystemMaterialCapabilitiesChanged,
                >(&bytes)
            {
                material_facts_loaded = cx.update(|cx| {
                    let mut state = crate::ui::material::UiMaterialCapabilities::default();
                    state.connected();
                    if !state.apply(facts) {
                        return false;
                    }
                    cx.set_global(state);
                    true
                });
            }
            if directory.join("cadence.finish").exists() {
                if let Some(summary) = cx.update(|cx| super::preview_cadence::snapshot(cx)) {
                    let _ = std::fs::write(
                        directory.join("cadence.json"),
                        serde_json::to_vec(&summary).expect("cadence summary"),
                    );
                }
                cx.update(|cx| cx.quit());
                break;
            }
            let visible_ready = cx.update(|cx| {
                let host = cx.global::<ScreenGlobal>().0.read(cx);
                host.visible && host.pending.is_none()
            });
            if visible_ready != was_visible {
                was_visible = visible_ready;
                cx.update(|cx| {
                    super::preview_cadence::mark(
                        if visible_ready { "visible" } else { "hidden" },
                        cx,
                    )
                });
            }
            if stack_fixture && visible_ready {
                let request = directory.join("interaction.request");
                if let Ok(action) = std::fs::read_to_string(&request) {
                    let _ = std::fs::remove_file(request);
                    if action.trim() == "dismiss" {
                        center.update(cx, |center, cx| {
                            let ids = center
                                .items()
                                .iter()
                                .map(|item| (item.id.clone(), item.generation()))
                                .collect::<Vec<_>>();
                            for (id, generation) in ids {
                                center.dismiss_generation(id, generation, cx);
                            }
                        });
                    } else {
                        let surface = cx.update(|cx| {
                            cx.global::<ScreenGlobal>()
                                .0
                                .read(cx)
                                .surface
                                .as_ref()
                                .map(|surface| surface.window)
                        });
                        if let Some(surface) = surface {
                            let action = action.trim().to_owned();
                            let _ = surface.update(cx, |_, window, cx| {
                                let viewport = window.viewport_size();
                                let pointer = if action == "hover" {
                                    point(
                                        viewport.width / 2.0,
                                        if position.is_top() {
                                            px(42.0)
                                        } else {
                                            viewport.height - px(42.0)
                                        },
                                    )
                                } else {
                                    point(px(2.0), px(2.0))
                                };
                                window.dispatch_event(
                                    gpui_kit::MouseMoveEvent {
                                        position: pointer,
                                        pressed_button: None,
                                        modifiers: Default::default(),
                                    }
                                    .to_platform_input(),
                                    cx,
                                );
                                window.refresh();
                                window.draw(cx).clear(cx);
                            });
                        }
                    }
                    let _ = std::fs::write(directory.join("interaction.accepted"), action);
                }
            }
            if exit_fixture
                && visible_ready
                && !exit_started
                && directory.join("dismiss.request").exists()
            {
                exit_started = true;
                cx.update(|cx| super::preview_cadence::mark("dismiss", cx));
                center.update(cx, |center, cx| {
                    if let Some(generation) = center.items().first().map(|item| item.generation()) {
                        center.dismiss_generation("screen-fixture", generation, cx);
                    }
                });
                let _ = std::fs::write(directory.join("exit.started"), b"started");
            }
            if let Some(id) = waiting {
                let reply_path = directory.join(format!("{id}.reply.json"));
                if let Ok(bytes) = std::fs::read(&reply_path) {
                    if let Ok(result) =
                        serde_json::from_slice::<nxtools_protocol::PlatformCommandResult>(&bytes)
                    {
                        cx.update(|cx| {
                            let notification = PlatformNotification::CommandResult {
                                correlation_id: id,
                                result: result.clone(),
                            };
                            let _ = endpoint.route_command_result_for_contract_test(id, result);
                            crate::ui::material::runtime::handle_notification(&notification, cx);
                            handle(notification, cx)
                        });
                    }
                    let _ = std::fs::remove_file(reply_path);
                    waiting = None;
                }
            } else if let Ok(command) = endpoint.try_receive_command_for_contract_test() {
                let path = directory.join(format!("{}.command.json", command.id));
                if let Ok(bytes) = serde_json::to_vec(&command) {
                    let _ = std::fs::write(path, bytes);
                }
                waiting = Some(command.id);
            }
            cx.background_executor()
                .timer(Duration::from_millis(20))
                .await;
        }
    })
    .detach();
    Ok(())
}
