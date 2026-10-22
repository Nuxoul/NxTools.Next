//! Official Notification/NotificationList presentation with bounded business slots.
use super::*;
use gpui_kit::base::{ElementExt as _, StyledExt as _};
use gpui_kit::component::{
    Sizable,
    button::Button,
    label::Label,
    notification::{Notification, NotificationDelivery, NotificationList},
};
use gpui_kit::{
    Anchor, AnyElement, AppContext, Bounds, IntoElement, Render, Subscription, div, prelude::*, px,
};

pub(super) const CARD_WIDTH: f32 = 336.0;
pub(super) const EDGE: f32 = 12.0;
const OFFICIAL_CARD_VERTICAL_PADDING: f32 = 30.0;
const OFFICIAL_PEEK: f32 = 14.0;
const MAX_STACK_HEIGHT: f32 = 400.0;
type HeightMeasured = Rc<dyn Fn(f32, &mut App)>;
struct OfficialSlot;
#[derive(Clone)]
struct Mounted {
    id: SharedString,
    generation: u64,
    position: NotificationPosition,
    entity: Entity<Notification>,
    closing: bool,
    height: f32,
    order: u64,
    focused: bool,
}
struct OfficialPresenter {
    center: Entity<NotificationCenter>,
    list: Entity<NotificationList>,
    slots: Vec<Option<Mounted>>,
    main: bool,
    source: Option<AnyWindowHandle>,
    top_inset: f32,
    measured: Option<HeightMeasured>,
    expanded: bool,
    pointer_inside: bool,
    last_height: Option<f32>,
    shrink_revision: u64,
    next_order: u64,
    _center: Subscription,
}
pub(crate) fn render(
    center: Entity<NotificationCenter>,
    main: bool,
    top_inset: f32,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    if super::screen::attached(cx) {
        return div().into_any_element();
    }
    render_host(center, main, top_inset, None, None, window, cx)
}
pub(super) fn render_screen(
    center: Entity<NotificationCenter>,
    source: AnyWindowHandle,
    measured: HeightMeasured,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    render_host(center, true, 0.0, Some(source), Some(measured), window, cx)
}
fn render_host(
    center: Entity<NotificationCenter>,
    main: bool,
    top_inset: f32,
    source: Option<AnyWindowHandle>,
    measured: Option<HeightMeasured>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let key = SharedString::from(format!(
        "official-notification-presenter-{:?}",
        center.entity_id()
    ));
    let presenter =
        window.use_keyed_state(key, cx, |window, cx: &mut Context<OfficialPresenter>| {
            let subscription = cx.observe(&center, |_, _, cx| cx.notify());
            OfficialPresenter {
                center,
                list: cx.new(|cx| NotificationList::new(window, cx)),
                slots: vec![None; MAX_VISIBLE_NOTIFICATIONS],
                main,
                source,
                top_inset,
                measured,
                expanded: false,
                pointer_inside: false,
                last_height: None,
                shrink_revision: 0,
                next_order: 0,
                _center: subscription,
            }
        });
    div()
        .absolute()
        .inset_0()
        .child(presenter)
        .into_any_element()
}
pub(super) fn anchor(position: NotificationPosition) -> Anchor {
    match position {
        NotificationPosition::TopLeft => Anchor::TopLeft,
        NotificationPosition::TopCenter => Anchor::TopCenter,
        NotificationPosition::TopRight => Anchor::TopRight,
        NotificationPosition::BottomLeft => Anchor::BottomLeft,
        NotificationPosition::BottomCenter => Anchor::BottomCenter,
        NotificationPosition::BottomRight => Anchor::BottomRight,
    }
}
impl OfficialPresenter {
    fn sync(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let position = self.center.read(cx).position();
        if self
            .slots
            .iter()
            .flatten()
            .any(|item| item.position != position)
        {
            self.pointer_inside = false;
            self.update_expansion(cx);
        }
        let window_handle = window.window_handle();
        let mut desired = self
            .center
            .read(cx)
            .items()
            .iter()
            .filter(|item| {
                self.source.is_some()
                    || match item.target {
                        Some(target) => target == window_handle,
                        None => self.main,
                    }
            })
            .cloned()
            .collect::<Vec<_>>();
        desired.reverse(); //Official list appends newest and owns the stack's order/geometry.
        for slot in &mut self.slots {
            if let Some(mounted) = slot.as_mut()
                && !desired.iter().any(|item| item.id == mounted.id)
                && !mounted.closing
            {
                mounted.closing = true;
                mounted
                    .entity
                    .update(cx, |note, cx| note.dismiss(window, cx));
            }
        }
        for item in desired {
            let existing = self
                .slots
                .iter()
                .position(|slot| slot.as_ref().is_some_and(|slot| slot.id == item.id));
            let index = existing.or_else(|| {
                self.slots.iter().position(|slot| {
                    slot.is_none()
                        || slot.as_ref().is_some_and(|mounted| {
                            !self
                                .center
                                .read(cx)
                                .items()
                                .iter()
                                .any(|item| item.id == mounted.id)
                        })
                })
            });
            let Some(index) = index else { continue };
            if let Some(mounted) = self.slots[index].as_mut()
                && mounted.id == item.id
                && mounted.generation == item.generation()
            {
                mounted.position = position;
                if item.exiting && !mounted.closing {
                    mounted.closing = true;
                    mounted
                        .entity
                        .update(cx, |note, cx| note.dismiss(window, cx));
                }
                continue;
            }
            let id = item.id.clone();
            let generation = item.generation();
            let center = self.center.clone();
            let presenter = cx.entity().downgrade();
            let closed_id = id.clone();
            let content = item.clone();
            let default_source = self.source.unwrap_or(window_handle);
            let screen_host = self.source.is_some();
            let top_inset = self.top_inset;
            let note = Notification::new()
                .id1::<OfficialSlot>(index)
                .delivery(NotificationDelivery::InApp)
                .with_type(match item.level {
                    NotificationLevel::Progress | NotificationLevel::Info => {
                        gpui_kit::component::notification::NotificationType::Info
                    }
                    NotificationLevel::Success => {
                        gpui_kit::component::notification::NotificationType::Success
                    }
                    NotificationLevel::Error => {
                        gpui_kit::component::notification::NotificationType::Error
                    }
                })
                .autohide(!item.persistent)
                .duration(item.dwell.unwrap_or(Duration::from_secs(
                    nxtools_core::default_notification_duration() as u64,
                )))
                .content(move |_, window, cx| {
                    let focus = window.use_keyed_state(
                        SharedString::from(format!(
                            "notification-content-focus-{}",
                            content.generation()
                        )),
                        cx,
                        |_, cx| cx.focus_handle(),
                    );
                    let count = center
                        .read(cx)
                        .items()
                        .len()
                        .clamp(1, MAX_VISIBLE_NOTIFICATIONS) as f32;
                    let stack_budget = if screen_host {
                        MAX_STACK_HEIGHT
                    } else {
                        (window.viewport_size().height.as_f32() - top_inset)
                            .clamp(0.0, MAX_STACK_HEIGHT)
                    };
                    let content_budget =
                        ((stack_budget - EDGE * 2.0 - OFFICIAL_PEEK * (count - 1.0)) / count
                            - OFFICIAL_CARD_VERTICAL_PADDING)
                            .floor()
                            .max(0.0);
                    content_element(
                        content.clone(),
                        center.clone(),
                        default_source,
                        presenter.clone(),
                        focus.read(cx).clone(),
                        content_budget,
                    )
                })
                .on_close({
                    let center = self.center.clone();
                    let presenter = cx.entity().downgrade();
                    move |_, cx| {
                        let center = center.clone();
                        let presenter = presenter.clone();
                        let id = closed_id.clone();
                        cx.defer(move |cx| {
                            center.update(cx, |center, cx| {
                                center.completed_close(&id, generation, cx)
                            });
                            let _ = presenter.update(cx, |presenter, cx| {
                                if presenter.slots[index].as_ref().is_some_and(|mounted| {
                                    mounted.generation == generation && mounted.id == id
                                }) {
                                    presenter.slots[index] = None;
                                    presenter.update_expansion(cx);
                                    cx.notify();
                                }
                            });
                        });
                    }
                });
            self.list.update(cx, |list, cx| list.push(note, window, cx));
            let entity = self
                .list
                .read(cx)
                .notifications()
                .last()
                .cloned()
                .expect("official list stores pushed notification");
            self.next_order = self.next_order.wrapping_add(1);
            self.slots[index] = Some(Mounted {
                id,
                generation,
                position,
                entity,
                closing: false,
                height: 104.0,
                order: self.next_order,
                focused: false,
            });
            self.update_expansion(cx);
        }
    }
    fn content_measured(
        &mut self,
        id: &str,
        generation: u64,
        height: f32,
        focused: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(mounted) = self
            .slots
            .iter_mut()
            .flatten()
            .find(|item| item.id.as_ref() == id && item.generation == generation)
        else {
            return;
        };
        let height = (height + OFFICIAL_CARD_VERTICAL_PADDING).ceil();
        if (mounted.height - height).abs() >= 1.0 {
            mounted.height = height;
            cx.notify();
        }
        if mounted.focused != focused {
            mounted.focused = focused;
            self.update_expansion(cx);
        }
    }
    fn desired_height(&self) -> f32 {
        let mut ordered = self.slots.iter().flatten().collect::<Vec<_>>();
        ordered.sort_by_key(|item| item.order);
        let heights = ordered
            .into_iter()
            .map(|item| item.height)
            .collect::<Vec<_>>();
        if heights.is_empty() {
            return 0.0;
        }
        let body = if self.expanded {
            heights.iter().sum::<f32>() + OFFICIAL_PEEK * heights.len().saturating_sub(1) as f32
        } else {
            heights
                .iter()
                .enumerate()
                .map(|(index, height)| {
                    height + OFFICIAL_PEEK * heights.len().saturating_sub(index + 1) as f32
                })
                .fold(0.0, f32::max)
        };
        (body + EDGE * 2.0).ceil().clamp(78.0, 400.0)
    }
    fn report_height(&mut self, cx: &mut Context<Self>) {
        let height = self.desired_height();
        if height == 0.0
            || self
                .last_height
                .is_some_and(|last| (last - height).abs() < 1.0)
        {
            return;
        }
        self.last_height = Some(height);
        if let Some(measured) = self.measured.clone() {
            cx.defer(move |cx| measured(height, cx));
        }
    }
    fn pointer_changed(&mut self, inside: bool, cx: &mut Context<Self>) {
        if inside == self.pointer_inside {
            return;
        }
        self.pointer_inside = inside;
        self.update_expansion(cx);
    }
    fn update_expansion(&mut self, cx: &mut Context<Self>) {
        self.shrink_revision = self.shrink_revision.wrapping_add(1);
        if self.slots.iter().all(Option::is_none) {
            self.pointer_inside = false;
            self.expanded = false;
            self.last_height = None;
            return;
        }
        if self.pointer_inside || self.slots.iter().flatten().any(|item| item.focused) {
            if !self.expanded {
                self.expanded = true;
                cx.notify();
            }
            return;
        }
        if !self.expanded {
            return;
        }
        let revision = self.shrink_revision;
        //Only the HWND waits for the official400ms stack reflow; official hover/clock runs independently.
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(450))
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.shrink_revision == revision && this.expanded {
                    this.expanded = false;
                    cx.notify();
                }
            });
        })
        .detach();
    }
}
impl Render for OfficialPresenter {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync(window, cx);
        self.report_height(cx);
        let width = CARD_WIDTH.min((window.viewport_size().width.as_f32() - EDGE * 2.0).max(0.0));
        let position = self.center.read(cx).position();
        {
            let settings = &mut gpui_kit::component::Theme::global_mut(cx).notification;
            settings.width = px(width);
            settings.placement = anchor(position);
            settings.max_items = MAX_VISIBLE_NOTIFICATIONS;
            settings.margins.top = px(self.top_inset + EDGE);
            settings.margins.bottom = px(EDGE);
            settings.margins.left = px(EDGE);
            settings.margins.right = px(EDGE);
        }
        let presenter = cx.entity().downgrade();
        let native_height = self.desired_height();
        let position = self.center.read(cx).position();
        let viewport = window.viewport_size();
        let top = if position.is_top() {
            self.top_inset + EDGE
        } else {
            (viewport.height.as_f32() - native_height + EDGE).max(0.0)
        };
        let left = match position {
            NotificationPosition::TopLeft | NotificationPosition::BottomLeft => EDGE,
            NotificationPosition::TopCenter | NotificationPosition::BottomCenter => {
                (viewport.width.as_f32() - width) / 2.0
            }
            _ => viewport.width.as_f32() - width - EDGE,
        };
        let hit = Bounds::new(
            gpui_kit::point(px(left), px(top)),
            gpui_kit::size(px(width), px((native_height - EDGE * 2.0).max(0.0))),
        );
        //Observer only sizes the native host. Official ToastStack owns hover and expiry.
        let observer = gpui_kit::canvas(
            |_, _, _| {},
            move |_, _, window, _cx| {
                window.on_mouse_event(move |event: &gpui_kit::MouseMoveEvent, phase, _, cx| {
                    if phase == gpui_kit::DispatchPhase::Bubble {
                        let presenter = presenter.clone();
                        let inside = hit.contains(&event.position);
                        cx.defer(move |cx| {
                            let _ = presenter
                                .update(cx, |presenter, cx| presenter.pointer_changed(inside, cx));
                        });
                    }
                });
            },
        )
        .absolute()
        .size_full();
        div()
            .debug_selector(|| "notification-stack".into())
            .size_full()
            .relative()
            .child(observer)
            .child(self.list.clone())
    }
}
fn content_element(
    item: AppNotification,
    center: Entity<NotificationCenter>,
    default_source: AnyWindowHandle,
    presenter: WeakEntity<OfficialPresenter>,
    focus: gpui_kit::FocusHandle,
    content_budget: f32,
) -> AnyElement {
    let id = item.id.clone();
    let generation = item.generation();
    let selector = format!("notification-{}", item.id);
    let measured_id = id.clone();
    let copy_selector = format!("notification-content-{id}");
    let mut body = div()
        .debug_selector(|| "notification-natural-text-body".into())
        .flex_shrink_0()
        .w_full()
        .min_w_0()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .w_full()
                .flex_shrink_0()
                .child(Label::new(item.title).font_semibold().whitespace_normal()),
        )
        .child(
            div()
                .w_full()
                .flex_shrink_0()
                .child(Label::new(item.message).whitespace_normal()),
        );
    for detail in item.details.into_iter().take(8) {
        body = body
            .child(Label::new(format!("{}：{}", detail.label, detail.value)).whitespace_normal());
    }
    let mut actions = div().flex().flex_wrap().flex_shrink_0().gap_1();
    let mut has_actions = false;
    for (index, action) in item
        .action
        .into_iter()
        .chain(item.additional_actions)
        .take(2)
        .enumerate()
    {
        has_actions = true;
        let center = center.clone();
        let action_id = id.clone();
        let source = item.target.unwrap_or(default_source);
        let selector = if index == 0 {
            format!("notification-action-{id}")
        } else {
            format!("notification-action-{id}-{index}")
        };
        actions = actions.child(
            Button::new(SharedString::from(selector.clone()))
                .debug_selector(move || selector.clone())
                .label(action.label)
                .outline()
                .small()
                .on_click(move |_, _, cx| {
                    cx.stop_propagation();
                    let center = center.clone();
                    let id = action_id.clone();
                    cx.defer(move |cx| {
                        let action = center.update(cx, |center, cx| {
                            center.invoke_action(&id, generation, index, cx)
                        });
                        if let Some(action) = action {
                            let _ = source.update(cx, |_, window, cx| action.invoke(window, cx));
                        }
                    });
                }),
        );
    }
    div()
        .debug_selector(move || selector.clone())
        .track_focus(&focus)
        .w_full()
        .min_w_0()
        .max_h(px(content_budget))
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .id(SharedString::from(format!("notification-copy-{id}")))
                .debug_selector(move || copy_selector.clone())
                .min_h_0()
                .max_h(px(content_budget))
                .overflow_y_scroll()
                .child(body),
        )
        .when(has_actions, |content| content.child(actions))
        .on_prepaint(move |bounds, window, cx| {
            let presenter = presenter.clone();
            let id = measured_id.clone();
            let height = bounds.size.height.as_f32();
            let focused = focus.contains_focused(window, cx);
            cx.defer(move |cx| {
                let _ = presenter.update(cx, |presenter, cx| {
                    presenter.content_measured(&id, generation, height, focused, cx)
                });
            });
        })
        .into_any_element()
}
