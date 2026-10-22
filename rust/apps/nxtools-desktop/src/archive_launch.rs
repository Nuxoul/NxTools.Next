//! Startup-only archive ingress. ArchiveApplication remains the only UI/job owner.
use anyhow::Result;
use gpui_kit::{App, Global};
use nxtools_ipc::desktop_archive_intent::ArchiveIntentReceiver;
pub(crate) use nxtools_ipc::desktop_archive_intent::LaunchCompletion;
use nxtools_protocol::desktop_archive_intent::{
    ArchiveExtractMode, ArchiveIntentAction, DesktopArchiveIntent, LaunchFailure, VERSION,
};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;
use uuid::Uuid;

type Handler = dyn Fn(DesktopArchiveIntent, LaunchCompletion, &mut App);
struct LaunchHandler(Rc<Handler>);
impl Global for LaunchHandler {}

pub(crate) fn register_handler(
    handler: impl Fn(DesktopArchiveIntent, LaunchCompletion, &mut App) + 'static,
    cx: &mut App,
) {
    cx.set_global(LaunchHandler(Rc::new(handler)));
}

pub(crate) fn start(
    receiver: ArchiveIntentReceiver,
    bridge: Rc<nxtools_ipc::PlatformBridge>,
    cx: &mut App,
) {
    cx.spawn(async move |cx| {
        loop {
            receiver.expire();
            cx.update(|cx| {
                let handler = cx
                    .try_global::<LaunchHandler>()
                    .map(|handler| handler.0.clone());
                if bridge.is_connected()
                    && let Some(handler) = handler
                {
                    // One bounded admission queue, original GPUI application turn/executor.
                    // Always defer across owner boundaries, including immediate/cache-hit paths.
                    while let Some((intent, completion)) = receiver.try_next() {
                        let handler = handler.clone();
                        cx.defer(move |cx| {
                            if completion.is_pending() {
                                handler(intent, completion, cx);
                            }
                        });
                    }
                }
            });
            cx.background_executor()
                .timer(Duration::from_millis(50))
                .await;
        }
    })
    .detach();
}

