//! Explorer-only intake. Reader state, canonical tab identity and document reads remain with the reader.
use gpui_kit::App;
use nxtools_ipc::PlatformBridge;
use nxtools_ipc::desktop_markdown_intent::MarkdownIntentReceiver;
use nxtools_protocol::desktop_markdown_intent::{DesktopMarkdownIntent, LaunchFailure};
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;
use uuid::Uuid;

pub(crate) const OPEN_ARGUMENT: &str = "--markdown-open";
pub(crate) const CORRELATION_ARGUMENT: &str = "--markdown-correlation-id";

pub(crate) fn parse_arguments(
    arguments: impl IntoIterator<Item = OsString>,
    cwd: &Path,
) -> Result<Option<DesktopMarkdownIntent>, LaunchFailure> {
    let arguments: Vec<_> = arguments.into_iter().collect();
    if !arguments
        .iter()
        .any(|arg| arg == OPEN_ARGUMENT || arg == CORRELATION_ARGUMENT)
    {
        return Ok(None);
    }
    let (path, operation) = match arguments.as_slice() {
        [command, path] if command == OPEN_ARGUMENT => (path, Uuid::new_v4()),
        [command, path, correlation, id]
            if command == OPEN_ARGUMENT && correlation == CORRELATION_ARGUMENT =>
        {
            let operation = id
                .to_str()
                .and_then(|id| Uuid::parse_str(id).ok())
                .ok_or(LaunchFailure::InvalidRequest)?;
            (path, operation)
        }
        _ => return Err(LaunchFailure::InvalidRequest),
    };
    if path.is_empty() || path.to_str().is_none_or(|path| path.starts_with("--")) {
        return Err(LaunchFailure::InvalidRequest);
    }
    let path = Path::new(path);
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else if path.has_root() || matches!(path.components().next(), Some(Component::Prefix(_))) {
        // A rooted or drive-relative input depends on undocumented sender drive state.
        return Err(LaunchFailure::InvalidRequest);
    } else {
        cwd.join(path)
    };
    let path = absolute
        .into_os_string()
        .into_string()
        .map_err(|_| LaunchFailure::InvalidRequest)?;
    let intent = DesktopMarkdownIntent::new(path, operation);
    intent.validate()?;
    Ok(Some(intent))
}

pub(crate) fn route_before_startup(
    intent: Option<&DesktopMarkdownIntent>,
    forward: impl FnOnce(&DesktopMarkdownIntent) -> anyhow::Result<bool>,
    ordinary: impl FnOnce() -> anyhow::Result<bool>,
) -> anyhow::Result<bool> {
    if let Some(intent) = intent {
        forward(intent)
    } else {
        ordinary()
    }
}

pub(crate) fn recover_instance_race(
    error: &anyhow::Error,
    intent: &DesktopMarkdownIntent,
    forward: impl FnOnce(&DesktopMarkdownIntent) -> anyhow::Result<bool>,
) -> anyhow::Result<bool> {
    if !error.is::<crate::runtime::DesktopInstanceConflict>() {
        return Ok(false);
    }
    if !forward(intent)? {
        return Err(LaunchFailure::Disconnected.into());
    }
    Ok(true)
}

fn check_file(path: &Path) -> Result<(), (LaunchFailure, Option<i32>)> {
    match std::fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => Ok(()),
        Ok(_) => Err((LaunchFailure::NotFile, None)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Err((LaunchFailure::NotFile, error.raw_os_error()))
        }
        Err(error) => Err((LaunchFailure::FileUnavailable, error.raw_os_error())),
    }
}

