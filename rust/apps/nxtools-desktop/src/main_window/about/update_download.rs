use super::*;
use gpui_kit::App;

#[derive(Clone)]
struct Installer {
    name: String,
    url: String,
    size: Option<u64>,
    key: Uuid,
}

pub(super) struct ReleaseCheck {
    pub(super) status: UpdateStatus,
    installer: Option<Installer>,
}

impl ReleaseCheck {
    pub(super) fn status(status: UpdateStatus) -> Self {
        Self {
            status,
            installer: None,
        }
    }
}

#[derive(serde::Deserialize)]
pub(super) struct Release {
    tag_name: String,
    #[serde(default)]
    assets: Vec<ReleaseAsset>,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
}

#[derive(serde::Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    size: u64,
}

pub(super) fn checked_release(
    current: &str,
    target: &str,
    release: Release,
) -> Result<ReleaseCheck, UpdateFailure> {
    if release.draft || release.prerelease {
        return Err(UpdateFailure::InvalidRelease);
    }
    let status =
        compare_release(current, &release.tag_name).ok_or(UpdateFailure::InvalidRelease)?;
    if !matches!(status, UpdateStatus::Newer) {
        return Ok(ReleaseCheck::status(status));
    }
    if target != "x86_64-pc-windows-msvc" {
        return Err(UpdateFailure::InstallerUnavailable);
    }
    let version = release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name);
    let name = format!("NxTools.Next-setup-{version}.exe");
    let mut matches = release
        .assets
        .into_iter()
        .filter(|asset| asset.name == name);
    let asset = matches.next().ok_or(UpdateFailure::InstallerUnavailable)?;
    if matches.next().is_some() {
        return Err(UpdateFailure::InstallerUnavailable);
    }
    let url = reqwest::Url::parse(&asset.browser_download_url)
        .map_err(|_| UpdateFailure::InvalidRelease)?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(UpdateFailure::InvalidRelease);
    }
    // Stable release identity reuses durable engine idempotency across dialog reopen.
    let mut key = *b"NXUP\0\0\0\0\0\0\0\0\0\0\0\0";
    for (index, part) in version.split('.').enumerate() {
        let value: u32 = part.parse().map_err(|_| UpdateFailure::InvalidRelease)?;
        key[4 + index * 4..8 + index * 4].copy_from_slice(&value.to_be_bytes());
    }
    Ok(ReleaseCheck {
        status,
        installer: Some(Installer {
            name: asset.name,
            url: asset.browser_download_url,
            size: (asset.size > 0).then_some(asset.size),
            key: Uuid::from_bytes(key),
        }),
    })
}

impl About {
    pub(super) fn observe_close(view: &Entity<Self>, window: &mut Window, cx: &mut App) {
        let owner = view.downgrade();
        window.on_window_should_close(cx, move |_, cx| {
            let _ = owner.update(cx, |view, _| view.retire_update());
            true
        });
    }

    pub(super) fn retire_update(&mut self) {
        self.closed = true;
        self.update_request = None;
        self.update_task = None;
    }

