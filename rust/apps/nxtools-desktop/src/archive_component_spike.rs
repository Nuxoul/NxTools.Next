//! Disconnected locked Kit 0.7.1 capability probe, never a production browser.
use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::rc::Rc;

use gpui_kit::component::Sizable;
use gpui_kit::component::list::ListItem;
use gpui_kit::component::table::{
    Column, DataTable, TableDelegate, TableEvent, TableSelection, TableState,
};
use gpui_kit::component::tree::{Tree, TreeItem, TreeState};
use gpui_kit::{
    App, AppContext, Context, Entity, Focusable, IntoElement, KeyDownEvent, KeyUpEvent, Keystroke,
    Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, Render, Subscription, TestAppContext,
    VisualTestContext, Window, div, prelude::*, px, size,
};

type ExternalSelection = Rc<RefCell<BTreeSet<usize>>>;
struct Rows {
    ids: Vec<usize>,
    external: Option<ExternalSelection>,
    anchor: Rc<Cell<Option<usize>>>,
}
impl TableDelegate for Rows {
    fn columns_count(&self, _: &App) -> usize {
        1
    }
    fn rows_count(&self, _: &App) -> usize {
        self.ids.len()
    }
    fn column(&self, _: usize, _: &App) -> Column {
        Column::new("name", "名称").width(px(250.0)).sortable()
    }
    fn render_tr(
        &mut self,
        row: usize,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> gpui_kit::Stateful<gpui_kit::Div> {
        let id = self.ids[row];
        let selected = self
            .external
            .as_ref()
            .is_some_and(|ids| ids.borrow().contains(&id));
        let table = cx.entity().downgrade();
        let anchor = self.anchor.clone();
        let ids = self.ids.clone();
        div()
            .id(("probe-row", id))
            .debug_selector(move || format!("spike-row-{id}"))
            .aria_selected(selected)
            .when_some(self.external.clone(), |row, external| {
                row.on_click(move |event, _, cx| {
                    let mut selected = external.borrow_mut();
                    if event.modifiers().shift {
                        let start = anchor.get().unwrap_or(row_index(&ids, id));
                        let end = row_index(&ids, id);
                        selected.clear();
                        selected.extend(ids[start.min(end)..=start.max(end)].iter().copied());
                    } else {
                        if !event.modifiers().control {
                            selected.clear();
                        }
                        if !selected.insert(id) {
                            selected.remove(&id);
                        }
                        anchor.set(Some(row_index(&ids, id)));
                    }
                    drop(selected);
                    let table = table.clone();
                    cx.defer(move |cx| {
                        let _ = table.update(cx, |_, cx| cx.notify());
                    });
                })
            })
    }
    fn render_td(
        &mut self,
        row: usize,
        _: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        div().child(format!("entry-{}", self.ids[row]))
    }
}
fn row_index(ids: &[usize], id: usize) -> usize {
    ids.iter()
        .position(|candidate| *candidate == id)
        .expect("fixture immutable id")
}
struct Probe {
    table: Entity<TableState<Rows>>,
    tree: Entity<TreeState>,
    folder: String,
    sync_count: usize,
    events: Rc<RefCell<Vec<&'static str>>>,
    _subscriptions: Vec<Subscription>,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .child(div().w(px(180.0)).h_full().child(Tree::new(
                &self.tree,
                |ix, entry, _, _, _| {
                    ListItem::new(("tree-item", ix))
                        .accessibility_label(entry.item().label.clone())
                        .child(entry.item().label.clone())
                },
            )))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .child(DataTable::new(&self.table).small()),
            )
    }
}
fn tree_items() -> Vec<TreeItem> {
    vec![
        TreeItem::new("root", "archive.zip")
            .expanded(true)
            .children([
                TreeItem::new("folder-a", "Folder A"),
                TreeItem::new("folder-b", "Folder B"),
            ]),
    ]
}
fn apply_folder(owner: gpui_kit::WeakEntity<Probe>, id: String, cx: &mut App) {
    cx.defer(move |cx| {
        let _ = owner.update(cx, |probe, cx| {
            if probe.folder == id {
                return;
            }
            probe.folder = id.clone();
            probe.sync_count += 1;
            let table = probe.table.downgrade();
            let tree = probe.tree.downgrade();
            cx.defer(move |cx| {
                let item = TreeItem::new(id.clone(), id.clone());
                let _ = tree.update(cx, |state, cx| {
                    if state
                        .selected_item()
                        .is_none_or(|selected| selected.id.as_ref() != id)
                    {
                        state.set_selected_item(Some(&item), cx);
                    }
                });
                let _ = table.update(cx, |state, cx| {
                    state.delegate_mut().ids = if id == "folder-a" {
                        vec![10, 11]
                    } else if id == "folder-b" {
                        vec![20, 21]
                    } else {
                        vec![0, 1, 2, 3, 4, 5]
                    };
                    state.clear_selection(cx);
                    state.refresh(cx);
                });
            });
            cx.notify();
        });
    });
}
fn mount(
    cx: &mut TestAppContext,
    external: Option<ExternalSelection>,
    sync: bool,
) -> (Entity<Probe>, VisualTestContext) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let retained = Rc::new(RefCell::new(None));
    let slot = retained.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| {
        let table = cx.new(|cx| {
            TableState::new(
                Rows {
                    ids: vec![0, 1, 2, 3, 4, 5],
                    external: external.clone(),
                    anchor: Rc::new(Cell::new(None)),
                },
                window,
                cx,
            )
            .row_selectable(external.is_none())
            .loop_selection(false)
        });
        let tree = cx.new(|cx| TreeState::new(cx).items(tree_items()));
        let view = cx.new(|cx| {
            let events = Rc::new(RefCell::new(Vec::new()));
            let observed = events.clone();
            let table_events = cx.subscribe(&table, move |_, _, event: &TableEvent, _| {
                observed.borrow_mut().push(match event {
                    TableEvent::SelectRow(_) => "select",
                    TableEvent::DoubleClickedRow(_) => "double",
                    TableEvent::RightClickedRow(_) => "right",
                    _ => "other",
                });
            });
            let mut subscriptions = vec![table_events];
            if sync {
                subscriptions.push(cx.observe(&tree, |_, tree, cx| {
                    let id = tree
                        .read(cx)
                        .selected_item()
                        .map(|item| item.id.to_string());
                    if let Some(id) = id {
                        apply_folder(cx.entity().downgrade(), id, cx);
                    }
                }));
                subscriptions.push(cx.subscribe(&table, |_, table, event: &TableEvent, cx| {
                    if let TableEvent::DoubleClickedRow(row) = event {
                        let id = table.read(cx).delegate().ids[*row];
                        if id < 2 {
                            apply_folder(
                                cx.entity().downgrade(),
                                if id == 0 { "folder-a" } else { "folder-b" }.into(),
                                cx,
                            );
                        }
                    }
                }));
            }
            Probe {
                table: table.clone(),
                tree: tree.clone(),
                folder: "root".into(),
                sync_count: 0,
                events,
                _subscriptions: subscriptions,
            }
        });
        *slot.borrow_mut() = Some(view.clone());
        gpui_kit::base::Root::new(view, window, cx)
    });
    let handle = surface.update(|window, _| window.window_handle());
    let mut surface = VisualTestContext::from_window(handle, cx);
    surface.simulate_resize(size(px(640.0), px(340.0)));
    surface.update(|window, cx| {
        window.activate_window();
        window.draw(cx).clear(cx);
    });
    let view = retained.borrow().clone().expect("fixture owner");
    (view, surface)
}
fn draw(surface: &mut VisualTestContext) {
    surface.run_until_parked();
    surface.update(|window, cx| window.draw(cx).clear(cx));
}
fn click(surface: &mut VisualTestContext, id: usize, modifiers: Modifiers, count: usize) {
    draw(surface);
    let position = surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .find(|node| node.path().last() == Some(&gpui_kit::ElementId::from(("probe-row", id))))
            .expect("actual current-frame row hit bounds")
            .bounds()
            .center()
    });
    surface.simulate_event(MouseDownEvent {
        position,
        button: MouseButton::Left,
        modifiers,
        click_count: count,
        ..Default::default()
    });
    surface.simulate_event(MouseUpEvent {
        position,
        button: MouseButton::Left,
        modifiers,
        click_count: count,
    });
    draw(surface);
}
fn key(surface: &mut VisualTestContext, stroke: &'static str) {
    surface.simulate_event(KeyDownEvent {
        keystroke: Keystroke::parse(stroke).expect("fixture key"),
        is_held: false,
        prefer_character_input: false,
    });
    surface.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse(stroke).expect("fixture release"),
    });
    draw(surface);
}
fn selected(surface: &mut VisualTestContext) -> usize {
    surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .iter()
            .filter(|node| {
                node.role() == Some(gpui_kit::Role::Row) && node.selected() == Some(true)
            })
            .count()
    })
}
fn table(probe: &Entity<Probe>, surface: &VisualTestContext) -> Entity<TableState<Rows>> {
    probe.read_with(surface, |probe, _| probe.table.clone())
}

