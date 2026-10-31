//! Official Kit menu on an independent transparent GPUI viewport, never the source HWND.
use super::{menu::MenuTone, project_menu::MenuItem};
use gpui_kit::base::Root;
use gpui_kit::component::{
    ActiveTheme, Icon, Side,
    label::Label,
    menu::{PopupMenu, PopupMenuItem},
};
use gpui_kit::{
    AnyWindowHandle, App, Bounds, Context, DismissEvent, Entity, Focusable, IntoElement, Pixels,
    Point, Render, Subscription, Window, WindowBackgroundAppearance, WindowBounds, WindowKind,
    WindowOptions, anchored, deferred, div, point, prelude::*, px, size,
};
use uuid::Uuid;

pub(super) struct Surface {
    pub window: AnyWindowHandle,
    pub view: Entity<MenuSurface>,
    pub hwnd: i64,
}
pub(super) fn open(
    id: Uuid,
    items: Vec<MenuItem>,
    source: &Window,
    cx: &mut App,
) -> anyhow::Result<Surface> {
    let mut view = None;
    let mut hwnd = None;
    let window: AnyWindowHandle = cx
        .open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    point(px(0.0), px(0.0)),
                    size(px(640.0), px(480.0)),
                ))),
                titlebar: None,
                kind: WindowKind::PopUp,
                show: false,
                focus: false,
                is_movable: false,
                is_resizable: false,
                is_minimizable: false,
                display_id: source.display(cx).map(|display| display.id()),
                window_background: WindowBackgroundAppearance::Transparent,
                ..Default::default()
            },
            |window, cx| {
                window.set_window_title("NxTools.ClipboardMenu");
                hwnd = crate::ui::native_window_handle(window);
                #[cfg(test)]
                if hwnd.is_none() {
                    hwnd = Some(-2);
                }
                let content = cx.new(|cx| MenuSurface::new(id, items, window, cx));
                view = Some(content.clone());
                cx.new(|cx| Root::new(content, window, cx).bg(gpui_kit::transparent_black()))
            },
        )?
        .into();
    match (view, hwnd) {
        (Some(view), Some(hwnd)) => Ok(Surface { window, view, hwnd }),
        _ => {
            let _: anyhow::Result<()> = window.update(cx, |_, window, _| window.remove_window());
            anyhow::bail!("menu HWND unavailable")
        }
    }
}