    pub(super) fn receive_update(
        &mut self,
        id: Uuid,
        started: Instant,
        result: ReleaseCheck,
        cx: &mut Context<Self>,
    ) {
        if self.closed || self.update_request != Some(id) {
            return;
        }
        self.update = result.status;
        let Some(installer) = result.installer else {
            self.update_request = None;
            cx.notify();
            return;
        };
        self.update = UpdateStatus::Submitting;
        let owner = cx.entity().downgrade();
        let downloads = self.downloads.clone();
        cx.defer(move |cx| {
            if !owner.update(cx, |view, _| !view.closed && view.update_request == Some(id)).unwrap_or(false) {
                return;
            }
            info!(event = "desktop.about.download.start", correlation_id = %id);
            let outcome = downloads.and_then(|downloads| {
                downloads.update(cx, |downloads, cx| {
                    downloads.submit_installer_download(installer.url, installer.name, installer.size, installer.key, cx)
                }).ok()
            }).unwrap_or_else(|| Err("download_owner_unavailable".into()));
            let accepted = outcome.is_ok();
            info!(event = "desktop.about.download.finished", correlation_id = %id, elapsed_ms = started.elapsed().as_millis() as u64, outcome = if accepted { "accepted" } else { "rejected" }, task_id = ?outcome.as_ref().ok().map(|task| task.as_uuid()));
            let _ = owner.update(cx, |view, cx| {
                if !view.closed && view.update_request == Some(id) {
                    view.update_request = None;
                    view.update = if accepted { UpdateStatus::Queued } else { UpdateStatus::Failed(UpdateFailure::Download) };
                    cx.notify();
                }
            });
        });
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release() -> Release {
        serde_json::from_value(serde_json::json!({
            "tag_name":"v0.7.10",
            "assets":[
                {"name":"NxTools.Next-0.7.10.zip","browser_download_url":"https://example.invalid/portable.zip","size":12},
                {"name":"NxTools.Next-setup-0.7.10.exe","browser_download_url":"https://example.invalid/setup.exe","size":24}
            ]
        })).expect("release fixture")
    }

    #[test]
    fn about_update_selects_only_the_supported_newer_installer() {
        let check =
            checked_release("0.7.9", "x86_64-pc-windows-msvc", release()).expect("newer installer");
        assert!(matches!(check.status, UpdateStatus::Newer));
        let installer = check.installer.expect("installer");
        assert_eq!(installer.name, "NxTools.Next-setup-0.7.10.exe");
        assert_eq!(installer.url, "https://example.invalid/setup.exe");
        assert_eq!(installer.size, Some(24));
        let again =
            checked_release("0.7.9", "x86_64-pc-windows-msvc", release()).expect("same release");
        assert_eq!(installer.key, again.installer.expect("same installer").key);
        let current =
            checked_release("0.7.10", "x86_64-pc-windows-msvc", release()).expect("current");
        assert!(matches!(current.status, UpdateStatus::Current));
        assert!(current.installer.is_none());
    }

    #[test]
    fn about_update_reports_missing_ambiguous_and_invalid_installers() {
        for mode in 0..5 {
            let mut input = release();
            match mode {
                0 => input.assets.clear(),
                1 => input.assets.push(ReleaseAsset {
                    name: input.assets[1].name.clone(),
                    browser_download_url: input.assets[1].browser_download_url.clone(),
                    size: 24,
                }),
                2 => input.assets[1].browser_download_url = "file:///C:/setup.exe".into(),
                3 => input.tag_name = "nightly".into(),
                _ => input.prerelease = true,
            }
            assert!(checked_release("0.7.9", "x86_64-pc-windows-msvc", input).is_err());
        }
        assert!(matches!(
            checked_release("0.7.9", "aarch64-pc-windows-msvc", release()),
            Err(UpdateFailure::InstallerUnavailable)
        ));
    }

    fn application(
        cx: &mut gpui_kit::TestAppContext,
    ) -> (
        Entity<crate::downloads::DownloadApplication>,
        tempfile::TempDir,
        nxtools_download::DownloadEngine,
    ) {
        let root = tempfile::tempdir().expect("owned update download directory");
        let database = root.path().join("downloads.sqlite3");
        {
            let store =
                nxtools_download::SqliteDownloadStore::open(&database).expect("fixture store");
            store
                .set_queue_running("default", false)
                .expect("pause fixture queue before admission");
        }
        let engine = nxtools_download::DownloadEngine::open(&database).expect("owned engine");
        let (platform, _endpoint) = nxtools_ipc::platform_bridge(16);
        let notifications =
            cx.update(|cx| cx.new(|_| crate::notifications::NotificationCenter::default()));
        let mut config = nxtools_core::AppConfig::default();
        config.download.automatic_categories_enabled = false;
        config.download.show_progress_window = false;
        config.download.show_completion_window = false;
        let application = cx.update(|cx| {
            cx.new(|cx| {
                crate::downloads::DownloadApplication::new(
                    engine.clone(),
                    std::rc::Rc::new(platform),
                    notifications,
                    crate::downloads::DownloadRuntimeContext {
                        fallback_default_directory: root.path().to_path_buf(),
                        config,
                        config_path: root.path().join("config.json"),
                        log_directory: root.path().to_path_buf(),
                        database_path: database,
                        ffmpeg_install_directory: root.path().join("ffmpeg"),
                    },
                    cx,
                )
            })
        });
        (application, root, engine)
    }

    #[gpui_kit::test]
    fn about_update_defers_real_engine_submission_and_deduplicates_repeated_checks(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let (downloads, root, engine) = application(cx);
        let about = cx.update(|cx| {
            cx.new(|cx| {
                let mut view = About::new(Uuid::new_v4(), cx);
                view.downloads = Some(downloads.downgrade());
                view
            })
        });
        for count in 0..2 {
            cx.update(|cx| {
                about.update(cx, |view, cx| {
                    let id = Uuid::new_v4();
                    view.update_request = Some(id);
                    view.receive_update(
                        id,
                        Instant::now(),
                        checked_release("0.7.9", "x86_64-pc-windows-msvc", release())
                            .expect("release"),
                        cx,
                    );
                    assert!(matches!(view.update, UpdateStatus::Submitting));
                    assert_eq!(
                        downloads.read(cx).tasks().len(),
                        count.min(1),
                        "no cross-owner submission during the leased About callback"
                    );
                })
            });
            cx.run_until_parked();
            cx.read(|cx| {
                assert!(matches!(about.read(cx).update, UpdateStatus::Queued));
                let tasks = downloads.read(cx).tasks();
                assert_eq!(tasks.len(), 1);
                let task = engine
                    .task(tasks[0].task.id())
                    .expect("durable task lookup")
                    .expect("accepted task");
                assert!(
                    task.queue_eligible,
                    "StartNow remains eligible in the intentionally paused fixture queue"
                );
                assert_eq!(
                    task.task.request().source_url,
                    "https://example.invalid/setup.exe"
                );
                assert_eq!(
                    task.task.request().file_name.as_deref(),
                    Some("NxTools.Next-setup-0.7.10.exe")
                );
                assert_eq!(task.task.request().save_directory, root.path());
                assert_eq!(task.task.request().connection_limit, Some(16));
                assert_eq!(task.task.request().expected_total_bytes, Some(24));
            });
        }
        drop(about);
        drop(downloads);
        drop(engine);
        cx.update(|_| {});
        cx.run_until_parked();
        root.close()
            .expect("release owned fixture database and files");
    }

    #[gpui_kit::test]
    fn about_update_retires_stale_closed_and_unavailable_download_owners(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let (downloads, root, engine) = application(cx);
        let about = cx.update(|cx| {
            cx.new(|cx| {
                let mut view = About::new(Uuid::new_v4(), cx);
                view.downloads = Some(downloads.downgrade());
                view
            })
        });
        cx.update(|cx| {
            about.update(cx, |view, cx| {
                view.update_request = Some(Uuid::new_v4());
                view.receive_update(
                    Uuid::new_v4(),
                    Instant::now(),
                    checked_release("0.7.9", "x86_64-pc-windows-msvc", release())
                        .expect("stale release"),
                    cx,
                );
                assert!(matches!(view.update, UpdateStatus::Idle));
                let id = view.update_request.expect("current request");
                view.receive_update(
                    id,
                    Instant::now(),
                    checked_release("0.7.9", "x86_64-pc-windows-msvc", release())
                        .expect("current release"),
                    cx,
                );
                view.retire_update();
            })
        });
        cx.run_until_parked();
        assert!(cx.read(|cx| downloads.read(cx).tasks().is_empty()));
        cx.update(|cx| {
            about.update(cx, |view, cx| {
                let id = Uuid::new_v4();
                view.update_request = Some(id);
                view.receive_update(
                    id,
                    Instant::now(),
                    checked_release("0.7.9", "x86_64-pc-windows-msvc", release())
                        .expect("late release"),
                    cx,
                );
                assert!(view.closed);
            })
        });
        assert!(cx.read(|cx| downloads.read(cx).tasks().is_empty()));
        let unavailable = cx.update(|cx| cx.new(|cx| About::new(Uuid::new_v4(), cx)));
        cx.update(|cx| {
            unavailable.update(cx, |view, cx| {
                let id = Uuid::new_v4();
                view.update_request = Some(id);
                view.receive_update(
                    id,
                    Instant::now(),
                    checked_release("0.7.9", "x86_64-pc-windows-msvc", release()).expect("release"),
                    cx,
                );
            })
        });
        cx.run_until_parked();
        assert!(cx.read(|cx| matches!(
            unavailable.read(cx).update,
            UpdateStatus::Failed(UpdateFailure::Download)
        )));
        drop(unavailable);
        drop(about);
        drop(downloads);
        drop(engine);
        cx.update(|_| {});
        cx.run_until_parked();
        root.close()
            .expect("release owned fixture database and files");
    }
}