#[gpui_kit::test]
fn stock_datatable_modifier_and_keyboard_probe(cx: &mut TestAppContext) {
    let (probe, mut surface) = mount(cx, None, false);
    let state = table(&probe, &surface);
    click(&mut surface, 0, Modifiers::default(), 1);
    click(
        &mut surface,
        1,
        Modifiers {
            control: true,
            ..Default::default()
        },
        1,
    );
    assert_eq!(
        state.read_with(&surface, |state, _| state.selection()),
        TableSelection::Row(1)
    );
    assert_eq!(
        selected(&mut surface),
        1,
        "Ctrl click replaces rather than adds"
    );
    click(
        &mut surface,
        3,
        Modifiers {
            shift: true,
            ..Default::default()
        },
        1,
    );
    assert_eq!(
        state.read_with(&surface, |state, _| state.selected_row()),
        Some(3)
    );
    assert_eq!(
        selected(&mut surface),
        1,
        "Shift click does not form a range"
    );
    surface.update(|window, cx| state.read(cx).focus_handle(cx).focus(window, cx));
    key(&mut surface, "shift-down");
    eprintln!(
        "SPIKE shift-down selection={:?} selected_rows={}",
        state.read_with(&surface, |state, _| state.selection()),
        selected(&mut surface)
    );
    assert_eq!(selected(&mut surface), 1, "no keyboard range");
    key(&mut surface, "ctrl-a");
    assert_eq!(selected(&mut surface), 1, "Ctrl+A does not select all rows");
    key(&mut surface, "down");
    assert_eq!(
        state.read_with(&surface, |state, _| state.selected_row()),
        Some(4)
    );
    key(&mut surface, "up");
    assert_eq!(
        state.read_with(&surface, |state, _| state.selected_row()),
        Some(3)
    );
    let position = surface.debug_bounds("spike-row-1").expect("row1").center();
    surface.simulate_event(MouseDownEvent {
        position,
        button: MouseButton::Right,
        ..Default::default()
    });
    draw(&mut surface);
    assert_eq!(
        state.read_with(&surface, |state, _| state.right_clicked_row()),
        Some(1)
    );
    assert_eq!(
        state.read_with(&surface, |state, _| state.selected_row()),
        Some(3),
        "right-click tracks context separately"
    );
    click(&mut surface, 3, Modifiers::default(), 2);
    assert!(probe.read_with(&surface, |probe, _| {
        probe.events.borrow().contains(&"double")
    }));
    eprintln!(
        "SPIKE stock: Ctrl=one Shift=one Ctrl+A=one arrows=work rightclick=separate double=emitted"
    );
}

