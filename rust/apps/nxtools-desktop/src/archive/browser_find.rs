//! Archive-wide Find: one snapshot-bound projection over the loaded browser catalog.
use super::{ArchiveBrowser, BrowserStatus, Command};
use crate::archive::model::{self, RowId};
use gpui_kit::base::TestSupportExt as _;
use gpui_kit::component::{
    Disableable, Sizable, Size, WindowExt,
    button::Button,
    checkbox::Checkbox,
    dialog::DialogFooter,
    form::{Field, Form},
    input::{Input, InputEvent, InputGroup, InputState},
    list::ListItem,
};
use gpui_kit::{
    AnyElement, AppContext, Context, Entity, Focusable, IntoElement, KeyDownEvent, ParentElement,
    Render, Role, ScrollStrategy, Subscription, UniformListScrollHandle, WeakEntity, Window, div,
    prelude::*, px, uniform_list,
};
use nxtools_archive::member_query::MemberQuery;
use nxtools_protocol::archive_job::ArchiveJobModifyAction;
use std::{ops::Range, time::Instant};
use uuid::Uuid;

#[derive(Clone)]
struct FindResult {
    id: RowId,
    path: String,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct FindWork {
    pub catalog_visits: u64,
    pub folder_visits: u64,
    pub query_checks: u64,
    pub matches: u64,
}

#[derive(Clone, Copy)]
enum Action {
    Navigate,
    Extract,
    Delete,
}

pub(super) struct FindView {
    browser: WeakEntity<ArchiveBrowser>,
    browser_generation: u64,
    snapshot: Uuid,
    find_epoch: u64,
    query_generation: u64,
    correlation_id: Uuid,
    input: Entity<InputState>,
    wildcard: bool,
    invalid: bool,
    invalidated: bool,
    results: Vec<FindResult>,
    selected: Option<usize>,
    scroll: UniformListScrollHandle,
    work: FindWork,
    #[cfg(test)]
    pub(super) last_query_work: FindWork,
    #[cfg(test)]
    pub(super) rendered_rows: std::cell::Cell<u64>,
    _input_subscription: Subscription,
}

impl ArchiveBrowser {
    pub(super) fn start_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.retired
            || !matches!(self.status, BrowserStatus::Ready)
            || window.has_active_dialog(cx)
        {
            return;
        }
        let Some(snapshot) = self.model.as_ref().map(|model| model.snapshot) else {
            return;
        };
        self.invalidate_find(cx);
        let epoch = self.find_epoch;
        let generation = self.generation;
        let browser = cx.entity().downgrade();
        let find =
            cx.new(|cx| FindView::new(browser.clone(), generation, snapshot, epoch, window, cx));
        self.find = Some(find.clone());
        let close_browser = browser.clone();
        let source = window.window_handle();
        window.open_dialog(cx, move |dialog, _, cx| {
            let browser = close_browser.clone();
            let confirm_find = find.downgrade();
            let valid = {
                let view = find.read(cx);
                !view.invalid && !view.invalidated && view.selected.is_some()
            };
            let mut footer = DialogFooter::new();
            for (id, label, action) in [
                ("archive-find-navigate", "定位项目", Action::Navigate),
                ("archive-find-extract", "提取结果", Action::Extract),
                ("archive-find-delete", "删除结果", Action::Delete),
            ] {
                let find = find.downgrade();
                footer = footer.child(
                    Button::new(id)
                        .label(label)
                        .outline()
                        .with_size(Size::Medium)
                        .disabled(!valid)
                        .on_click(move |_, window, cx| {
                            let _ = find.update(cx, |view, cx| view.action(action, window, cx));
                        }),
                );
            }
            let footer = footer.child(
                Button::new("archive-find-close")
                    .label("关闭查找")
                    .outline()
                    .with_size(Size::Medium)
                    .on_click(|_, window, cx| window.close_dialog(cx)),
            );
            dialog
                .width(px(720.))
                .title("查找整个压缩包")
                .child(find.clone())
                .footer(footer)
                .on_ok(move |_, _, cx| {
                    let find = confirm_find.clone();
                    cx.defer(move |cx| {
                        let _ = source.update(cx, |_, window, cx| {
                            let _ = find
                                .update(cx, |view, cx| view.action(Action::Navigate, window, cx));
                        });
                    });
                    false
                })
                .on_close(move |_, window, cx| {
                    let browser = browser.clone();
                    let source = window.window_handle();
                    cx.defer(move |cx| {
                        let _ = source.update(cx, |_, window, cx| {
                            let _ = browser.update(cx, |view, cx| {
                                if view.find_epoch == epoch && view.find.is_some() {
                                    view.invalidate_find(cx);
                                    if !view.retired && matches!(view.status, BrowserStatus::Ready)
                                    {
                                        view.list_focus.focus(window, cx);
                                    }
                                }
                            });
                        });
                    });
                })
        });
        // Initial catalog access happens after the Browser's current lease is released.
        let first = self.find.as_ref().map(Entity::downgrade);
        if let Some(first) = first {
            let source = window.window_handle();
            cx.defer(move |cx| {
                let _ = source.update(cx, |_, window, cx| {
                    let _ = first.update(cx, |find, cx| {
                        find.search(cx);
                        if !find.invalidated {
                            find.input.focus_handle(cx).focus(window, cx);
                        }
                    });
                });
            });
        }
    }

