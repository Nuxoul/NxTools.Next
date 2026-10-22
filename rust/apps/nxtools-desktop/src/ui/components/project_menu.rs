//! Shared menu model/trigger and source-frame lifetime guard.
//! Production renders official PopupMenu in an independent GPUI window; legacy rows are test-only.
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Instant;

use gpui_kit::base::{Button, ElementExt, Root, RootPlugin};
use gpui_kit::component::Icon;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::{
    AnyElement, App, Bounds, ClickEvent, Context, FocusHandle, IntoElement, KeyDownEvent,
    KeyUpEvent, MouseButton, Pixels, Point, Render, RenderOnce, Role, ScrollHandle, SharedString,
    Window, deferred, div, point, prelude::*, px, size,
};
use uuid::Uuid;

use crate::icons::MaterialIcon;
use crate::theme::Theme;
use crate::ui::components::menu::MenuTone;
use crate::ui::metrics::{CONTROL_HEIGHT_ACTION, RADIUS_CONTROL, SPACING_4};
use crate::ui::typography::{LINE_HEIGHT_BODY, TEXT_BODY, UI_FONT_FAMILY};

pub(super) type Handler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
const EDGE: f32 = SPACING_4;
const WIDTH: f32 = 240.0;
const ROW: f32 = CONTROL_HEIGHT_ACTION;
gpui_kit::actions!(project_menu, [DismissMenu]);
const PAD: f32 = SPACING_4;

#[derive(Clone)]
pub(crate) struct MenuItem {
    pub(super) id: SharedString,
    pub(super) label: SharedString,
    pub(super) icon: Option<MaterialIcon>,
    pub(super) tone: MenuTone,
    pub(super) disabled: bool,
    pub(super) checked: Option<bool>,
    pub(super) children: Vec<MenuItem>,
    pub(super) handler: Option<Handler>,
    pub(super) separator: bool,
}
impl MenuItem {
    pub(crate) fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            tone: MenuTone::Normal,
            disabled: false,
            checked: None,
            children: Vec::new(),
            handler: None,
            separator: false,
        }
    }
    pub(crate) fn icon(mut self, icon: MaterialIcon) -> Self {
        self.icon = Some(icon);
        self
    }
    pub(crate) fn tone(mut self, tone: MenuTone) -> Self {
        self.tone = tone;
        self
    }
    pub(crate) fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub(crate) fn checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }
    pub(crate) fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.handler = Some(Rc::new(handler));
        self
    }
    pub(crate) fn submenu(mut self, children: Vec<Self>) -> Self {
        self.children = children;
        self
    }
    pub(crate) fn separator() -> Self {
        let mut item = Self::new("separator", "");
        item.separator = true;
        item.disabled = true;
        item
    }
    pub(super) fn eligible(&self) -> bool {
        !self.disabled && !self.separator
    }
}

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        gpui_kit::KeyBinding::new("tab", DismissMenu, Some("ProjectMenu")),
        gpui_kit::KeyBinding::new("shift-tab", DismissMenu, Some("ProjectMenu")),
    ]);
    super::native_context_menu::init(cx);
    Root::register_plugin(cx, ProjectMenu::new);
}