#[gpui_kit::test]
fn public_row_slot_multiselect_cannot_replace_table_semantics(cx: &mut TestAppContext) {
    let external = Rc::new(RefCell::new(BTreeSet::new()));
    let (probe, mut surface) = mount(cx, Some(external.clone()), false);
    let state = table(&probe, &surface);
    click(&mut surface, 0, Modifiers::default(), 1);
    click(
        &mut surface,
        1,
        Modifiers {
            control: true,
            ..Default::default()
        },
        1,
    );
    assert_eq!(*external.borrow(), BTreeSet::from([0, 1]));
    click(
        &mut surface,
        3,
        Modifiers {
            shift: true,
            ..Default::default()
        },
        1,
    );
    assert_eq!(*external.borrow(), BTreeSet::from([1, 2, 3]));
    assert_eq!(
        selected(&mut surface),
        0,
        "DataTable overwrites public row aria_selected; external set alone is not selected semantics"
    );
    assert_eq!(
        state.read_with(&surface, |state, _| state.selection()),
        TableSelection::None
    );
    surface.update(|window, cx| state.read(cx).focus_handle(cx).focus(window, cx));
    key(&mut surface, "down");
    key(&mut surface, "shift-down");
    key(&mut surface, "ctrl-a");
    assert_eq!(*external.borrow(), BTreeSet::from([1, 2, 3]));
    assert_eq!(
        state.read_with(&surface, |state, _| state.selection()),
        TableSelection::None
    );
    click(&mut surface, 3, Modifiers::default(), 2);
    assert!(
        !probe.read_with(&surface, |probe, _| probe
            .events
            .borrow()
            .contains(&"double")),
        "disabling row selection also disables its double-click event"
    );
    eprintln!(
        "SPIKE public slot: external_range=3 semantic_selected_rows=0 built_in_arrows=disabled double_event=absent"
    );
}

