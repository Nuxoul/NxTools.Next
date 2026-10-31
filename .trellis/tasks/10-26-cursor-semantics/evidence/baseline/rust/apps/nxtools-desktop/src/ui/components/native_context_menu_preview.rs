//! Debug-only owned GPUI -> scratch transport -> production C# menu lifecycle fixture.
//! No ClipboardApplication, product pipe, history/config, plugins or native hooks.
use super::*;
use gpui_kit::base::ElementExt as _;
use gpui_kit::{Bounds, Context, Render, WindowBounds, WindowOptions, size};
use std::{path::PathBuf, time::Duration};

struct Source {
    _trigger: Rc<Cell<bool>>,
    focus: FocusHandle,
}
impl Render for Source {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let trigger = self._trigger.clone();
        div()
            .size_full()
            .track_focus(&self.focus)
            .on_prepaint(move |_, _, _| trigger.set(true))
            .child("隔离菜单验证 · 300×440")
    }
}
pub(crate) fn open(cx: &mut App) -> anyhow::Result<()> {
    let scratch = PathBuf::from(std::env::var("PI_SCRATCH_DIR")?).canonicalize()?;
    let directory = PathBuf::from(std::env::var("NXTOOLS_PREVIEW_MENU_DIR")?).canonicalize()?;
    anyhow::ensure!(
        directory.starts_with(&scratch) && directory != scratch,
        "owned scratch fixture directory required"
    );
    init(cx);
    let (bridge, endpoint) = nxtools_ipc::platform_bridge(16);
    endpoint.set_connected_for_contract_test(true);
    let platform = Rc::new(bridge);
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                point(px(100.0), px(100.0)),
                size(px(300.0), px(440.0)),
            ))),
            titlebar: None,
            focus: true,
            show: true,
            ..Default::default()
        },
        move |window, cx| {
            window.set_window_title("NxTools.MenuLifecycleFixture");
            println!(
                "ui.menu_fixture.source hwnd={} pid={}",
                crate::ui::native_window_handle(window).unwrap_or_default(),
                std::process::id()
            );
            let trigger = Rc::new(Cell::new(true));
            let token = Rc::downgrade(&trigger);
            let focus = cx.focus_handle();
            focus.focus(window, cx);
            let previous = focus.clone();
            let source = cx.new(|_| Source {
                _trigger: trigger,
                focus,
            });
            window.defer(cx, move |window, cx| {
                let selected = |_: &ClickEvent, _: &mut Window, _: &mut App| {
                    println!("ui.menu_fixture.selected");
                };
                open_with_source(
                    vec![
                        MenuItem::new("fixture-copy", "复制（合成动作）").on_click(selected),
                        MenuItem::new("fixture-disabled", "禁用动作")
                            .disabled(true)
                            .on_click(selected),
                        MenuItem::new("fixture-cascade", "来源分类").submenu(vec![
                            MenuItem::new("fixture-child", "合成分类")
                                .checked(true)
                                .on_click(selected),
                        ]),
                    ],
                    token,
                    point(px(299.0), px(30.0)),
                    Some(previous),
                    (platform, None),
                    window,
                    cx,
                );
            });
            cx.new(|cx| {
                gpui_kit::base::Root::new(source, window, cx).bg(gpui_kit::transparent_black())
            })
        },
    )?;
    cx.spawn(async move |cx| {
        let deadline = Instant::now() + Duration::from_secs(90);
        let mut waiting = None;
        let mut last_state = "preparing";
        while Instant::now() < deadline {
            if let Some(id) = waiting {
                let path = directory.join(format!("{id}.reply.json"));
                if let Ok(bytes) = std::fs::read(&path) {
                    if let Ok(result) =
                        serde_json::from_slice::<nxtools_protocol::PlatformCommandResult>(&bytes)
                    {
                        cx.update(|cx| {
                            handle_notification(
                                &PlatformNotification::CommandResult {
                                    correlation_id: id,
                                    result,
                                },
                                cx,
                            )
                        });
                    }
                    let _ = std::fs::remove_file(path);
                    waiting = None;
                }
            } else if let Ok(command) = endpoint.try_receive_command_for_contract_test() {
                let path = directory.join(format!("{}.command.json", command.id));
                if let Ok(bytes) = serde_json::to_vec(&command) {
                    let _ = std::fs::write(path, bytes);
                }
                waiting = Some(command.id);
            }
            let state = cx.update(|cx| {
                cx.global::<NativeMenuState>()
                    .pending
                    .as_ref()
                    .map_or("terminal", |p| if p.ready { "ready" } else { "preparing" })
            });
            if state != last_state {
                println!("ui.menu_fixture.state value={state}");
                let _ = std::fs::write(directory.join("state.txt"), state);
                last_state = state;
            }
            cx.background_executor()
                .timer(Duration::from_millis(10))
                .await;
        }
    })
    .detach();
    Ok(())
}