/// Retained on the real trigger's keyed lifetime, not by the open menu.
#[derive(gpui_kit::IntoElement)]
struct MenuTrigger {
    id: SharedString,
    content: AnyElement,
    items: Vec<MenuItem>,
}
pub(crate) fn context_menu(
    id: impl Into<SharedString>,
    content: AnyElement,
    items: Vec<MenuItem>,
) -> AnyElement {
    MenuTrigger {
        id: id.into(),
        content,
        items,
    }
    .into_any_element()
}
impl RenderOnce for MenuTrigger {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let lifetime = window.use_keyed_state(
            SharedString::from(format!("project-menu-trigger-{}", self.id)),
            cx,
            |_, _| Rc::new(Cell::new(true)),
        );
        let token = Rc::downgrade(lifetime.read(cx));
        let painted = lifetime.read(cx).clone();
        div()
            .id(self.id)
            .w_full()
            .child(self.content)
            .on_prepaint(move |_, _, _| painted.set(true))
            .on_mouse_down(MouseButton::Right, move |event, window, cx| {
                let items = self.items.clone();
                let token = token.clone();
                let position = event.position;
                let previous_focus = window.focused(cx);
                window.defer(cx, move |window, cx| {
                    if !token.upgrade().is_some_and(|token| token.get()) {
                        return;
                    }
                    let native_opened = super::native_context_menu::open(
                        items.clone(),
                        token.clone(),
                        position,
                        previous_focus.clone(),
                        window,
                        cx,
                    );
                    #[cfg(not(test))]
                    let _ = native_opened;
                    // Legacy geometry fixtures remain test-only; production never clips a menu
                    // into the source window when native preparation is unavailable.
                    #[cfg(test)]
                    if !native_opened
                        && let Some(menu) = Root::read(window, cx).plugin::<ProjectMenu>()
                    {
                        menu.update(cx, |menu, cx| {
                            menu.open_fixture(items, token, position, previous_focus, window, cx)
                        });
                    }
                });
                cx.stop_propagation();
            })
    }
}

pub(crate) fn dismiss_window(window: &mut Window, cx: &mut App) {
    super::native_context_menu::dismiss_window(window, cx);
    // A panel callback may be leased; plugin and Root work belongs to the next turn.
    window.defer(cx, |window, cx| {
        if let Some(root) = window.root::<Root>().flatten() {
            let menu = root.read(cx).plugin::<ProjectMenu>();
            if let Some(menu) = menu {
                menu.update(cx, |menu, cx| {
                    menu.dismiss("host_hidden", false, window, cx)
                });
            }
        }
    });
}

