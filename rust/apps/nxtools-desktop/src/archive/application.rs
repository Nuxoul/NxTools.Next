#[path = "entry.rs"]
mod entry;
#[path = "jobs.rs"]
mod jobs;
#[path = "launch_entry.rs"]
mod launch_entry;
#[path = "password.rs"]
mod password;
#[path = "readmission.rs"]
mod readmission;
use entry::{Lease, PendingEntry};
pub(crate) use launch_entry::{
    extract_from_window, handle_launch_intent, new_from_window, open_internal,
};
#[cfg(test)]
#[path = "entry_tests.rs"]
mod entry_tests;
#[cfg(test)]
#[path = "application_tests.rs"]
mod tests;
use super::browser::ui_state::{UiState, UiStateLoad, UiStateStore};
use super::browser::{ArchiveBrowser, BrowserStatus};
use gpui_kit::component::Disableable as _;
use gpui_kit::{
    AnyWindowHandle, App, AppContext, Bounds, Context, Entity, Global, Task, WeakEntity,
    WindowBounds, WindowOptions, px, size,
};
use nxtools_archive::product::{ArchiveReadClient, ArchiveReadOperation};
use nxtools_ipc::{PlatformBridge, PlatformNotification};
use nxtools_protocol::archive_browser::ArchiveBrowserContext;
use nxtools_protocol::archive_read::*;
use std::{collections::HashMap, path::PathBuf, rc::Rc, sync::Arc, time::Instant};
use uuid::Uuid;