#[gpui_kit::test]
fn tree_table_public_ids_sync_on_deferred_turn_without_reentry(cx: &mut TestAppContext) {
    let (probe, mut surface) = mount(cx, None, true);
    let state = table(&probe, &surface);
    let tree = probe.read_with(&surface, |probe, _| probe.tree.clone());
    let folder_b = surface.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .find(|node| {
                node.role() == Some(gpui_kit::Role::TreeItem) && node.label() == Some("Folder B")
            })
            .expect("actual Tree item")
            .bounds()
    });
    surface.simulate_click(folder_b.center(), Modifiers::default());
    draw(&mut surface);
    assert_eq!(
        probe.read_with(&surface, |probe, _| probe.folder.clone()),
        "folder-b"
    );
    assert_eq!(
        state.read_with(&surface, |state, _| state.delegate().ids.clone()),
        vec![20, 21]
    );
    surface.update(|window, cx| tree.update(cx, |tree, cx| tree.focus(window, cx)));
    key(&mut surface, "up");
    assert_eq!(
        probe.read_with(&surface, |probe, _| probe.folder.clone()),
        "folder-a"
    );
    assert_eq!(
        state.read_with(&surface, |state, _| state.delegate().ids.clone()),
        vec![10, 11]
    );
    key(&mut surface, "down");
    assert_eq!(
        probe.read_with(&surface, |probe, _| probe.folder.clone()),
        "folder-b"
    );
    let count = probe.read_with(&surface, |probe, _| probe.sync_count);
    tree.update(&mut surface, |_, cx| cx.notify());
    draw(&mut surface);
    assert_eq!(
        probe.read_with(&surface, |probe, _| probe.sync_count),
        count,
        "unchanged ID notify has no feedback loop"
    );
    tree.update(&mut surface, |state, cx| {
        state.set_selected_item(Some(&TreeItem::new("root", "archive.zip")), cx)
    });
    draw(&mut surface);
    assert_eq!(
        state.read_with(&surface, |state, _| state.delegate().ids.clone()),
        vec![0, 1, 2, 3, 4, 5]
    );
    click(&mut surface, 0, Modifiers::default(), 2);
    assert_eq!(
        probe.read_with(&surface, |probe, _| probe.folder.clone()),
        "folder-a"
    );
    assert_eq!(
        tree.read_with(&surface, |tree, _| tree
            .selected_item()
            .map(|item| item.id.to_string())),
        Some("folder-a".into())
    );
    assert_eq!(
        state.read_with(&surface, |state, _| state.delegate().ids.clone()),
        vec![10, 11]
    );
    eprintln!(
        "SPIKE Tree pointer->table and table double->Tree: public IDs, deduplicated deferred updates"
    );
}