    pub(super) fn invalidate_find(&mut self, cx: &mut Context<Self>) {
        self.find_epoch = self.find_epoch.wrapping_add(1);
        if let Some(find) = self.find.take() {
            let find = find.downgrade();
            cx.defer(move |cx| {
                let _ = find.update(cx, |view, cx| {
                    view.invalidated = true;
                    view.results.clear();
                    view.selected = None;
                    cx.notify();
                });
            });
        }
    }

    fn find_action(
        &mut self,
        id: RowId,
        action: Action,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(model) = self.model.as_mut() else {
            return;
        };
        let path = match &id {
            RowId::Entry(snapshot, ordinal) if *snapshot == model.snapshot => model
                .entries
                .iter()
                .find(|entry| {
                    entry.ordinal == *ordinal
                        && entry.kind != "Directory"
                        && (matches!(action, Action::Navigate) || !model::entry_warning(entry))
                })
                .map(|entry| {
                    model::member_path(&entry.name)
                        .trim_end_matches('/')
                        .to_owned()
                }),
            RowId::Folder(snapshot, path)
                if *snapshot == model.snapshot
                    && model.folders().iter().any(|folder| folder == path)
                    && (matches!(action, Action::Navigate)
                        || !model
                            .entries
                            .iter()
                            .rev()
                            .find(|entry| {
                                entry.kind == "Directory"
                                    && model::member_path(&entry.name).trim_end_matches('/') == path
                            })
                            .is_some_and(model::entry_warning)) =>
            {
                Some(path.clone())
            }
            _ => None,
        };
        let Some(path) = path.filter(|path| {
            !path.is_empty()
                && !path.starts_with('/')
                && path.split('/').all(|part| !matches!(part, "" | "." | ".."))
        }) else {
            return;
        };
        window.close_dialog(cx);
        let parent = path
            .rsplit_once('/')
            .map_or("", |(parent, _)| parent)
            .to_owned();
        // Reproject even if the parent is already open: the current-folder filter may hide this row.
        model.enter(&parent);
        if !model
            .visible()
            .iter()
            .any(|row| row.id == id && (matches!(action, Action::Navigate) || !row.warning))
        {
            return;
        }
        model.select(id.clone(), false, false);
        self.sync_filter(window, cx);
        self.projection_changed(window, cx);
        self.list_focus.focus(window, cx);
        let epoch = self.projection_epoch;
        let scroll = self.scroll.clone();
        let weak = cx.entity().downgrade();
        window.defer(cx, move |_, cx| {
            if let Ok(Some(index)) = weak.read_with(cx, |view, _| {
                (view.projection_epoch == epoch && matches!(view.status, BrowserStatus::Ready))
                    .then(|| {
                        view.model
                            .as_ref()
                            .and_then(|model| model.visible().iter().position(|row| row.id == id))
                    })
                    .flatten()
            }) {
                scroll.scroll_to_item(index, ScrollStrategy::Nearest);
            }
        });
        match action {
            Action::Navigate => {}
            Action::Extract => self.command(Command::Extract(true), window, cx),
            Action::Delete => {
                self.command(Command::Edit(ArchiveJobModifyAction::Delete), window, cx)
            }
        }
    }
}

