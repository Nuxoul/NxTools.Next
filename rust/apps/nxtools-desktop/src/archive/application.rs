#[path = "browser_host.rs"]
mod browser_host;
#[path = "clipboard_requests.rs"]
mod clipboard_requests;
#[path = "create_output_plan.rs"]
mod create_output_plan;
#[path = "drag_progress.rs"]
mod drag_progress;
#[path = "entry.rs"]
mod entry;
#[path = "job_location.rs"]
mod job_location;
#[path = "jobs.rs"]
mod jobs;
#[path = "launch_entry.rs"]
mod launch_entry;
#[path = "password.rs"]
mod password;
#[path = "readmission.rs"]
mod readmission;
#[path = "recovery_requests.rs"]
mod recovery_requests;
#[cfg(test)]
#[path = "recovery_tests.rs"]
mod recovery_tests;
#[path = "shell_integration.rs"]
pub(crate) mod shell_integration;
#[path = "space_requests.rs"]
mod space_requests;
use browser_host::BrowserHost;
use entry::{Lease, PendingEntry};
pub(crate) use launch_entry::{
    extract_from_window, handle_launch_intent, new_from_window, open_internal,
};
#[cfg(test)]
#[path = "entry_tests.rs"]
mod entry_tests;
#[cfg(test)]
#[path = "hz1_lifecycle_tests.rs"]
mod hz1_lifecycle_tests;
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

static SETTINGS_SAVE_ORDER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

pub(crate) struct SavedArchiveSettings {
    settings: nxtools_core::archive::ArchiveSettings,
    order: u64,
}

impl SavedArchiveSettings {
    pub(crate) fn settings(&self) -> &nxtools_core::archive::ArchiveSettings {
        &self.settings
    }
}

pub(crate) fn save_settings(
    path: &std::path::Path,
    mutate: impl FnOnce(&mut nxtools_core::archive::ArchiveSettings),
) -> anyhow::Result<SavedArchiveSettings> {
    let mut order = 0;
    let merged = nxtools_core::AppConfig::update_section(path, |config| {
        mutate(&mut config.archive);
        // Allocated under the original config lock, so order follows serialized disk saves.
        order = SETTINGS_SAVE_ORDER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    })?;
    Ok(SavedArchiveSettings {
        settings: merged.archive,
        order,
    })
}