pub(crate) fn parse_arguments(
    arguments: impl IntoIterator<Item = OsString>,
    cwd: &Path,
) -> Result<Option<DesktopArchiveIntent>, LaunchFailure> {
    let arguments: Vec<OsString> = arguments.into_iter().collect();
    let mut paths = Vec::new();
    let mut action = None;
    let mut mode = None;
    let mut incompatible = false;
    let mut unknown_option = false;
    let mut index = 0;
    while index < arguments.len() {
        let argument = &arguments[index];
        match argument.to_str() {
            Some("--silent-start") => {}
            Some("--background" | "--browser-dialog") => {
                incompatible = true;
            }
            Some("--wait-for-instance-release-ms" | "--browser-correlation-id") => {
                index += 1;
                if index >= arguments.len() {
                    return Err(LaunchFailure::InvalidRequest);
                }
            }
            Some(
                "--archive-open" | "--archive-extract" | "--archive-compress" | "--archive-test",
            ) => {
                if action.is_some() || !paths.is_empty() {
                    return Err(LaunchFailure::InvalidRequest);
                }
                action = Some(match argument.to_str() {
                    Some("--archive-open") => ArchiveIntentAction::Open,
                    Some("--archive-extract") => ArchiveIntentAction::Extract,
                    Some("--archive-compress") => ArchiveIntentAction::Compress,
                    _ => ArchiveIntentAction::Test,
                });
                if action == Some(ArchiveIntentAction::Extract) {
                    index += 1;
                    mode = Some(
                        match arguments.get(index).and_then(|value| value.to_str()) {
                            Some("smart") => ArchiveExtractMode::Smart,
                            Some("here") => ArchiveExtractMode::Here,
                            Some("name") => ArchiveExtractMode::Name,
                            Some("dialog") => ArchiveExtractMode::Dialog,
                            _ => return Err(LaunchFailure::InvalidRequest),
                        },
                    );
                }
            }
            Some(value) if value.starts_with('-') => {
                unknown_option = true;
                if action.is_some()
                    || arguments.iter().any(|value| {
                        value
                            .to_str()
                            .is_some_and(|value| value.starts_with("--archive-"))
                    })
                {
                    return Err(LaunchFailure::InvalidRequest);
                }
            }
            _ => paths.push(PathBuf::from(argument)),
        }
        index += 1;
    }
    if unknown_option && (action.is_some() || paths.iter().any(|path| recognized_archive(path))) {
        return Err(LaunchFailure::InvalidRequest);
    }
    if action.is_none() {
        if paths.is_empty() || !paths.iter().any(|path| recognized_archive(path)) {
            return Ok(None);
        }
        if paths.len() != 1 || !recognized_archive(&paths[0]) {
            return Err(LaunchFailure::InvalidRequest);
        }
        action = Some(ArchiveIntentAction::Open);
    }
    // Explicit background/browser launches stay their original distinct policies; combining
    // them with an archive operation fails rather than silently discarding either request.
    if incompatible {
        return Err(LaunchFailure::InvalidRequest);
    }
    let mut resolved = Vec::with_capacity(paths.len());
    for path in paths {
        if path.as_os_str().is_empty() || (!path.is_absolute() && path.has_root()) {
            return Err(LaunchFailure::InvalidRequest);
        }
        // Drive-relative X:foo needs per-drive process state, not cwd.join; reject
        // explicitly rather than resolving it against a different receiver's drive.
        if !path.is_absolute()
            && path
                .components()
                .any(|part| matches!(part, std::path::Component::Prefix(_)))
        {
            return Err(LaunchFailure::InvalidRequest);
        }
        let path = if path.is_absolute() {
            path
        } else {
            cwd.join(path)
        };
        let Some(path) = path.to_str() else {
            return Err(LaunchFailure::InvalidRequest);
        };
        resolved.push(path.to_owned());
    }
    let intent = DesktopArchiveIntent {
        version: VERSION,
        request_id: Uuid::new_v4(),
        action: action.ok_or(LaunchFailure::InvalidRequest)?,
        paths: resolved,
        mode,
    };
    intent.validate()?;
    Ok(Some(intent))
}

pub(crate) fn recognized_archive(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            [
                "zip", "7z", "rar", "tar", "tgz", "txz", "gz", "xz", "bz2", "tbz", "tbz2", "zst",
                "tzst",
            ]
            .iter()
            .any(|known| extension.eq_ignore_ascii_case(known))
        })
}

// These injectable functions ARE the production startup decisions, not test-only policies.
pub(crate) fn route_before_startup(
    intent: Option<&DesktopArchiveIntent>,
    forward: impl FnOnce(&DesktopArchiveIntent) -> Result<bool>,
    ordinary: impl FnOnce() -> Result<bool>,
) -> Result<bool> {
    if let Some(intent) = intent {
        forward(intent)
    } else {
        ordinary()
    }
}
pub(crate) fn recover_instance_race(
    error: &anyhow::Error,
    intent: Option<&DesktopArchiveIntent>,
    forward: impl FnOnce(&DesktopArchiveIntent) -> Result<bool>,
) -> Result<bool> {
    if !error.is::<crate::runtime::DesktopInstanceConflict>() {
        return Ok(false);
    }
    if let Some(intent) = intent
        && !forward(intent)?
    {
        return Err(LaunchFailure::Disconnected.into());
    }
    Ok(true)
}
pub(crate) fn main_window_presentation(
    mode: crate::DesktopLaunchMode,
    archive_origin: bool,
) -> (bool, bool) {
    (
        !archive_origin && mode.main_window_focus(),
        !archive_origin && mode.main_window_show(),
    )
}

#[cfg(test)]
#[path = "archive_launch_tests.rs"]
mod tests;