impl FindView {
    fn new(
        browser: WeakEntity<ArchiveBrowser>,
        browser_generation: u64,
        snapshot: Uuid,
        find_epoch: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("输入名称或内部路径"));
        let subscription = cx.subscribe_in(&input, window, |this, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                this.search(cx);
            }
        });
        let correlation_id = Uuid::new_v4();
        tracing::info!(event="archive.find.accepted", %correlation_id, "archive-wide query opened");
        Self {
            browser,
            browser_generation,
            snapshot,
            find_epoch,
            query_generation: 0,
            correlation_id,
            input,
            wildcard: false,
            invalid: false,
            invalidated: false,
            results: Vec::new(),
            selected: None,
            scroll: UniformListScrollHandle::new(),
            work: FindWork::default(),
            #[cfg(test)]
            last_query_work: FindWork::default(),
            #[cfg(test)]
            rendered_rows: std::cell::Cell::new(0),
            _input_subscription: subscription,
        }
    }

    fn live(&self, browser: &ArchiveBrowser) -> bool {
        !self.invalidated
            && !browser.retired
            && browser.generation == self.browser_generation
            && browser.find_epoch == self.find_epoch
            && matches!(browser.status, BrowserStatus::Ready)
            && browser
                .model
                .as_ref()
                .is_some_and(|model| model.snapshot == self.snapshot)
    }

    fn search(&mut self, cx: &mut Context<Self>) {
        self.query_generation = self.query_generation.wrapping_add(1);
        self.results.clear();
        self.selected = None;
        self.invalid = false;
        self.work = FindWork::default();
        let started = Instant::now();
        let query = self.input.read(cx).value().to_string();
        let matcher = MemberQuery::new(&query, self.wildcard);
        let browser_handle = self.browser.clone();
        let status = if matcher.is_err() {
            self.invalid = true;
            "invalid"
        } else if let Ok(Some(())) = browser_handle.read_with(cx, |browser, _| {
            if !self.live(browser) {
                return None;
            }
            let model = browser.model.as_ref()?;
            let matcher = matcher.as_ref().ok()?;
            for folder in model.folders() {
                self.work.folder_visits += 1;
                self.work.query_checks += 1;
                if matcher.matches(folder) {
                    self.results.push(FindResult {
                        id: RowId::Folder(self.snapshot, folder.clone()),
                        path: folder.clone(),
                    });
                }
            }
            for entry in &model.entries {
                self.work.catalog_visits += 1;
                if entry.kind == "Directory" {
                    continue;
                }
                self.work.query_checks += 1;
                if matcher.matches(&entry.name) {
                    self.results.push(FindResult {
                        id: RowId::Entry(self.snapshot, entry.ordinal),
                        path: model::member_path(&entry.name).into_owned(),
                    });
                }
            }
            Some(())
        }) {
            self.work.matches = self.results.len() as u64;
            if self.results.is_empty() {
                "empty"
            } else {
                "matched"
            }
        } else {
            self.invalidated = true;
            "stale"
        };
        #[cfg(test)]
        {
            self.last_query_work = self.work;
        }
        tracing::info!(event="archive.find.query_finished", correlation_id=%self.correlation_id,
            status, catalog_visits=self.work.catalog_visits, folder_visits=self.work.folder_visits,
            query_checks=self.work.query_checks, matches=self.work.matches,
            elapsed_ms=started.elapsed().as_millis() as u64, "archive-wide query completed");
        cx.notify();
    }

    fn action(&mut self, action: Action, window: &mut Window, cx: &mut Context<Self>) {
        let Some(result) = self
            .selected
            .and_then(|index| self.results.get(index))
            .cloned()
        else {
            return;
        };
        let query_generation = self.query_generation;
        let find_epoch = self.find_epoch;
        let snapshot = self.snapshot;
        let browser_generation = self.browser_generation;
        let find = cx.entity().downgrade();
        let browser = self.browser.clone();
        let source = window.window_handle();
        cx.defer(move |cx| {
            let current = find
                .read_with(cx, |view, _| {
                    view.query_generation == query_generation
                        && view.selected.is_some_and(|index| {
                            view.results
                                .get(index)
                                .is_some_and(|row| row.id == result.id)
                        })
                        && !view.invalidated
                })
                .unwrap_or(false);
            if !current {
                return;
            }
            let _ = source.update(cx, |_, window, cx| {
                let _ = browser.update(cx, |view, cx| {
                    if view.generation != browser_generation
                        || view.find_epoch != find_epoch
                        || !matches!(view.status, BrowserStatus::Ready)
                        || view.retired
                        || view
                            .model
                            .as_ref()
                            .is_none_or(|model| model.snapshot != snapshot)
                    {
                        return;
                    }
                    view.find_action(result.id, action, window, cx);
                });
            });
        });
    }

    fn choose(&mut self, index: usize, double: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.invalidated || self.invalid || index >= self.results.len() {
            return;
        }
        self.selected = Some(index);
        self.scroll.scroll_to_item(index, ScrollStrategy::Nearest);
        cx.notify();
        if double {
            self.action(Action::Navigate, window, cx);
        }
    }

    fn keyboard(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.modifiers.modified() || event.is_held {
            return;
        }
        let next = match event.keystroke.key.to_ascii_lowercase().as_str() {
            "up" => Some(self.selected.unwrap_or(0).saturating_sub(1)),
            "down" => Some(self.selected.map_or(0, |ix| ix.saturating_add(1))),
            "home" => Some(0),
            "end" => Some(self.results.len().saturating_sub(1)),
            "enter" => {
                self.action(Action::Navigate, window, cx);
                cx.stop_propagation();
                return;
            }
            _ => None,
        };
        if let Some(index) = next {
            if !self.results.is_empty() {
                self.choose(index.min(self.results.len() - 1), false, window, cx);
            }
            cx.stop_propagation();
        }
    }

    fn render_row(&self, ix: usize, cx: &mut Context<Self>) -> AnyElement {
        #[cfg(test)]
        self.rendered_rows.set(self.rendered_rows.get() + 1);
        let row = &self.results[ix];
        let selected = self.selected == Some(ix);
        let query_generation = self.query_generation;
        let id = row.id.clone();
        ListItem::new(gpui_kit::SharedString::from(format!(
            "archive-find-result-{query_generation}-{ix}"
        )))
        .debug_selector(move || format!("archive-find-result-{ix}"))
        .role(Role::ListBoxOption)
        .aria_selected(selected)
        .accessibility_label(row.path.clone())
        .selected(selected)
        .h(px(32.))
        .w_full()
        .px_2()
        .overflow_hidden()
        .child(div().min_w_0().text_ellipsis().child(row.path.clone()))
        .on_click(
            cx.listener(move |this, event: &gpui_kit::ClickEvent, window, cx| {
                if this.query_generation != query_generation
                    || this.results.get(ix).is_none_or(|row| row.id != id)
                {
                    return;
                }
                this.choose(ix, event.click_count() == 2, window, cx);
            }),
        )
        .into_any_element()
    }
}