pub(crate) fn saved_settings_updated(saved: SavedArchiveSettings, cx: &mut App) {
    if let Some(owner) = cx.try_global::<ArchiveGlobal>().map(|g| g.0.downgrade()) {
        cx.defer(move |cx| {
            let _ = owner.update(cx, |app, cx| app.publish_saved_settings(saved, cx));
        });
    }
}
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
        if let Ok(store) = UiStateStore::initialize(ui_state_path) {
            owner.install_ui_state(store, cx);
        }
        owner
    });
    cx.set_global(ArchiveGlobal(owner));
    crate::archive_launch::register_handler(handle_launch_intent, cx);
}
pub(crate) fn recent_snapshot(cx: &App) -> Result<Vec<PathBuf>, &'static str> {
    let owner = cx
        .try_global::<ArchiveGlobal>()
        .ok_or("owner_unavailable")?;
    owner.0.read(cx).recent_paths()
}
pub(crate) fn recent_owner(cx: &App) -> Option<Entity<ArchiveApplication>> {
    cx.try_global::<ArchiveGlobal>()
        .map(|global| global.0.clone())
}
pub(crate) fn clear_recent(
    reply: impl Fn(Result<(), &'static str>, &mut App) + 'static,
    cx: &mut App,
) {
    let owner = cx
        .try_global::<ArchiveGlobal>()
        .map(|global| global.0.downgrade());
    let correlation_id = Uuid::new_v4();
    let started = Instant::now();
    tracing::info!(event="archive.recent.clear_started", %correlation_id, "recent archive clear accepted");
    cx.defer(move |cx| {
        let pending = owner
            .and_then(|owner| owner.update(cx, |app, cx| app.clear_recent_paths(cx)).ok())
            .unwrap_or(Err("owner_unavailable"));
        cx.spawn(async move |cx| {
            let result = match pending {
                Ok(receipt) => match receipt.recv().await {
                    Ok(true) => Ok(()),
                    _ => Err("write_failed"),
                },
                Err(reason) => Err(reason),
            };
            tracing::info!(event="archive.recent.clear_finished", %correlation_id, success=result.is_ok(), elapsed_ms=started.elapsed().as_millis() as u64, "recent archive clear finished");
            cx.update(|cx| reply(result, cx));
        })
        .detach();
    });
}
pub(crate) fn open_recent(path: PathBuf, source: AnyWindowHandle, cx: &mut App) {
    if let Some(owner) = cx
        .try_global::<ArchiveGlobal>()
        .map(|global| global.0.downgrade())
    {
        cx.defer(move |cx| {
            let _ = owner.update(cx, |app, cx| {
                app.open_at_depth(
                    path,
                    0,
                    vec![],
                    Some(OpenOrigin {
                        window: source,
                        browser: None,
                    }),
                    cx,
                )
            });
        });
    }
}
type TempDirectoryReply = Rc<dyn Fn(Result<Option<String>, &'static str>, &mut App)>;
pub(crate) fn pick_temp_directory(
    current: String,
    source: AnyWindowHandle,
    reply: impl Fn(Result<Option<String>, &'static str>, &mut App) + 'static,
    cx: &mut App,
) {
    let owner = cx
        .try_global::<ArchiveGlobal>()
        .map(|global| global.0.downgrade());
    let reply: TempDirectoryReply = Rc::new(reply);
    cx.defer(move |cx| {
        let fallback = reply.clone();
        if owner.is_none_or(|owner| {
            owner
                .update(cx, |app, cx| {
                    if app.temp_directory_picker.is_some() {
                        cx.defer(move |cx| reply(Err("busy"), cx));
                        return;
                    }
                    let id = Uuid::new_v4();
                    let source_window_handle = source
                        .update(cx, |_, window, _| {
                            crate::ui::native_window_handle(window).unwrap_or_default()
                        })
                        .unwrap_or_default();
                    app.temp_directory_picker = Some((id, reply.clone()));
                    if app
                        .platform
                        .send_command_with_id(
                            nxtools_protocol::PlatformCommand::PickDirectory(
                                nxtools_protocol::PickDirectoryCommand {
                                    current_directory: current,
                                    title: "选择压缩包临时目录".into(),
                                    source_window_handle,
                                    source_process_id: std::process::id(),
                                },
                            ),
                            id,
                        )
                        .is_err()
                    {
                        app.temp_directory_picker = None;
                        cx.defer(move |cx| reply(Err("disconnected"), cx));
                    }
                })
                .is_err()
        }) {
            fallback(Err("owner_unavailable"), cx);
        }
    });
}
pub(crate) fn set_config_path(path: PathBuf, cx: &mut App) {
    if let Some(owner) = cx
        .try_global::<ArchiveGlobal>()
        .map(|global| global.0.clone())
    {
        owner.update(cx, |app, _| app.config_path = Some(path));
    }
}
#[cfg(test)]
pub(crate) fn settings_updated(settings: nxtools_core::archive::ArchiveSettings, cx: &mut App) {
    saved_settings_updated(
        SavedArchiveSettings {
            settings,
            order: SETTINGS_SAVE_ORDER.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        },
        cx,
    );
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
#[derive(Clone)]
struct DropAddTarget {
    request: nxtools_protocol::archive_job::ArchiveJobRequest,
    origin: jobs::EditFormOrigin,
}

struct BrowserSlot {
    path: PathBuf,
    view: WeakEntity<ArchiveBrowser>,
    window: AnyWindowHandle,
    generation: u64,
    browser: Option<ArchiveBrowserContext>,
    depth: u32,
    leases: Vec<Lease>,
    preview_lease: Option<Lease>,
}
struct BrowserHostSlot {
    id: Uuid,
    view: WeakEntity<BrowserHost>,
    window: AnyWindowHandle,
    password_recovery: password::PasswordRecovery,
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
    settings_save_order: u64,
    config_path: Option<PathBuf>,
    #[cfg(test)]
    remember_publication_barrier: Option<async_channel::Receiver<()>>,
    #[cfg(test)]
    preview_decode_barrier: Option<async_channel::Receiver<()>>,
    windows: HashMap<Uuid, BrowserSlot>,
    host: Option<BrowserHostSlot>,
    activation: Option<(Uuid, Uuid, Uuid, Instant)>,
    paths: HashMap<PathBuf, Uuid>,
    reads: HashMap<Uuid, PendingRead>,
    picker: Option<(Uuid, Picker)>,
    entries: HashMap<Uuid, PendingEntry>,
    previews: HashMap<Uuid, super::preview::PreviewState>,
    preview_entries: HashMap<Uuid, super::preview::PreviewRequest>,
    preview_prompts: HashMap<Uuid, entry::PreviewPrompt>,
    shells: HashMap<Uuid, (Uuid, u64, Lease)>,
    temp_directory_picker: Option<(Uuid, TempDirectoryReply)>,
    ui_state: Option<UiStateStore>,
    passwords: HashMap<Uuid, nxtools_protocol::archive_password::ArchivePasswordPrompt>,
    password_dialogs: HashMap<Uuid, password::PasswordDialog>,
    jobs: jobs::Jobs,
    integrations: HashMap<Uuid, shell_integration::PendingIntegration>,
    _poll: Option<Task<()>>,
    _drain: Task<()>,
    _ui_changes: Option<Task<()>>,
}
impl ArchiveApplication {
    fn install_ui_state(&mut self, store: UiStateStore, cx: &mut Context<Self>) {
        let changes = store.changes();
        self.ui_state = Some(store);
        self._ui_changes = Some(cx.spawn(async move |this, cx| {
            while changes.recv().await.is_ok() {
                if this
                    .update(cx, |app, cx| {
                        if let Some(store) = &app.ui_state {
                            let limit = if app.settings.remember_recent {
                                usize::from(app.settings.recent_limit)
                            } else {
                                0
                            };
                            if store.current_snapshot().recent.len() > limit {
                                let _ = store.update_recent(|recent| recent.truncate(limit));
                            }
                        }
                        app.publish_recent(cx);
                    })
                    .is_err()
                {
                    break;
                }
            }
        }));
    }
    fn publish_saved_settings(&mut self, saved: SavedArchiveSettings, cx: &mut Context<Self>) {
        if saved.order <= self.settings_save_order {
            return;
        }
        self.settings_save_order = saved.order;
        self.apply_saved_settings(saved.settings, cx);
    }
    fn apply_saved_settings(
        &mut self,
        settings: nxtools_core::archive::ArchiveSettings,
        cx: &mut Context<Self>,
    ) {
        if self.settings == settings {
            return;
        }
        self.settings = settings.clone();
        if let Some(store) = &self.ui_state {
            let limit = if settings.remember_recent {
                usize::from(settings.recent_limit)
            } else {
                0
            };
            let _ = store.update_recent(|recent| recent.truncate(limit));
        }
        self.publish_recent(cx);
        for slot in self.windows.values() {
            let view = slot.view.clone();
            let window = slot.window;
            let settings = settings.clone();
            cx.defer(move |cx| {
                let _ = window.update(cx, |_, window, cx| {
                    let _ = view.update(cx, |view, cx| view.apply_settings(settings, window, cx));
                });
            });
        }
    }
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
            settings_save_order: 0,
            config_path: None,
            #[cfg(test)]
            remember_publication_barrier: None,
            #[cfg(test)]
            preview_decode_barrier: None,
            windows: HashMap::new(),
            host: None,
            activation: None,
            paths: HashMap::new(),
            reads: HashMap::new(),
            picker: None,
            entries: HashMap::new(),
            previews: HashMap::new(),
            preview_entries: HashMap::new(),
            preview_prompts: HashMap::new(),
            shells: HashMap::new(),
            temp_directory_picker: None,
            ui_state: None,
            password_dialogs: HashMap::new(),
            passwords: HashMap::new(),
            jobs: jobs::Jobs::default(),
            integrations: HashMap::new(),
            _poll: poll,
            _drain: drain,
            _ui_changes: None,
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
                                            .title("无法打开压缩包")
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
        if let Some(host_id) = self
            .host
            .as_ref()
            .filter(|host| host.view.upgrade().is_none())
            .map(|host| host.id)
        {
            self.retire_host(host_id, cx);
        }
        if let Some(id) = self.browser_id(&path) {
            for lease in leases {
                self.release(lease);
            }
            self.activate_browser(id, cx);
            return;
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
        if self.host.is_none() {
            let host_id = Uuid::new_v4();
            let default_bounds = Bounds::centered(None, size(px(960.0), px(640.0)), cx);
            let bounds =
                restored
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
            let captured = Rc::new(std::cell::RefCell::new(None));
            let capture = captured.clone();
            let closing_owner = owner.clone();
            let opened = cx.open_window(
                WindowOptions {
                    window_bounds: Some(bounds),
                    window_min_size: Some(size(px(720.0), px(480.0))),
                    app_id: Some("NxTools.Archive".into()),
                    ..crate::ui::components::titlebar::caption_window_options()
                },
                move |window, cx| {
                    let host = cx.new(|_| BrowserHost::new(host_id, closing_owner.clone()));
                    let closing_host = host.downgrade();
                    window.on_window_should_close(cx, move |window, cx| {
                        let states = closing_host
                            .upgrade()
                            .map(|host| {
                                let tabs = host
                                    .read(cx)
                                    .tabs
                                    .iter()
                                    .map(|(_, view)| view.downgrade())
                                    .collect::<Vec<_>>();
                                tabs.into_iter()
                                    .filter_map(|view| {
                                        view.update(cx, |view, _| {
                                            let state = view.snapshot(window);
                                            view.final_snapshot = Some(state.clone());
                                            view.invalidate_drag();
                                            state
                                        })
                                        .ok()
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default();
                        let owner = closing_owner.clone();
                        cx.defer(move |cx| {
                            let _ = owner.update(cx, |app, cx| {
                                for state in states {
                                    app.save_ui_state(state);
                                }
                                app.retire_host(host_id, cx);
                            });
                        });
                        true
                    });
                    *capture.borrow_mut() = Some(host.downgrade());
                    cx.new(|cx| gpui_kit::base::Root::new(host, window, cx))
                },
            );
            if let (Ok(window), Some(view)) = (opened, captured.borrow_mut().take()) {
                self.host = Some(BrowserHostSlot {
                    id: host_id,
                    view,
                    window: window.into(),
                    password_recovery: password::PasswordRecovery::default(),
                });
            }
        }
        let Some(host) = &self.host else {
            for lease in leases {
                self.release(lease);
            }
            return;
        };
        let host_view = host.view.clone();
        let window = host.window;
        let path_for_view = path.clone();
        let settings = self.settings.clone();
        let recent = self.recent_paths().unwrap_or_default();
        let icon_platform = self.platform.clone();
        let created = window
            .update(cx, |_, window, cx| {
                let view = cx.new(|cx| {
                    let mut view = ArchiveBrowser::new_hosted(path_for_view, id, owner, window, cx);
                    view.hosted = true;
                    view.recent = recent;
                    view.configure_icons(icon_platform);
                    view.apply_settings(settings, window, cx);
                    if let Some(restored) = restored {
                        view.apply_restored_state(restored, window, cx);
                    }
                    view
                });
                let weak = view.downgrade();
                host_view
                    .update(cx, |host, cx| host.add(id, view, window, cx))
                    .ok()?;
                Some(weak)
            })
            .ok()
            .flatten();
        if let Some(view) = created {
            self.paths.insert(path.clone(), id);
            self.windows.insert(
                id,
                BrowserSlot {
                    path,
                    view,
                    window,
                    generation: 0,
                    browser: None,
                    depth,
                    leases,
                    preview_lease: None,
                },
            );
            tracing::info!(event="archive.browser.tab_opened", correlation_id=%id, tab_count=self.windows.len(), "archive tab opened");
            self.refresh_browser(id, default_codepage(self.settings.filename_encoding), cx);
            self.activate_browser(id, cx);
        } else {
            for lease in leases {
                self.release(lease);
            }
        }
    }
    fn browser_id(&self, path: &std::path::Path) -> Option<Uuid> {
        self.paths
            .iter()
            .find(|(candidate, id)| {
                self.windows.contains_key(id)
                    && (candidate.as_path() == path
                        || (cfg!(windows)
                            && candidate
                                .to_string_lossy()
                                .eq_ignore_ascii_case(&path.to_string_lossy())))
            })
            .map(|(_, id)| *id)
    }
    fn activate_browser(&mut self, id: Uuid, cx: &mut Context<Self>) {
        let Some(host) = &self.host else {
            return;
        };
        let host_id = host.id;
        let host_view = host.view.clone();
        let window = host.window;
        let request = Uuid::new_v4();
        self.activation = Some((request, id, host_id, Instant::now()));
        let owner = cx.entity().downgrade();
        cx.defer(move |cx| {
            let _ = owner.update(cx, |app, cx| {
                if app.activation.as_ref().is_none_or(|pending| pending.0 != request)
                    || app.host.as_ref().is_none_or(|host| host.id != host_id)
                    || !app.windows.contains_key(&id) { return; }
                let hwnd = window.update(cx, |_, window, cx| {
                    if !host_view.update(cx, |host, cx| host.activate(id, window, cx)).is_ok_and(|active| active) { return None; }
                    let hwnd = crate::ui::native_window_handle(window);
                    if hwnd.is_none() { window.activate_window(); }
                    hwnd
                }).ok().flatten();
                if let Some(window_handle) = hwnd {
                    tracing::info!(event="archive.browser.activation_started", correlation_id=%request, session_id=%id, host_id=%host_id, "archive host activation requested");
                    if app.platform.send_command_with_id(nxtools_protocol::PlatformCommand::ActivateWindow(
                        nxtools_protocol::ActivateWindowCommand { window_handle }), request).is_err() {
                        app.activation = None;
                        tracing::warn!(event="archive.browser.activation_finished", correlation_id=%request, outcome="not_submitted", "archive host activation unavailable");
                    }
                } else { app.activation = None; }
            });
        });
    }
    fn close_host(&mut self, host_id: Uuid, cx: &mut Context<Self>) {
        let Some(window) = self
            .host
            .as_ref()
            .filter(|host| host.id == host_id)
            .map(|host| host.window)
        else {
            return;
        };
        self.retire_host(host_id, cx);
        cx.defer(move |cx| {
            let _ = window.update(cx, |_, window, _| window.remove_window());
        });
    }
    fn retire_host(&mut self, host_id: Uuid, cx: &mut Context<Self>) {
        if self.host.as_ref().is_none_or(|host| host.id != host_id) {
            return;
        }
        self.host = None;
        self.activation = None;
        let ids = self.windows.keys().copied().collect::<Vec<_>>();
        for id in ids {
            self.retire_browser(id, cx);
        }
        tracing::info!(event="archive.browser.host_closed", correlation_id=%host_id, "archive host retired");
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
                view.recovery = None;
                view.generation = generation;
                view.invalidate_drag();
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
                format!("无法读取压缩包：{error}。请检查原生适配器和文件后重试。"),
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
            self.hide_password_dialog(id, cx);
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
                let drag_browser = slot.browser.clone().expect("completed browser");
                let drag_platform = self.platform.clone();
                let drag_depth = slot.depth;
                let drag_motw = self.settings.copy_motw;
                let drag_connection = read.operation.connection_generation();
                let recent_path =
                    (slot.depth == 0 && slot.leases.is_empty()).then(|| slot.path.clone());
                if let Some(path) = recent_path {
                    self.remember_recent(path, cx);
                }
                cx.defer(move |cx| {
                    let _ = window.update(cx, |_, window, cx| {
                        let _ = view.update(cx, |view, cx| {
                            if view.generation == read.generation {
                                view.read_id = None;
                                view.install(snapshot, read.entries, format, window, cx);
                                view.configure_drag(
                                    drag_platform,
                                    drag_browser.clone(),
                                    drag_connection,
                                    drag_depth,
                                    drag_motw,
                                );
                                view.bind_edit_context(&drag_browser, cx);
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
                if terminal.cleanup_complete
                    && matches!(
                        terminal.status,
                        ArchiveReadStatus::Failed | ArchiveReadStatus::Rejected
                    )
                {
                    self.inspect_browser_recovery(read.window_id, read.generation, cx);
                }
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
        self.retire_recovery(id);
        self.invalidate_space_source(id, cx);
        self.cancel_preview(id, cx);
        let prompts = self
            .passwords
            .values()
            .filter(|prompt| prompt.session_id == id)
            .map(|prompt| prompt.operation_id)
            .collect::<Vec<_>>();
        for operation in prompts {
            self.hide_password_dialog(operation, cx);
        }
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
                    view.recovery = None;
                    view.invalidate_drag();
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
        if self
            .previews
            .get(&id)
            .is_some_and(|state| state.running.is_none())
        {
            self.previews.remove(&id);
        }
        tracing::info!(event="archive.browser.retired",correlation_id=%id,"archive browser retired");
    }
    pub(crate) fn close_browser(&mut self, id: Uuid, cx: &mut Context<Self>) {
        if !self.windows.contains_key(&id) {
            return;
        }
        let host = self
            .host
            .as_ref()
            .map(|host| (host.id, host.view.clone(), host.window));
        self.retire_browser(id, cx);
        let last = self.windows.is_empty();
        if last {
            self.host = None;
        }
        if let Some((host_id, host, window)) = host {
            cx.defer(move |cx| {
                let _ = window.update(cx, |_, window, cx| {
                    let _ = host.update(cx, |host, cx| host.remove(id, window, cx));
                    if last { window.remove_window(); }
                });
                tracing::info!(event="archive.browser.tab_closed", correlation_id=%id, host_id=%host_id, last, "archive tab closed");
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
    fn recent_paths(&self) -> Result<Vec<PathBuf>, &'static str> {
        let store = self.ui_state.as_ref().ok_or("unavailable")?;
        match store.load_status() {
            UiStateLoad::Loading => Err("loading"),
            UiStateLoad::Preserved(_) => Err("preserved"),
            UiStateLoad::Ready(_) => Ok(if self.settings.remember_recent {
                store
                    .current_snapshot()
                    .recent
                    .into_iter()
                    .take(usize::from(self.settings.recent_limit))
                    .collect()
            } else {
                Vec::new()
            }),
        }
    }
    fn remember_recent(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if !self.settings.remember_recent || self.settings.recent_limit == 0 {
            return;
        }
        let limit = usize::from(self.settings.recent_limit);
        if let Some(store) = &self.ui_state {
            let correlation_id = Uuid::new_v4();
            let started = Instant::now();
            tracing::info!(event="archive.recent.record_started", %correlation_id, "recent archive record accepted");
            let result = store.update_recent(|recent| {
                recent.retain(|old| {
                    !old.to_string_lossy()
                        .eq_ignore_ascii_case(&path.to_string_lossy())
                });
                recent.insert(0, path);
                recent.truncate(limit);
            });
            if result.is_ok() {
                self.publish_recent(cx);
                if let Ok(receipt) = store.flush() {
                    cx.spawn(async move |_, _| {
                        let saved = receipt.recv().await == Ok(true);
                        tracing::info!(event="archive.recent.record_finished", %correlation_id, success=saved, elapsed_ms=started.elapsed().as_millis() as u64, "recent archive record finished");
                    }).detach();
                } else {
                    tracing::warn!(event="archive.recent.record_finished", %correlation_id, success=false, elapsed_ms=started.elapsed().as_millis() as u64, "recent archive record receipt unavailable");
                }
            }
            if result.is_err() {
                tracing::warn!(event="archive.recent.record_finished", %correlation_id, success=false, elapsed_ms=started.elapsed().as_millis() as u64, "recent archive record rejected");
            }
        }
    }
    fn clear_recent_paths(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Result<async_channel::Receiver<bool>, &'static str> {
        let store = self.ui_state.as_ref().ok_or("unavailable")?;
        store.update_recent(Vec::clear).map_err(|_| "preserved")?;
        let receipt = store.flush().map_err(|_| "write_failed")?;
        self.publish_recent(cx);
        Ok(receipt)
    }
    fn publish_recent(&self, cx: &mut Context<Self>) {
        cx.notify();
        let recent = self.recent_paths().unwrap_or_default();
        for slot in self.windows.values() {
            let view = slot.view.clone();
            let recent = recent.clone();
            cx.defer(move |cx| {
                let _ = view.update(cx, |view, cx| {
                    view.recent = recent;
                    cx.notify();
                });
            });
        }
    }
    pub(crate) fn handle_drop(
        &mut self,
        id: Uuid,
        paths: Vec<PathBuf>,
        source: AnyWindowHandle,
        cx: &mut Context<Self>,
    ) {
        if paths.is_empty()
            || paths.len() > 32
            || paths.iter().any(|path| {
                path.to_str()
                    .is_none_or(|path| !nxtools_protocol::archive_job::valid_job_path(path))
            })
        {
            self.incoming_notice(
                id,
                "一次最多拖入 32 个有效来源；未截断文件列表。".into(),
                cx,
            );
            return;
        }
        let frozen_target = self.drop_add_target(id, &paths, cx);
        let Some(slot) = self.windows.get(&id) else {
            return;
        };
        let generation = slot.generation;
        let frozen_target = Rc::new(std::cell::RefCell::new(frozen_target));
        let add_reason = slot
            .view
            .read_with(cx, |view, _| view.edit_reason())
            .unwrap_or(Some("原压缩包已关闭".into()))
            .or_else(|| (paths.len() > 31).then(|| "Add 最多接受 31 个来源".into()))
            .or_else(|| {
                frozen_target
                    .borrow()
                    .is_none()
                    .then(|| "当前来源或连接已失效，请重新打开压缩包。".into())
            });
        let eligible = add_reason.is_none();
        let owner = cx.entity().downgrade();
        cx.defer(move |cx| {
            let _ = source.update(cx, |_, window, cx| {
                use gpui_kit::ParentElement;
                use gpui_kit::component::{
                    Sizable, WindowExt,
                    button::Button,
                    dialog::{DialogAction, DialogClose, DialogFooter},
                };
                let owner_open = owner.clone();
                let owner_add = owner.clone();
                let owner_new = owner.clone();
                let new_paths = paths
                    .iter()
                    .map(|path| path.to_str().expect("validated path").to_owned())
                    .collect::<Vec<_>>();
                let open_path = paths[0].clone();
                window.open_dialog(cx, move |dialog, _, _| {
                    let owner_open = owner_open.clone();
                    let owner_add = owner_add.clone();
                    let owner_new = owner_new.clone();
                    let new_paths = new_paths.clone();
                    let open_path = open_path.clone();
                    let frozen_target = frozen_target.clone();
                    let closing_target = frozen_target.clone();
                    dialog
                        .title("拖入文件")
                        .child(add_reason.as_ref().map_or_else(
                            || "选择打开首项、使用全部来源新建，或添加到当前压缩包。".to_owned(),
                            |reason| format!("可打开首项或使用全部来源新建。无法添加：{reason}"),
                        ))
                        .footer(
                            DialogFooter::new()
                                .child(DialogClose::new().trigger(|button| button.label("取消")))
                                .child(
                                    DialogAction::new().child(
                                        Button::new("archive-drop-open")
                                            .label("打开首项")
                                            .outline()
                                            .with_size(gpui_kit::component::Size::Medium),
                                    ),
                                )
                                .child(
                                    Button::new("archive-drop-new")
                                        .label("全部来源新建")
                                        .outline()
                                        .with_size(gpui_kit::component::Size::Medium)
                                        .on_click(move |_, window, cx| {
                                            let owner = owner_new.clone();
                                            let paths = new_paths.clone();
                                            window.close_dialog(cx);
                                            cx.defer(move |cx| {
                                                let _ = owner.update(cx, |app, cx| {
                                                    if app.windows.get(&id).is_some_and(|slot| {
                                                        slot.generation == generation
                                                    }) {
                                                        app.open_create_paths(paths, cx);
                                                    }
                                                });
                                            });
                                        }),
                                )
                                .child(
                                    Button::new("archive-drop-add")
                                        .label("添加到当前压缩包")
                                        .outline()
                                        .with_size(gpui_kit::component::Size::Medium)
                                        .disabled(!eligible)
                                        .on_click(move |_, window, cx| {
                                            let owner = owner_add.clone();
                                            let Some(target) = frozen_target.borrow_mut().take()
                                            else {
                                                return;
                                            };
                                            window.close_dialog(cx);
                                            cx.defer(move |cx| {
                                                let _ = owner
                                                    .update(cx, |app, cx| app.add_drop(target, cx));
                                            });
                                        }),
                                ),
                        )
                        .on_close(move |_, _, _| {
                            closing_target.borrow_mut().take();
                        })
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
    fn drop_add_target(&self, id: Uuid, paths: &[PathBuf], cx: &App) -> Option<DropAddTarget> {
        let slot = self.windows.get(&id)?;
        let connection = self.platform.connection_generation()?;
        let browser = slot.browser.clone()?;
        let (folder, codepage) = slot
            .view
            .read_with(cx, |view, _| {
                let model = view.model.as_ref()?;
                (view.edit_reason().is_none()
                    && view.preview_connection() == connection
                    && model.snapshot == browser.snapshot_id)
                    .then(|| (model.folder.clone(), view.codepage))
            })
            .ok()
            .flatten()?;
        let draft = crate::archive::draft::ModifyDraft {
            source: slot.path.clone(),
            browser,
            action: nxtools_protocol::archive_job::ArchiveJobModifyAction::Add,
            ordinals: vec![],
            value: (!folder.is_empty()).then(|| format!("{folder}/")),
            source_prefix: None,
            added: paths
                .iter()
                .map(|path| path.to_str().map(str::to_owned))
                .collect::<Option<Vec<_>>>()?,
            codepage,
        };
        let request = draft.request().ok()?;
        Some(DropAddTarget {
            origin: jobs::EditFormOrigin {
                browser: (id, slot.generation, draft.browser.snapshot_id),
                connection,
            },
            request,
        })
    }

    fn add_drop(&mut self, target: DropAddTarget, cx: &mut Context<Self>) {
        let id = target.origin.browser.0;
        if !self.tab_is_active(id, cx)
            || !self.edit_request_current(&target.request, target.origin, cx)
        {
            self.incoming_notice(
                id,
                "原压缩包、目录或连接已变化，请在目标目录重新拖入文件。".into(),
                cx,
            );
            return;
        }
        if let Err(message) = self.submit_drop_add(target.request, target.origin, cx) {
            self.incoming_notice(id, message, cx);
        }
    }

    fn incoming_notice(&self, id: Uuid, message: String, cx: &mut Context<Self>) {
        tracing::warn!(event = "archive.drop_add.refused", browser_id = %id, "incoming Add refused before job admission");
        let Some(center) = crate::notifications::current(cx) else {
            return;
        };
        let mut notice = crate::notifications::AppNotification::new(
            format!("archive-drop-{id}"),
            crate::notifications::NotificationLevel::Error,
            "添加文件失败",
            message,
        );
        if let Some(slot) = self.windows.get(&id) {
            notice = notice.in_window(slot.window);
        }
        cx.defer(move |cx| center.update(cx, |center, cx| center.push(notice, cx)));
    }
    fn notification(&mut self, notification: PlatformNotification, cx: &mut Context<Self>) {
        if let PlatformNotification::Event(
            nxtools_protocol::PlatformEvent::ArchiveVirtualDragProgress(value),
        ) = &notification
        {
            self.drag_progress_event((**value).clone(), cx);
            return;
        }
        if self.job_notification(&notification, cx) {
            return;
        }
        if let PlatformNotification::Event(nxtools_protocol::PlatformEvent::ArchivePassword(
            prompt,
        )) = &notification
        {
            if !self.preview_password_prompt(prompt.clone(), cx) {
                self.password_prompt(prompt.clone(), cx);
            }
            return;
        }
        if matches!(notification, PlatformNotification::Disconnected { .. }) {
            self.drag_progress_disconnected(cx);
            self.activation = None;
            self.previews_disconnected(cx);
            self.picker = None;
            if let Some((_, reply)) = self.temp_directory_picker.take() {
                cx.defer(move |cx| reply(Err("disconnected"), cx));
            }
            self.reads.clear();
            self.entries.clear();
            self.shells.clear();
            self.passwords.clear();
            let dialogs = self.password_dialogs.keys().copied().collect::<Vec<_>>();
            for operation in dialogs {
                self.hide_password_dialog(operation, cx);
            }
            self.integrations_disconnected(cx);
            let ids = self.windows.keys().copied().collect::<Vec<_>>();
            for id in ids {
                if let Some(slot) = self.windows.get_mut(&id) {
                    slot.browser = None;
                    slot.leases.clear();
                    let view = slot.view.clone();
                    cx.defer(move |cx| {
                        let _ = view.update(cx, |view, cx| {
                            view.icons_disconnected();
                            view.drag_disconnected(cx);
                        });
                    });
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
            if self
                .activation
                .as_ref()
                .is_some_and(|pending| pending.0 == correlation_id)
            {
                let (_, session, host, started) =
                    self.activation.take().expect("matching activation");
                tracing::info!(event="archive.browser.activation_finished", correlation_id=%correlation_id, session_id=%session, host_id=%host,
                    success=result.success, native_error_code=?result.native_error_code, elapsed_ms=started.elapsed().as_millis() as u64, "archive host activation finished");
                return;
            }
            if self
                .temp_directory_picker
                .as_ref()
                .is_some_and(|(id, _)| *id == correlation_id)
            {
                let (_, reply) = self
                    .temp_directory_picker
                    .take()
                    .expect("matching directory request");
                let picked = if result.success {
                    Ok(result.data.filter(|value| !value.is_empty()))
                } else {
                    Err("native_failure")
                };
                cx.defer(move |cx| reply(picked, cx));
                return;
            }
            let icon_view = self
                .windows
                .values()
                .find(|slot| {
                    slot.view
                        .read_with(cx, |view, _| view.icon_matches(correlation_id))
                        .unwrap_or(false)
                })
                .map(|slot| slot.view.clone());
            if let Some(view) = icon_view {
                cx.defer(move |cx| {
                    let _ =
                        view.update(cx, |view, cx| view.icon_result(correlation_id, &result, cx));
                });
                return;
            }
            let authorization_view = self
                .windows
                .values()
                .find(|slot| {
                    slot.view
                        .read_with(cx, |view, _| view.authorization_matches(correlation_id))
                        .unwrap_or(false)
                })
                .map(|slot| slot.view.clone());
            if let Some(view) = authorization_view {
                cx.defer(move |cx| {
                    let _ = view.update(cx, |view, cx| {
                        view.authorization_result(correlation_id, &result, cx)
                    });
                });
                return;
            }
            let matching_drag = self
                .windows
                .values()
                .find(|slot| {
                    slot.view
                        .read_with(cx, |view, _| view.drag_matches(correlation_id))
                        .unwrap_or(false)
                })
                .map(|slot| slot.view.clone());
            if let Some(view) = matching_drag {
                let owner = cx.entity().downgrade();
                cx.defer(move |cx| {
                    let located = view
                        .update(cx, |view, cx| view.drag_result(correlation_id, &result, cx))
                        .ok()
                        .flatten();
                    cx.defer(move |cx| {
                        let _ = owner.update(cx, |app, cx| {
                            if let Some(drop) = located {
                                app.extract_dropped(drop, cx);
                            }
                            app.flush_job_locations(cx)
                        });
                    });
                });
                return;
            }
            if self.entry_result(correlation_id, &result, cx)
                || self.shell_result(correlation_id, &result, cx)
                || self.integration_result(correlation_id, &result, cx)
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
                    .title("无法选择压缩包")
                    .child("原生文件选择器不可用或连接已断开，请重试。")
                    .footer(DialogClose::new().trigger(|button| button.label("关闭")))
            });
        });
    });
}