struct ArchiveGlobal(Entity<ArchiveApplication>);
impl Global for ArchiveGlobal {}
pub(crate) fn register(
    platform: Rc<PlatformBridge>,
    settings: nxtools_core::archive::ArchiveSettings,
    ui_state_path: PathBuf,
    cx: &mut App,
) {
    let client = ArchiveReadClient::new(platform.archive_client_bridge())
        .ok()
        .map(Arc::new);
    let owner = cx.new(|cx| {
        let mut owner = ArchiveApplication::new(platform, client, cx);
        owner.settings = settings;
        owner.ui_state = UiStateStore::initialize(ui_state_path).ok();
        owner
    });
    cx.set_global(ArchiveGlobal(owner));
    crate::archive_launch::register_handler(handle_launch_intent, cx);
}
pub(crate) fn set_config_path(path: PathBuf, cx: &mut App) {
    if let Some(owner) = cx
        .try_global::<ArchiveGlobal>()
        .map(|global| global.0.clone())
    {
        owner.update(cx, |app, _| app.config_path = Some(path));
    }
}
pub(crate) fn settings_updated(settings: nxtools_core::archive::ArchiveSettings, cx: &mut App) {
    if let Some(owner) = cx.try_global::<ArchiveGlobal>().map(|g| g.0.downgrade()) {
        cx.defer(move |cx| {
            let _ = owner.update(cx, |app, _| app.settings = settings);
        });
    }
}
pub(crate) fn pick_from_window(source: AnyWindowHandle, cx: &mut App) {
    if let Some(owner) = cx.try_global::<ArchiveGlobal>().map(|g| g.0.downgrade()) {
        cx.defer(move |cx| {
            let _ = owner.update(cx, |app, cx| app.pick_archive(source, cx));
        });
    }
}
pub(crate) fn handle(notification: PlatformNotification, cx: &mut App) {
    if let Some(owner) = cx.try_global::<ArchiveGlobal>().map(|g| g.0.downgrade()) {
        cx.defer(move |cx| {
            let _ = owner.update(cx, |app, cx| app.notification(notification, cx));
        });
    }
}
struct BrowserSlot {
    path: PathBuf,
    view: WeakEntity<ArchiveBrowser>,
    window: AnyWindowHandle,
    generation: u64,
    browser: Option<ArchiveBrowserContext>,
    depth: u32,
    leases: Vec<Lease>,
}
struct PendingRead {
    window_id: Uuid,
    generation: u64,
    operation: ArchiveReadOperation,
    entries: Vec<ArchiveReadEntry>,
    snapshot: Option<Uuid>,
    format: String,
}
struct Picker {
    source: AnyWindowHandle,
    started: Instant,
}
struct OpenOrigin {
    window: AnyWindowHandle,
    browser: Option<(Uuid, u64)>,
}
pub(crate) struct ArchiveApplication {
    platform: Rc<PlatformBridge>,
    client: Option<Arc<ArchiveReadClient>>,
    settings: nxtools_core::archive::ArchiveSettings,
    config_path: Option<PathBuf>,
    windows: HashMap<Uuid, BrowserSlot>,
    paths: HashMap<PathBuf, Uuid>,
    reads: HashMap<Uuid, PendingRead>,
    picker: Option<(Uuid, Picker)>,
    entries: HashMap<Uuid, PendingEntry>,
    shells: HashMap<Uuid, (Uuid, u64, Lease)>,
    ui_state: Option<UiStateStore>,
    passwords: HashMap<Uuid, nxtools_protocol::archive_password::ArchivePasswordPrompt>,
    jobs: jobs::Jobs,
    _poll: Option<Task<()>>,
    _drain: Task<()>,
}
impl ArchiveApplication {
    fn new(
        platform: Rc<PlatformBridge>,
        client: Option<Arc<ArchiveReadClient>>,
        cx: &mut Context<Self>,
    ) -> Self {
        let (signal, receiver) = async_channel::bounded(1);
        let poll = client.clone().map(|client| {
            cx.background_executor().spawn(async move {
                loop {
                    if client.poll().await.is_err() {
                        break;
                    }
                    let _ = signal.try_send(());
                }
            })
        });
        let drain = cx.spawn(async move |this, cx| {
            while receiver.recv().await.is_ok() {
                let _ = this.update(cx, |app, cx| app.consume(cx));
            }
        });
        Self {
            platform,
            client,
            settings: Default::default(),
            config_path: None,
            windows: HashMap::new(),
            paths: HashMap::new(),
            reads: HashMap::new(),
            picker: None,
            entries: HashMap::new(),
            shells: HashMap::new(),
            ui_state: None,
            passwords: HashMap::new(),
            jobs: jobs::Jobs::default(),
            _poll: poll,
            _drain: drain,
        }
    }
    pub(crate) fn pick_archive(&mut self, source: AnyWindowHandle, cx: &mut Context<Self>) {
        if self.picker.is_some() {
            return;
        }
        let native = source.update(cx, |_, window, _| {
            crate::ui::native_window_handle(window).unwrap_or(0)
        });
        let Ok(source_hwnd) = native else {
            return;
        };
        let id = Uuid::new_v4();
        self.picker = Some((
            id,
            Picker {
                source,
                started: Instant::now(),
            },
        ));
        if self
            .platform
            .send_command_with_id(
                nxtools_protocol::PlatformCommand::PickArchivePath(
                    nxtools_protocol::PickArchivePathCommand { source_hwnd },
                ),
                id,
            )
            .is_err()
        {
            self.picker = None;
            tracing::warn!(event="archive.picker.finished",correlation_id=%id,outcome="not_submitted","archive picker submission failed");
            picker_failure(source, cx);
        } else {
            tracing::info!(event="archive.picker.started",correlation_id=%id,"archive picker accepted");
        }
    }
    fn open_at_depth(
        &mut self,
        path: PathBuf,
        depth: u32,
        leases: Vec<Lease>,
        origin: Option<OpenOrigin>,
        cx: &mut Context<Self>,
    ) {
        let owner = cx.entity().downgrade();
        let background = cx.background_executor().clone();
        cx.spawn(async move |_, cx| {
            let normalized = background
                .spawn(async move {
                    std::fs::canonicalize(path).map(|path| {
                        let value = path.to_string_lossy();
                        if let Some(stripped) = value.strip_prefix(r"\\?\") {
                            PathBuf::from(stripped)
                        } else {
                            path
                        }
                    })
                })
                .await;
            let _ = owner.update(cx, |app, cx| {
                let live = origin.as_ref().is_none_or(|origin| {
                    origin.window.update(cx, |_, _, _| ()).is_ok()
                        && origin.browser.is_none_or(|(id, generation)| {
                            app.windows
                                .get(&id)
                                .is_some_and(|slot| slot.generation == generation)
                        })
                });
                if !live {
                    for lease in leases {
                        app.release(lease);
                    }
                    return;
                }
                match normalized {
                    Ok(path) => app.open_canonical(path, depth, leases, cx),
                    Err(_) => {
                        for lease in leases {
                            app.release(lease);
                        }
                        if let Some(origin) = origin {
                            cx.defer(move |cx| {
                                let _ = origin.window.update(cx, |_, window, cx| {
                                    use gpui_kit::ParentElement;
                                    use gpui_kit::component::{WindowExt, dialog::DialogClose};
                                    window.open_dialog(cx, |dialog, _, _| {
                                        dialog
                                            .title("无法打开归档")
                                            .child("文件已不可用或没有访问权限，请重新选择。")
                                            .footer(
                                                DialogClose::new()
                                                    .trigger(|button| button.label("关闭")),
                                            )
                                    });
                                });
                            });
                        }
                        tracing::warn!(
                            event = "archive.browser.open_failed",
                            reason = "source_unavailable",
                            "archive browser open failed"
                        );
                    }
                }
            });
        })
        .detach();
    }
    fn open_canonical(
        &mut self,
        path: PathBuf,
        depth: u32,
        leases: Vec<Lease>,
        cx: &mut Context<Self>,
    ) {
        if let Some(id) = self.paths.get(&path).copied() {
            if let Some(slot) = self.windows.get(&id) {
                for lease in leases {
                    self.release(lease);
                }
                let window = slot.window;
                cx.defer(move |cx| {
                    let _ = window.update(cx, |_, window, cx| {
                        window.activate_window();
                        window.focus_next(cx);
                    });
                });
                return;
            }
            self.paths.remove(&path);
        }
        if self
            .ui_state
            .as_ref()
            .is_some_and(|store| matches!(store.load_status(), UiStateLoad::Loading))
        {
            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(10))
                    .await;
                let _ = this.update(cx, |app, cx| app.open_canonical(path, depth, leases, cx));
            })
            .detach();
            return;
        }
        let restored = self
            .ui_state
            .as_ref()
            .and_then(|store| match store.load_status() {
                UiStateLoad::Ready(_) => Some(store.current_snapshot()),
                _ => None,
            });
        let id = Uuid::new_v4();
        let owner = cx.entity().downgrade();
        let view_slot = Rc::new(std::cell::RefCell::new(None));
        let capture = view_slot.clone();
        let path_for_view = path.clone();
        let default_bounds = Bounds::centered(None, size(px(960.0), px(640.0)), cx);
        let bounds = restored
            .as_ref()
            .map_or(WindowBounds::Windowed(default_bounds), |state| {
                let geometry = state.geometry;
                let candidate = Bounds::new(
                    gpui_kit::point(px(geometry.x as f32), px(geometry.y as f32)),
                    size(px(geometry.width as f32), px(geometry.height as f32)),
                );
                let visible = cx
                    .displays()
                    .iter()
                    .any(|display| display.bounds().contains(&candidate.origin));
                let bounds = if visible {
                    candidate
                } else {
                    Bounds::centered(None, candidate.size, cx)
                };
                if geometry.maximized {
                    WindowBounds::Maximized(bounds)
                } else {
                    WindowBounds::Windowed(bounds)
                }
            });
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(bounds),
                window_min_size: Some(size(px(720.0), px(480.0))),
                titlebar: None,
                app_id: Some(format!("NxTools.Archive.{id}")),
                ..Default::default()
            },
            move |window, cx| {
                window.set_window_title(&format!(
                    "{} — NxTools",
                    path_for_view
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                ));
                let view = cx.new(|cx| {
                    let mut view =
                        ArchiveBrowser::new(path_for_view, id, owner.clone(), window, cx);
                    if let Some(restored) = restored {
                        view.apply_restored_state(restored, window, cx);
                    }
                    view
                });
                let closing_view = view.downgrade();
                window.on_window_should_close(cx, move |window, cx| {
                    let state = closing_view
                        .update(cx, |view, _| view.snapshot(window))
                        .ok();
                    let owner = owner.clone();
                    cx.defer(move |cx| {
                        let _ = owner.update(cx, |app, cx| {
                            if let Some(state) = state {
                                app.save_ui_state(state);
                            }
                            app.retire_browser(id, cx);
                        });
                    });
                    true
                });
                *capture.borrow_mut() = Some(view.downgrade());
                cx.new(|cx| gpui_kit::base::Root::new(view, window, cx))
            },
        );
        if let (Ok(window), Some(view)) = (opened, view_slot.borrow_mut().take()) {
            self.paths.insert(path.clone(), id);
            self.windows.insert(
                id,
                BrowserSlot {
                    path,
                    view,
                    window: window.into(),
                    generation: 0,
                    browser: None,
                    depth,
                    leases,
                },
            );
            self.refresh_browser(id, default_codepage(self.settings.filename_encoding), cx);
        } else {
            for lease in leases {
                self.release(lease);
            }
        }
    }
    pub(crate) fn refresh_browser(&mut self, id: Uuid, codepage: u32, cx: &mut Context<Self>) {
        self.cancel_browser(id, cx);
        let Some(slot) = self.windows.get_mut(&id) else {
            return;
        };
        slot.generation = slot.generation.wrapping_add(1).max(1);
        let generation = slot.generation;
        slot.browser = None;
        let request = ArchiveReadCommand {
            version: 2,
            operation_id: Uuid::new_v4(),
            session_id: id,
            generation,
            action: ArchiveReadAction::Open,
            path: Some(slot.path.to_string_lossy().into_owned()),
            codepage,
            limits: Some(ArchiveReadLimits {
                max_entries: 100_000,
                max_metadata_bytes: 128 << 20,
                max_retained_bytes: 64 << 20,
                max_transport_bytes: 64 << 20,
                ack_timeout_ms: 10000,
                browser_context: true,
                ..Default::default()
            }),
            sequence: None,
        };
        let result = self
            .client
            .as_ref()
            .ok_or(ArchiveReadError::Disconnected)
            .and_then(|client| client.start(request));
        let view = slot.view.clone();
        let operation_id = result.as_ref().ok().map(ArchiveReadOperation::operation_id);
        let loading_view = view.clone();
        cx.defer(move |cx| {
            let _ = loading_view.update(cx, |view, cx| {
                if view.generation > generation {
                    return;
                }
                view.generation = generation;
                view.codepage = codepage;
                view.read_id = operation_id;
                view.status = BrowserStatus::Loading;
                view.finish_entry_open(cx);
            });
        });
        match result {
            Ok(operation) => {
                let operation_id = operation.operation_id();
                self.reads.insert(
                    operation_id,
                    PendingRead {
                        window_id: id,
                        generation,
                        operation,
                        entries: vec![],
                        snapshot: None,
                        format: String::new(),
                    },
                );
            }
            Err(error) => self.fail_view(
                id,
                format!("无法读取归档：{error}。请检查原生适配器和文件后重试。"),
                cx,
            ),
        }
    }
    fn consume(&mut self, cx: &mut Context<Self>) {
        let mut finished = vec![];
        for (id, read) in &mut self.reads {
            if let Some(batch) = read.operation.take_batch() {
                read.snapshot = Some(batch.snapshot_id);
                read.format = batch.format;
                let sequence = batch.sequence;
                read.entries.extend(batch.entries);
                if read.operation.acknowledge(sequence).is_err() {
                    let _ = read.operation.cancel();
                }
            }
            if let Some(terminal) = read.operation.terminal() {
                finished.push((*id, terminal));
            }
        }
        for (id, terminal) in finished {
            let Some(read) = self.reads.remove(&id) else {
                continue;
            };
            self.passwords.remove(&id);
            let Some(slot) = self
                .windows
                .get_mut(&read.window_id)
                .filter(|slot| slot.generation == read.generation)
            else {
                continue;
            };
            let view = slot.view.clone();
            let window = slot.window;
            if terminal.status == ArchiveReadStatus::Succeeded && terminal.cleanup_complete {
                let Some(browser) = terminal.browser.filter(|browser| {
                    browser.validate().is_ok()
                        && read.snapshot.is_none_or(|id| id == browser.snapshot_id)
                }) else {
                    self.fail_view(
                        read.window_id,
                        "原生读取未返回有效浏览身份，请更新匹配组件后重试。".into(),
                        cx,
                    );
                    continue;
                };
                let snapshot = browser.snapshot_id;
                let format = browser.format.clone();
                let source_size = browser.source.size;
                let comment = browser.comment.clone();
                slot.browser = Some(*browser);
                cx.defer(move |cx| {
                    let _ = window.update(cx, |_, window, cx| {
                        let _ = view.update(cx, |view, cx| {
                            if view.generation == read.generation {
                                view.read_id = None;
                                view.install(snapshot, read.entries, format, window, cx);
                                view.set_metadata(Some(source_size), comment, None, None, cx);
                            }
                        });
                    });
                });
            } else {
                let message = if terminal.status == ArchiveReadStatus::Cancelled {
                    "读取已取消".into()
                } else {
                    format!(
                        "读取失败：{}。上次有效视图仅供查看，请刷新后再操作。",
                        terminal.code.map_or("Unknown".into(), |c| c.to_string())
                    )
                };
                self.fail_view(read.window_id, message, cx);
            }
        }
    }
    fn fail_view(&self, id: Uuid, message: String, cx: &mut Context<Self>) {
        if let Some(slot) = self.windows.get(&id) {
            let view = slot.view.clone();
            let window = slot.window;
            let generation = slot.generation;
            cx.defer(move |cx| {
                let _ = window.update(cx, |_, window, cx| {
                    let _ = view.update(cx, |view, cx| {
                        if view.generation != generation {
                            return;
                        }
                        view.read_id = None;
                        view.set_failure(message, window, cx);
                    });
                });
            });
        }
    }
    pub(crate) fn cancel_browser(&mut self, id: Uuid, cx: &mut Context<Self>) {
        self.passwords.retain(|_, prompt| prompt.session_id != id);
        let ids = self
            .reads
            .iter()
            .filter(|(_, r)| r.window_id == id)
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();
        for read in ids {
            self.reads.remove(&read);
        }
        self.cancel_entries(id);
        self.finish_open(id, cx);
        if let Some(slot) = self.windows.get(&id) {
            let view = slot.view.clone();
            let generation = slot.generation;
            cx.defer(move |cx| {
                let _ = view.update(cx, |view, cx| {
                    if view.generation != generation {
                        return;
                    }
                    view.read_id = None;
                    view.status = BrowserStatus::Cancelled;
                    cx.notify();
                });
            });
        }
    }
    fn retire_browser(&mut self, id: Uuid, cx: &mut Context<Self>) {
        let state = self.windows.get(&id).and_then(|slot| {
            slot.view
                .read_with(cx, |view, _| {
                    view.final_snapshot()
                        .unwrap_or_else(|| view.snapshot_from_observed())
                })
                .ok()
        });
        if let Some(state) = state {
            self.save_ui_state(state);
        }
        self.cancel_browser(id, cx);
        if let Some(slot) = self.windows.remove(&id) {
            self.paths.remove(&slot.path);
            for lease in slot.leases {
                self.release(lease);
            }
        }
        tracing::info!(event="archive.browser.retired",correlation_id=%id,"archive browser retired");
    }
    pub(crate) fn close_browser(&mut self, id: Uuid, cx: &mut Context<Self>) {
        let window = self.windows.get(&id).map(|s| s.window);
        self.retire_browser(id, cx);
        if let Some(window) = window {
            cx.defer(move |cx| {
                let _ = window.update(cx, |_, window, _| window.remove_window());
            });
        }
    }
    fn save_ui_state(&self, state: UiState) {
        if let Some(store) = &self.ui_state
            && let Err(reason) = store.enqueue_save(state)
        {
            tracing::warn!(
                event = "archive.ui_state.save_failed",
                ?reason,
                "archive UI state preserved"
            );
        }
    }
    pub(crate) fn handle_drop(
        &mut self,
        id: Uuid,
        paths: Vec<PathBuf>,
        source: AnyWindowHandle,
        cx: &mut Context<Self>,
    ) {
        if paths.is_empty() || paths.len() > 31 {
            self.notice(id, "一次最多拖入 31 个来源；未截断文件列表。".into(), cx);
            return;
        }
        let Some(slot) = self.windows.get(&id) else {
            return;
        };
        let generation = slot.generation;
        let eligible = slot.browser.as_ref().is_some_and(|b| b.format == "Zip")
            && slot
                .view
                .read_with(cx, |v, _| v.edit_reason().is_none())
                .unwrap_or(false);
        let owner = cx.entity().downgrade();
        cx.defer(move |cx| {
            let _ = source.update(cx, |_, window, cx| {
                use gpui_kit::ParentElement;
                use gpui_kit::component::{
                    WindowExt,
                    button::Button,
                    dialog::{DialogAction, DialogClose, DialogFooter},
                };
                let owner_open = owner.clone();
                let owner_add = owner.clone();
                let open_path = paths[0].clone();
                let add_paths = paths.clone();
                window.open_dialog(cx, move |dialog, _, _| {
                    let owner_open = owner_open.clone();
                    let owner_add = owner_add.clone();
                    let open_path = open_path.clone();
                    let add_paths = add_paths.clone();
                    dialog
                        .title("拖入文件")
                        .child(if eligible {
                            "打开为归档，或添加到当前 ZIP？"
                        } else {
                            "当前归档不能编辑；可以打开首个来源。"
                        })
                        .footer(
                            DialogFooter::new()
                                .child(DialogClose::new().trigger(|button| button.label("取消")))
                                .child(
                                    DialogAction::new().child(
                                        Button::new("archive-drop-open").label("打开为归档"),
                                    ),
                                )
                                .child(
                                    Button::new("archive-drop-add")
                                        .label("添加到 ZIP")
                                        .disabled(!eligible)
                                        .on_click(move |_, window, cx| {
                                            let owner = owner_add.clone();
                                            let paths = add_paths.clone();
                                            window.close_dialog(cx);
                                            cx.defer(move |cx| {
                                                let _ = owner.update(cx, |app, cx| {
                                                    app.add_drop(id, generation, paths, cx)
                                                });
                                            });
                                        }),
                                ),
                        )
                        .on_ok(move |_, _, cx| {
                            let owner = owner_open.clone();
                            let path = open_path.clone();
                            cx.defer(move |cx| {
                                let _ = owner.update(cx, |app, cx| {
                                    if app
                                        .windows
                                        .get(&id)
                                        .is_some_and(|slot| slot.generation == generation)
                                    {
                                        app.open_at_depth(
                                            path,
                                            0,
                                            vec![],
                                            Some(OpenOrigin {
                                                window: source,
                                                browser: Some((id, generation)),
                                            }),
                                            cx,
                                        );
                                    }
                                });
                            });
                            true
                        })
                });
            });
        });
    }
    fn add_drop(&mut self, id: Uuid, generation: u64, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        let Some(slot) = self
            .windows
            .get(&id)
            .filter(|slot| slot.generation == generation)
        else {
            return;
        };
        let Some((snapshot, prefix, codepage)) = slot
            .view
            .read_with(cx, |view, _| {
                if view.edit_reason().is_some() {
                    return None;
                }
                let model = view.model.as_ref()?;
                Some((
                    model.snapshot,
                    (!model.folder.is_empty()).then(|| format!("{}/", model.folder)),
                    view.codepage,
                ))
            })
            .ok()
            .flatten()
        else {
            return;
        };
        let Some(browser) = slot
            .browser
            .clone()
            .filter(|browser| browser.snapshot_id == snapshot)
        else {
            return;
        };
        let draft = crate::archive::draft::ModifyDraft {
            source: slot.path.clone(),
            browser,
            action: nxtools_protocol::archive_job::ArchiveJobModifyAction::Add,
            ordinals: vec![],
            value: prefix,
            added: paths
                .iter()
                .filter_map(|p| p.to_str().map(str::to_owned))
                .collect(),
            codepage,
        };
        if draft.added.len() != paths.len() || draft.request().is_err() {
            self.notice(id, "拖入来源无效；未丢弃任何路径。".into(), cx);
            return;
        }
        self.open_modify_form(draft, (id, generation, snapshot), cx);
    }
    fn notification(&mut self, notification: PlatformNotification, cx: &mut Context<Self>) {
        if self.job_notification(&notification, cx) {
            return;
        }
        if let PlatformNotification::Event(nxtools_protocol::PlatformEvent::ArchivePassword(
            prompt,
        )) = &notification
        {
            self.password_prompt(prompt.clone(), cx);
            return;
        }
        if matches!(notification, PlatformNotification::Disconnected { .. }) {
            self.picker = None;
            self.reads.clear();
            self.entries.clear();
            self.shells.clear();
            self.passwords.clear();
            let ids = self.windows.keys().copied().collect::<Vec<_>>();
            for id in ids {
                if let Some(slot) = self.windows.get_mut(&id) {
                    slot.browser = None;
                    slot.leases.clear();
                }
                self.finish_open(id, cx);
                self.fail_view(
                    id,
                    "原生连接已断开；保留的视图仅供查看，请重新连接后刷新。".into(),
                    cx,
                );
            }
            return;
        }
        if let PlatformNotification::CommandResult {
            correlation_id,
            result,
        } = notification
        {
            if self.entry_result(correlation_id, &result, cx)
                || self.shell_result(correlation_id, &result, cx)
            {
                return;
            }
            if self
                .picker
                .as_ref()
                .is_some_and(|(id, _)| *id == correlation_id)
            {
                let (_, picker) = self.picker.take().expect("matching picker");
                tracing::info!(event="archive.picker.finished",correlation_id=%correlation_id,elapsed_ms=picker.started.elapsed().as_millis() as u64,success=result.success,"archive picker finished");
                let source = picker.source;
                if result.success
                    && let Some(nxtools_protocol::ArchivePickerResult::Selected { path }) = result
                        .data
                        .as_deref()
                        .and_then(|data| serde_json::from_str(data).ok())
                {
                    let owner = cx.entity().downgrade();
                    cx.defer(move |cx| {
                        if source.update(cx, |_, _, _| ()).is_ok() {
                            let _ = owner.update(cx, |app, cx| {
                                app.open_at_depth(
                                    path.into(),
                                    0,
                                    vec![],
                                    Some(OpenOrigin {
                                        window: source,
                                        browser: None,
                                    }),
                                    cx,
                                )
                            });
                        }
                    });
                }
                if !result.success {
                    picker_failure(source, cx);
                }
            }
        }
    }
}

fn default_codepage(encoding: nxtools_core::archive::ArchiveFilenameEncoding) -> u32 {
    use nxtools_core::archive::ArchiveFilenameEncoding::*;
    match encoding {
        Auto => 0,
        Utf8 => 65001,
        Gbk => 936,
        Big5 => 950,
        ShiftJis => 932,
        Korean => 949,
        Cp437 => 437,
        Windows1250 => 1250,
        Windows1251 => 1251,
        Windows1252 => 1252,
    }
}

fn picker_failure(source: AnyWindowHandle, cx: &mut App) {
    cx.defer(move |cx| {
        let _ = source.update(cx, |_, window, cx| {
            use gpui_kit::ParentElement;
            use gpui_kit::component::{WindowExt, dialog::DialogClose};
            window.open_dialog(cx, |dialog, _, _| {
                dialog
                    .title("无法选择归档")
                    .child("原生文件选择器不可用或连接已断开，请重试。")
                    .footer(DialogClose::new().trigger(|button| button.label("关闭")))
            });
        });
    });
}