impl Render for FindView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let height = (f32::from(window.viewport_size().height) - 250.).clamp(32., 256.);
        let count = self.results.len();
        let scroll = self.scroll.clone();
        let wildcard = self.wildcard;
        let invalid = self.invalid;
        let empty = if self.invalidated {
            "查找已失效，请重新打开"
        } else if self.invalid {
            "查询无效，请修改通配符或缩短查询"
        } else if count == 0 && self.input.read(cx).value().is_empty() {
            "压缩包没有项目"
        } else {
            "没有匹配的项目"
        };
        div()
            .w_full()
            .flex()
            .flex_col()
            .gap_2()
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                this.keyboard(event, window, cx)
            }))
            .child(
                Form::vertical().child(
                    Field::new().label("名称或内部路径").child(
                        InputGroup::new("archive-find-query-group")
                            .input(Input::new(&self.input).aria_label("名称或内部路径")),
                    ),
                ),
            )
            .child(
                Checkbox::new("archive-find-wildcard")
                    .label("通配符")
                    .checked(wildcard)
                    .on_click(cx.listener(|this, checked, _, cx| {
                        if this.wildcard != *checked {
                            this.wildcard = *checked;
                            this.search(cx);
                        }
                    })),
            )
            .child(if count == 0 || self.invalidated || self.invalid {
                div()
                    .id(if self.invalid {
                        "archive-find-invalid"
                    } else {
                        "archive-find-empty"
                    })
                    .debug_selector(move || {
                        if invalid {
                            "archive-find-invalid".into()
                        } else {
                            "archive-find-empty".into()
                        }
                    })
                    .test_support()
                    .h(px(height))
                    .child(empty)
                    .into_any_element()
            } else {
                uniform_list(
                    "archive-find-results",
                    count,
                    cx.processor(|this, range: Range<usize>, _, cx| {
                        range.map(|ix| this.render_row(ix, cx)).collect::<Vec<_>>()
                    }),
                )
                .track_scroll(&scroll)
                .h(px(height))
                .w_full()
                .into_any_element()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui_kit::test]
    fn opt07_find_deferred_actions_retire_on_query_snapshot_and_browser_lifecycle(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        for change in [
            "query", "refresh", "encoding", "snapshot", "tab", "retire", "failure",
        ] {
            let (browser, mut surface) = super::super::tests::mount(cx, 3);
            surface.update(|window, cx| browser.update(cx, |view, cx| view.start_find(window, cx)));
            surface.run_until_parked();
            let find = browser.read_with(&surface, |view, _| {
                view.find.clone().expect("original Find")
            });
            surface.update(|window, cx| {
                find.update(cx, |view, cx| {
                    view.choose(1, false, window, cx);
                    view.action(Action::Navigate, window, cx);
                });
                if change == "query" {
                    find.update(cx, |view, cx| {
                        view.wildcard = true;
                        view.search(cx);
                    });
                } else {
                    browser.update(cx, |view, cx| match change {
                        "refresh" => view.command(Command::Refresh, window, cx),
                        "encoding" => view.command(Command::Encoding(936), window, cx),
                        "snapshot" => {
                            let entries = view.model.as_ref().expect("catalog").entries.clone();
                            view.install(Uuid::new_v4(), entries, "Zip".into(), window, cx);
                        }
                        "tab" => view.suspend_tab(cx),
                        "retire" => view.retire(cx),
                        _ => view.set_failure("来源已变化".into(), window, cx),
                    });
                }
            });
            surface.run_until_parked();
            browser.read_with(&surface, |view, _| {
                assert!(
                    view.model.as_ref().expect("catalog").selected.is_empty(),
                    "{change}: stale action must not install a row"
                );
                assert!(!view.open_pending && view.open_dispatches.is_empty());
            });
        }
    }

    #[gpui_kit::test]
    fn opt07_find_large_catalog_records_actual_visits_and_virtual_rows(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        let (browser, mut surface) = super::super::tests::mount(cx, 10_000);
        for trial in 0..3 {
            let started = Instant::now();
            surface.update(|window, cx| browser.update(cx, |view, cx| view.start_find(window, cx)));
            surface.run_until_parked();
            surface.update(|window, cx| {
                window.draw(cx).clear(cx);
            });
            let find = browser.read_with(&surface, |view, _| view.find.clone().expect("Find"));
            find.read_with(&surface,|view,_| {
                assert_eq!(view.last_query_work.catalog_visits,10_000);
                assert_eq!(view.last_query_work.query_checks,10_000);
                assert_eq!(view.results.len(),10_000);
                let rows=view.rendered_rows.get();
                assert!(rows>0 && rows<200,"virtual result rows: {rows}");
                println!("OPT07 Find trial={trial} catalog_visits={} folder_visits={} query_checks={} matches={} rendered_rows={rows} open_draw_us={}",view.work.catalog_visits,view.work.folder_visits,view.work.query_checks,view.work.matches,started.elapsed().as_micros());
            });
            surface.update(|window, cx| window.close_dialog(cx));
            surface.run_until_parked();
        }
    }

    #[gpui_kit::test]
    fn opt07_find_old_pointer_release_cannot_select_replacement_query_row(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        let (browser, mut surface) = super::super::tests::mount(cx, 3);
        surface.update(|window, cx| browser.update(cx, |view, cx| view.start_find(window, cx)));
        surface.run_until_parked();
        let find = browser.read_with(&surface, |view, _| view.find.clone().expect("Find"));
        surface.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        let position = surface
            .debug_bounds("archive-find-result-0")
            .expect("original Find row")
            .center();
        surface.simulate_event(gpui_kit::MouseDownEvent {
            position,
            button: gpui_kit::MouseButton::Left,
            click_count: 1,
            ..Default::default()
        });
        surface.update(|window, cx| {
            find.update(cx, |view, cx| {
                let replacement = view.results[1].path.clone();
                view.input
                    .update(cx, |input, cx| input.set_value(replacement, window, cx));
                view.search(cx);
            })
        });
        assert_eq!(
            find.read_with(&surface, |view, _| view.results.len()),
            1,
            "replacement query applied before old release"
        );
        surface.simulate_event(gpui_kit::MouseUpEvent {
            position,
            button: gpui_kit::MouseButton::Left,
            click_count: 1,
            modifiers: Default::default(),
        });
        surface.run_until_parked();
        assert!(
            find.read_with(&surface, |view, _| view.selected.is_none()),
            "old query callback cannot select a new row at its former index"
        );
        assert!(browser.read_with(&surface, |view, _| {
            view.model.as_ref().expect("catalog").selected.is_empty()
        }));
    }
}