pub(super) struct MenuSurface {
    id: Uuid,
    items: Vec<MenuItem>,
    menu: Entity<PopupMenu>,
    anchor: Point<Pixels>,
    placed: bool,
    ever_active: bool,
    _dismiss: Subscription,
    _activation: Subscription,
}
impl MenuSurface {
    fn new(id: Uuid, items: Vec<MenuItem>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let menu = build_menu(items.clone(), true, window, cx);
        let dismiss = cx.subscribe(&menu, move |_, _, _: &DismissEvent, cx| {
            // PopupMenu is still leased. Its on_click precedes DismissEvent; terminal is first-wins.
            cx.defer(move |cx| super::native_context_menu::finish(id, None, false, "escape", cx));
        });
        let activation = cx.observe_window_activation(window, move |view, window, cx| {
            if window.is_window_active() {
                view.ever_active = true;
            }
            if view.placed && view.ever_active && !window.is_window_active() {
                cx.defer(move |cx| {
                    super::native_context_menu::finish(id, None, true, "deactivated", cx)
                });
            }
        });
        Self {
            id,
            items,
            menu,
            anchor: point(px(0.0), px(0.0)),
            placed: false,
            ever_active: false,
            _dismiss: dismiss,
            _activation: activation,
        }
    }
    pub(super) fn place(
        &mut self,
        anchor: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.anchor = anchor;
        self.placed = true;
        // Hidden HWNDs start at CW_USEDEFAULT; build against the confirmed native viewport.
        self.menu = build_menu(self.items.clone(), true, window, cx);
        let id = self.id;
        self._dismiss = cx.subscribe(&self.menu, move |_, _, _: &DismissEvent, cx| {
            cx.defer(move |cx| super::native_context_menu::finish(id, None, false, "escape", cx));
        });
        self.menu.focus_handle(cx).focus(window, cx);
        cx.notify();
    }
}
fn build_menu(
    items: Vec<MenuItem>,
    ancestor_enabled: bool,
    window: &mut Window,
    cx: &mut App,
) -> Entity<PopupMenu> {
    PopupMenu::build(window, cx, move |mut menu, window, cx| {
        menu = menu
            .min_w(px(224.0))
            .max_w(px(280.0))
            .max_h((window.viewport_size().height - px(16.0)).max(px(26.0)))
            .scrollable(true)
            .check_side(Side::Right);
        for item in items {
            let enabled = ancestor_enabled && item.eligible();
            let mut rendered = if item.separator {
                PopupMenuItem::separator()
            } else if !item.children.is_empty() {
                let child = build_menu(item.children, enabled, window, cx);
                PopupMenuItem::submenu(item.label, child)
            } else {
                // Public element slot preserves the original ID/accessibility label and semantic
                // destructive foreground; the official row still owns hover/check/disabled/input.
                let label = item.label;
                let selector = item.id;
                let danger = item.tone == MenuTone::Danger;
                let checked = item.checked;
                let handler = item.handler;
                PopupMenuItem::element(move |_, cx| {
                    div()
                        .id(selector.clone())
                        .aria_label(label.clone())
                        .debug_selector({
                            let selector = selector.clone();
                            move || selector.to_string()
                        })
                        .flex_1()
                        .min_w_0()
                        .when(danger && enabled, |row| row.text_color(cx.theme().danger))
                        .child(Label::new(label.clone()).text_sm().truncate())
                })
                .checked(checked.unwrap_or(false))
                .on_click(move |event, window, cx| {
                    if enabled && let Some(handler) = handler.as_ref() {
                        handler(event, window, cx);
                    }
                })
            }
            .disabled(!enabled);
            if let Some(icon) = item.icon {
                rendered = rendered.icon(
                    Icon::new(icon).when(item.tone == MenuTone::Danger && enabled, |icon| {
                        icon.text_color(cx.theme().danger)
                    }),
                );
            }
            menu = menu.item(rendered);
        }
        menu
    })
}
impl Render for MenuSurface {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::theme::observe_theme(window, cx);
        let id = super::native_context_menu::surface_session(window, cx);
        // Kit's own outside handler runs after capture. Mark outside first, then retire next turn.
        div()
            .size_full()
            .capture_key_down(move |_, _, cx| {
                if let Some(id) = id {
                    super::native_context_menu::mark_keyboard(id, cx);
                }
            })
            .capture_any_mouse_down(move |event, window, cx| {
                if let Some(id) = id {
                    super::native_context_menu::mark_pointer_position(
                        id,
                        event.position,
                        window,
                        cx,
                    );
                }
            })
            .child(
                deferred(
                    anchored()
                        .position(self.anchor)
                        .snap_to_window_with_margin(px(8.0))
                        .child(
                            div()
                                .debug_selector(|| "styled-menu-content".into())
                                .child(self.menu.clone()),
                        ),
                )
                .with_priority(gpui_kit::base::POPUP_PRIORITY),
            )
    }
}

#[cfg(test)]
#[path = "styled_menu_surface_tests.rs"]
mod tests;

#[cfg(debug_assertions)]
pub(crate) fn open_isolated_preview(cx: &mut App) -> anyhow::Result<()> {
    if std::env::var_os("NXTOOLS_PREVIEW_MENU_DIR").is_some() {
        return super::native_context_menu::open_isolated_preview(cx);
    }
    let _scratch = std::env::var("PI_SCRATCH_DIR")?;
    let items = vec![
        MenuItem::new("preview-paste", "粘贴").icon(crate::icons::MaterialIcon::ContentPaste),
        MenuItem::new("preview-copy", "复制").icon(crate::icons::MaterialIcon::ContentCopy),
        MenuItem::new("preview-plain", "纯文本粘贴").icon(crate::icons::MaterialIcon::TextFields),
        MenuItem::separator(),
        MenuItem::new("preview-edit", "编辑").icon(crate::icons::MaterialIcon::Edit),
        MenuItem::new("preview-favorite", "收藏")
            .icon(crate::icons::MaterialIcon::Star)
            .checked(true),
        MenuItem::new("preview-disabled", "暂不可用的动作").disabled(true),
        MenuItem::new("preview-delete", "删除")
            .icon(crate::icons::MaterialIcon::Delete)
            .tone(MenuTone::Danger),
        MenuItem::separator(),
        MenuItem::new("preview-categories", "来源分类").submenu(vec![
            MenuItem::new("preview-category", "合成分类").checked(true),
        ]),
        MenuItem::new("preview-extensions", "扩展处理").submenu(vec![MenuItem::new(
            "preview-extension",
            "按原顺序去重行（保留原文）",
        )]),
    ];
    cx.open_window(
        crate::main_window::visual_preview::final_gallery::options(720.0, 480.0),
        move |window, cx| {
            let view = cx.new(|cx| MenuSurface::new(Uuid::new_v4(), items, window, cx));
            view.update(cx, |view, cx| {
                view.place(point(px(280.0), px(30.0)), window, cx)
            });
            crate::main_window::visual_preview::final_gallery::ready(
                "clipboard-styled-menu",
                720.0,
                480.0,
                window,
            );
            cx.new(|cx| Root::new(view, window, cx))
        },
    )?;
    Ok(())
}