struct Session {
    items: Vec<MenuItem>,
    trigger: Weak<Cell<bool>>,
    position: Point<Pixels>,
    previous_focus: Option<FocusHandle>,
    id: Uuid,
    started: Instant,
}
struct ProjectMenu {
    session: Option<Session>,
    focus: FocusHandle,
    #[cfg(test)]
    return_focus: Option<FocusHandle>,
    selected: Option<usize>,
    child_selected: Option<usize>,
    child_open: bool,
    child_keyboard: bool,
    armed: Option<(String, bool, usize)>,
    root_scroll: ScrollHandle,
    child_scroll: ScrollHandle,
    root_rows: Rc<RefCell<Vec<Bounds<Pixels>>>>,
    child_rows: Rc<RefCell<Vec<Bounds<Pixels>>>>,
    _focus_subscription: gpui_kit::Subscription,
}
impl ProjectMenu {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        let subscription = cx.on_focus_out(&focus, window, |_, _, window, cx| {
            let entity = cx.weak_entity();
            window.defer(cx, move |window, cx| {
                let _ = entity.update(cx, |menu, cx| {
                    if !menu.focus.is_focused(window) {
                        menu.dismiss("focus_out", window.focused(cx).is_none(), window, cx);
                    }
                });
            });
        });
        Self {
            session: None,
            focus,
            #[cfg(test)]
            return_focus: None,
            selected: None,
            child_selected: None,
            child_open: false,
            child_keyboard: false,
            armed: None,
            root_scroll: ScrollHandle::new(),
            child_scroll: ScrollHandle::new(),
            root_rows: Rc::new(RefCell::new(Vec::new())),
            child_rows: Rc::new(RefCell::new(Vec::new())),
            _focus_subscription: subscription,
        }
    }
    #[cfg(test)]
    fn open_fixture(
        &mut self,
        items: Vec<MenuItem>,
        trigger: Weak<Cell<bool>>,
        position: Point<Pixels>,
        previous_focus: Option<FocusHandle>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let previous_focus = if previous_focus.as_ref() == Some(&self.focus) {
            self.return_focus.clone()
        } else {
            previous_focus
        };
        self.return_focus = previous_focus.clone();
        self.dismiss("replaced", false, window, cx);
        let id = Uuid::new_v4();
        tracing::info!(event="ui.menu.opened", correlation_id=%id, item_count=items.len(), "project menu opened");
        self.selected = items.iter().position(MenuItem::eligible);
        self.child_selected = None;
        self.child_open = false;
        self.child_keyboard = false;
        self.armed = None;
        self.root_scroll.set_offset(point(px(0.0), px(0.0)));
        self.child_scroll.set_offset(point(px(0.0), px(0.0)));
        self.session = Some(Session {
            items,
            trigger,
            position,
            previous_focus,
            id,
            started: Instant::now(),
        });
        self.focus.focus(window, cx);
        cx.notify();
    }
    fn dismiss(
        &mut self,
        reason: &'static str,
        restore: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(session) = self.session.take() {
            tracing::info!(event="ui.menu.closed", correlation_id=%session.id, reason, elapsed_ms=session.started.elapsed().as_millis(), "project menu closed");
            if restore && (self.focus.is_focused(window) || window.focused(cx).is_none()) {
                if let Some(focus) = session.previous_focus {
                    focus.focus(window, cx);
                } else {
                    window.blur(cx);
                }
            }
            self.armed = None;
            self.child_open = false;
            cx.notify();
        }
    }
    fn child_items(&self) -> &[MenuItem] {
        self.session
            .as_ref()
            .and_then(|s| self.selected.and_then(|ix| s.items.get(ix)))
            .map_or(&[], |item| item.children.as_slice())
    }
    fn open_child(&mut self, keyboard: bool, cx: &mut Context<Self>) {
        if self.child_items().is_empty() {
            return;
        }
        self.child_selected = self.child_items().iter().position(MenuItem::eligible);
        self.child_open = true;
        self.child_keyboard = keyboard;
        self.armed = None;
        self.child_scroll.set_offset(point(px(0.0), px(0.0)));
        cx.notify();
    }
    fn select(&mut self, child: bool, ix: usize, cx: &mut Context<Self>) {
        let item = if child {
            self.child_items().get(ix)
        } else {
            self.session.as_ref().and_then(|s| s.items.get(ix))
        };
        if !item.is_some_and(MenuItem::eligible) {
            return;
        }
        if child {
            self.child_keyboard = true;
            if self.child_selected != Some(ix) {
                self.armed = None;
                self.child_selected = Some(ix);
                cx.notify();
            }
        } else if self.selected != Some(ix)
            || (!self.child_open && !item.expect("eligible item").children.is_empty())
        {
            self.selected = Some(ix);
            self.armed = None;
            self.child_open = false;
            self.child_keyboard = false;
            self.open_child(false, cx);
            cx.notify();
        }
    }
    fn activate(
        &mut self,
        child: bool,
        ix: usize,
        event: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let item = if child {
            self.child_items().get(ix)
        } else {
            self.session.as_ref().and_then(|s| s.items.get(ix))
        }
        .cloned();
        let Some(item) = item.filter(MenuItem::eligible) else {
            return;
        };
        if !item.children.is_empty() {
            self.selected = Some(ix);
            self.open_child(true, cx);
            return;
        }
        let token = self.session.as_ref().map(|s| s.trigger.clone());
        self.dismiss("activated", true, window, cx);
        if let Some(handler) = item.handler {
            let event = event.clone();
            window.defer(cx, move |window, cx| {
                if token.is_some_and(|token| token.upgrade().is_some_and(|token| token.get())) {
                    handler(&event, window, cx);
                }
            });
        }
    }
    fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.session.is_none() || !self.focus.is_focused(window) {
            return;
        }
        let key = event.keystroke.key.as_str();
        if event.keystroke.modifiers.modified() && key != "tab" {
            self.armed = None;
            cx.stop_propagation();
            return;
        }
        let child = self.child_open && self.child_keyboard;
        match key {
            "escape" | "left" if self.child_open => {
                self.child_open = false;
                self.child_keyboard = false;
                self.armed = None;
                cx.notify();
            }
            "escape" | "tab" => self.dismiss("keyboard_dismiss", true, window, cx),
            "right" if !child => self.open_child(true, cx),
            "up" | "down" | "home" | "end" => {
                let items = if child {
                    self.child_items()
                } else {
                    &self.session.as_ref().expect("open menu").items
                };
                let eligible: Vec<_> = items
                    .iter()
                    .enumerate()
                    .filter(|(_, i)| i.eligible())
                    .map(|(ix, _)| ix)
                    .collect();
                if !eligible.is_empty() {
                    let current = if child {
                        self.child_selected
                    } else {
                        self.selected
                    };
                    let pos = eligible
                        .iter()
                        .position(|ix| Some(*ix) == current)
                        .unwrap_or(0);
                    let next = match key {
                        "home" => 0,
                        "end" => eligible.len() - 1,
                        "up" => (pos + eligible.len() - 1) % eligible.len(),
                        _ => (pos + 1) % eligible.len(),
                    };
                    if child {
                        self.child_selected = Some(eligible[next]);
                    } else {
                        self.selected = Some(eligible[next]);
                        self.child_open = false;
                    }
                    self.armed = None;
                    let scroll = if child {
                        &self.child_scroll
                    } else {
                        &self.root_scroll
                    };
                    scroll.scroll_to_item(eligible[next]);
                    cx.notify();
                }
            }
            "enter" | "space" if !event.is_held => {
                let selected = if child {
                    self.child_selected
                } else {
                    self.selected
                };
                self.armed = selected.map(|ix| (key.to_owned(), child, ix));
            }
            _ => {}
        }
        window.prevent_default();
        cx.stop_propagation();
    }
    fn key_up(&mut self, event: &KeyUpEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.session.is_none() || !self.focus.is_focused(window) {
            self.armed = None;
            return;
        }
        let armed = self.armed.take();
        if !event.keystroke.modifiers.modified()
            && let Some((key, child, ix)) = armed
            && key == event.keystroke.key
        {
            let event = ClickEvent::Keyboard(gpui_kit::KeyboardClickEvent {
                button: if key == "space" {
                    gpui_kit::KeyboardButton::Space
                } else {
                    gpui_kit::KeyboardButton::Enter
                },
                bounds: (if child {
                    &self.child_rows
                } else {
                    &self.root_rows
                })
                .borrow()
                .get(ix)
                .copied()
                .unwrap_or_default(),
            });
            self.activate(child, ix, &event, window, cx);
        }
        cx.stop_propagation();
    }
    fn panel(
        &self,
        items: &[MenuItem],
        child: bool,
        bounds: Bounds<Pixels>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = Theme::active(cx);
        let rows = if child {
            self.child_rows.clone()
        } else {
            self.root_rows.clone()
        };
        rows.borrow_mut().resize(items.len(), Bounds::default());
        let scroll = if child {
            &self.child_scroll
        } else {
            &self.root_scroll
        };
        let selected = if child {
            self.child_selected
        } else {
            self.selected
        };
        let mut content = div()
            .id(if child {
                "project-menu-child-scroll"
            } else {
                "project-menu-root-scroll"
            })
            .w_full()
            .h_full()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(scroll)
            .flex()
            .flex_col()
            .p(px(PAD));
        for (ix, item) in items.iter().enumerate() {
            if item.separator {
                content = content.child(
                    div()
                        .h(px(5.0))
                        .flex_none()
                        .py(px(2.0))
                        .child(div().h(px(1.0)).bg(theme.border_subtle)),
                );
                continue;
            }
            let item = item.clone();
            let observed_rows = rows.clone();
            let id = item.id.clone();
            let label_selector = format!("{}-label", item.id);
            let tooltip = item.label.clone();
            let is_selected = selected == Some(ix);
            let text = if item.disabled {
                theme.text_muted
            } else if item.tone == MenuTone::Danger {
                theme.danger
            } else {
                theme.text
            };
            let role = if item.checked.is_some() {
                Role::MenuItemCheckBox
            } else {
                Role::MenuItem
            };
            let row =
                Button::new(id.clone())
                    .accessibility_label(item.label.clone())
                    .role(role)
                    .when_some(item.checked, |row, checked| {
                        row.aria_toggled(if checked {
                            gpui_kit::accesskit::Toggled::True
                        } else {
                            gpui_kit::accesskit::Toggled::False
                        })
                    })
                    .focusable(false)
                    .disabled(item.disabled)
                    .selected(is_selected)
                    .debug_selector(move || id.to_string())
                    .w_full()
                    .h(px(ROW))
                    .flex_none()
                    .px(px(8.0))
                    .bg(if is_selected {
                        theme.element_selected
                    } else {
                        theme.surface_raised
                    })
                    .text_color(text)
                    .font_family(UI_FONT_FAMILY)
                    .text_size(px(TEXT_BODY))
                    .line_height(px(LINE_HEIGHT_BODY))
                    .hover(move |style| {
                        style.bg(if is_selected {
                            theme.element_selected
                        } else {
                            theme.surface_hover
                        })
                    })
                    .active(move |style| style.bg(theme.surface_pressed))
                    .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
                    .on_prepaint(move |bounds, _, _| observed_rows.borrow_mut()[ix] = bounds)
                    .on_hover(cx.listener(move |menu, hovered, _, cx| {
                        if *hovered {
                            menu.select(child, ix, cx);
                        }
                    }))
                    .on_click(cx.listener(move |menu, event, window, cx| {
                        menu.activate(child, ix, event, window, cx);
                        cx.stop_propagation();
                    }))
                    .child(
                        div()
                            .w_full()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .w(px(14.0))
                                    .flex_none()
                                    .when(item.checked == Some(true), |slot| {
                                        slot.child(Icon::new(MaterialIcon::Check).size(px(14.0)))
                                    })
                                    .when(item.checked != Some(true), |slot| {
                                        slot.when_some(item.icon, |slot, icon| {
                                            slot.child(Icon::new(icon).size(px(14.0)))
                                        })
                                    }),
                            )
                            .child(
                                div()
                                    .debug_selector(move || label_selector.clone())
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .child(item.label),
                            )
                            .child(div().w(px(14.0)).flex_none().when(
                                !item.children.is_empty(),
                                |slot| {
                                    slot.child(Icon::new(MaterialIcon::ChevronRight).size(px(14.0)))
                                },
                            )),
                    );
            content = content.child(row);
        }
        div()
            .id(if child {
                "project-menu-child"
            } else {
                "project-menu-root"
            })
            .debug_selector(move || {
                if child {
                    "project-menu-child".into()
                } else {
                    "project-menu-root".into()
                }
            })
            .absolute()
            .left(bounds.left())
            .top(bounds.top())
            .w(bounds.size.width)
            .h(bounds.size.height)
            .border_1()
            .border_color(theme.border)
            .rounded(px(RADIUS_CONTROL))
            .bg(theme.surface_raised)
            .occlude()
            .child(content)
            .into_any_element()
    }
}
fn menu_height(items: &[MenuItem], viewport: f32) -> f32 {
    (items
        .iter()
        .map(|item| if item.separator { 5.0 } else { ROW })
        .sum::<f32>()
        + PAD * 2.0
        + 2.0)
        .min((viewport - EDGE * 2.0).max(1.0))
}
fn clamp_position(
    position: Point<Pixels>,
    width: f32,
    height: f32,
    viewport: gpui_kit::Size<Pixels>,
) -> Bounds<Pixels> {
    Bounds::new(
        point(
            px(position
                .x
                .as_f32()
                .clamp(EDGE, (viewport.width.as_f32() - width - EDGE).max(EDGE))),
            px(position
                .y
                .as_f32()
                .clamp(EDGE, (viewport.height.as_f32() - height - EDGE).max(EDGE))),
        ),
        size(px(width), px(height)),
    )
}
impl RootPlugin for ProjectMenu {
    fn prepare(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        super::native_context_menu::prepare(window, cx);
        if let Some(token) = self
            .session
            .as_ref()
            .and_then(|session| session.trigger.upgrade())
        {
            token.set(false);
        }
    }
}
impl Render for ProjectMenu {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::theme::observe_theme(window, cx);
        let Some(session) = self.session.as_ref() else {
            return super::native_context_menu::guard(window, cx);
        };
        let viewport = window.viewport_size();
        let width = WIDTH.min((viewport.width.as_f32() - EDGE * 2.0).max(1.0));
        let root_bounds = clamp_position(
            session.position,
            width,
            menu_height(&session.items, viewport.height.as_f32()),
            viewport,
        );
        let root = self.panel(&session.items, false, root_bounds, cx);
        let child_items = self.child_items();
        let child_bounds = if self.child_open && !child_items.is_empty() {
            let row = self
                .selected
                .and_then(|ix| self.root_rows.borrow().get(ix).copied())
                .unwrap_or(root_bounds);
            // Overlapping narrow-client children retain a reachable parent edge.
            let child_width = width.min((viewport.width.as_f32() - EDGE * 2.0 - 24.0).max(1.0));
            let left_space = root_bounds.left().as_f32() - EDGE;
            let right_space = viewport.width.as_f32() - EDGE - root_bounds.right().as_f32();
            let x = if left_space >= child_width + EDGE {
                root_bounds.left() - px(child_width + EDGE)
            } else if right_space >= child_width + EDGE {
                root_bounds.right() + px(EDGE)
            } else if left_space >= right_space {
                px(EDGE)
            } else {
                viewport.width - px(child_width + EDGE)
            };
            Some(clamp_position(
                point(x, row.top()),
                child_width,
                menu_height(child_items, viewport.height.as_f32()),
                viewport,
            ))
        } else {
            None
        };
        let child = child_bounds.map(|bounds| self.panel(child_items, true, bounds, cx));
        deferred(
            div()
                .id("project-menu-layer")
                .absolute()
                .inset_0()
                .track_focus(&self.focus)
                .key_context("ProjectMenu")
                .on_action(cx.listener(|menu, _: &DismissMenu, window, cx| {
                    menu.dismiss("keyboard_dismiss", true, window, cx);
                    cx.stop_propagation();
                }))
                .capture_key_down(cx.listener(Self::key_down))
                .capture_key_up(cx.listener(Self::key_up))
                .on_any_mouse_down(cx.listener(
                    move |menu, event: &gpui_kit::MouseDownEvent, window, cx| {
                        if !root_bounds.contains(&event.position)
                            && !child_bounds.is_some_and(|b| b.contains(&event.position))
                        {
                            menu.dismiss("outside_pointer", true, window, cx);
                            cx.stop_propagation();
                        }
                    },
                ))
                .child(root)
                .children(child)
                .child(
                    gpui_kit::canvas(|_, _, _| (), {
                        let entity = cx.weak_entity();
                        let token = session.trigger.clone();
                        let session_id = session.id;
                        let focus = self.focus.clone();
                        move |_, (), window, cx| {
                            let retired = !token.upgrade().is_some_and(|token| token.get());
                            if retired || !focus.is_focused(window) {
                                let entity = entity.clone();
                                window.defer(cx, move |window, cx| {
                                    let _ = entity.update(cx, |menu, cx| {
                                        if menu
                                            .session
                                            .as_ref()
                                            .is_some_and(|session| session.id == session_id)
                                        {
                                            menu.dismiss(
                                                if retired {
                                                    "trigger_retired"
                                                } else {
                                                    "focus_out"
                                                },
                                                retired,
                                                window,
                                                cx,
                                            );
                                        }
                                    });
                                });
                            }
                        }
                    })
                    .absolute()
                    .size_0(),
                ),
        )
        .with_priority(gpui_kit::base::POPUP_PRIORITY + 10)
        .into_any_element()
    }
}

#[cfg(test)]
#[path = "project_menu_tests.rs"]
mod tests;