pub(crate) fn start(receiver: MarkdownIntentReceiver, platform: Rc<PlatformBridge>, cx: &mut App) {
    cx.spawn(async move |cx| {
        while let Some(operation) = receiver.next().await {
            let path = PathBuf::from(&operation.intent.path);
            let file = path.clone();
            let background = cx.background_executor();
            let expiry = background.timer(operation.remaining());
            let expiring = operation.clone();
            let timeout = background.spawn(async move {
                expiry.await;
                let _ = expiring.is_pending();
            });
            // One metadata call at a time, including on slow or unreachable shared paths.
            // Expiry retires the operation; it does not spawn replacement I/O on a blocked share.
            let eligible = background.spawn(async move { check_file(&file) }).await;
            drop(timeout);
            if let Err((failure, native_code)) = eligible {
                operation.finish(Err(failure), native_code);
                continue;
            }
            let platform = Rc::clone(&platform);
            cx.update(|cx| {
                cx.defer(move |cx| {
                    if !operation.begin_dispatch() {
                        return;
                    }
                    let accepted = crate::tools::markdown_reader::open_external(
                        path,
                        operation.intent.request_id,
                        Some(platform),
                        cx,
                    );
                    // The reader owns the subsequent document-read and selection result.
                    operation.finish(
                        accepted.then_some(()).ok_or(LaunchFailure::NotAdmitted),
                        None,
                    );
                });
            });
        }
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(arguments: &[&str]) -> Result<Option<DesktopMarkdownIntent>, LaunchFailure> {
        parse_arguments(
            arguments.iter().map(OsString::from),
            Path::new(r"C:\Working Dir"),
        )
    }

    #[test]
    fn only_the_explicit_single_path_launch_is_markdown() {
        assert_eq!(parse(&["notes.md"]), Ok(None));
        assert_eq!(parse(&["--archive-open", r"C:\a.zip"]), Ok(None));
        let request = parse(&["--markdown-open", "中文 notes.Md"])
            .expect("parse")
            .expect("intent");
        assert_eq!(request.path, r"C:\Working Dir\中文 notes.Md");
        assert!(!request.request_id.is_nil());
        let id = Uuid::new_v4();
        let redirected = parse(&[
            "--markdown-open",
            r"\\server\share\中文 notes.MARKDOWN",
            "--markdown-correlation-id",
            &id.to_string(),
        ])
        .expect("parse")
        .expect("intent");
        assert_eq!(redirected.request_id, id);
        for arguments in [
            &["--markdown-open"][..],
            &["--markdown-open", ""][..],
            &["--markdown-open", "--flag.md"][..],
            &["--markdown-open", "a.md", "b.md"][..],
            &["--markdown-open", "a.zip"][..],
            &[
                "--markdown-open",
                "a.md",
                "--markdown-correlation-id",
                "not-an-id",
            ][..],
            &["--markdown-correlation-id", "not-an-id"][..],
            &["--silent-start", "--markdown-open", "a.md"][..],
            &["--markdown-open", "a.md", "--archive-open", "a.zip"][..],
        ] {
            assert_eq!(parse(arguments), Err(LaunchFailure::InvalidRequest));
        }
    }

    #[test]
    fn warm_markdown_never_summons_and_conflict_retries_the_original_operation() {
        let intent = parse(&["--markdown-open", "a.md"])
            .expect("parse")
            .expect("intent");
        for warm in [false, true] {
            assert_eq!(
                route_before_startup(
                    Some(&intent),
                    |request| {
                        assert_eq!(request, &intent);
                        Ok(warm)
                    },
                    || panic!("Markdown must never summon the main window")
                )
                .expect("route"),
                warm
            );
        }
        let conflict = crate::runtime::DesktopInstanceConflict.into();
        assert!(
            recover_instance_race(&conflict, &intent, |request| {
                assert_eq!(request, &intent);
                Ok(true)
            })
            .expect("recovered")
        );
        let failed =
            recover_instance_race(&conflict, &intent, |_| Ok(false)).expect_err("winner was lost");
        assert_eq!(
            failed.downcast_ref::<LaunchFailure>(),
            Some(&LaunchFailure::Disconnected)
        );
        assert!(
            !recover_instance_race(&anyhow::anyhow!("ordinary failure"), &intent, |_| panic!(
                "not an instance conflict"
            ))
            .expect("not recovered")
        );
        assert_eq!(
            crate::startup_main_window_presentation(crate::DesktopLaunchMode::Normal, false, true),
            (false, false)
        );
        assert_eq!(
            crate::startup_main_window_presentation(crate::DesktopLaunchMode::Normal, false, false),
            (true, true)
        );
    }

    #[test]
    fn the_eligibility_worker_rejects_missing_and_directory_paths() {
        let fixture = tempfile::tempdir().expect("temp directory");
        let file = fixture.path().join("中文 notes.md");
        let directory = fixture.path().join("directory.markdown");
        std::fs::write(&file, "# Test").expect("write fixture");
        std::fs::create_dir(&directory).expect("make directory");
        assert_eq!(check_file(&file), Ok(()));
        assert_eq!(check_file(&directory), Err((LaunchFailure::NotFile, None)));
        assert!(matches!(
            check_file(&fixture.path().join("missing.md")),
            Err((LaunchFailure::NotFile, Some(_)))
        ));
    }
}
