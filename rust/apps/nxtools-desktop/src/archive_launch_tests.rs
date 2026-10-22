use super::*;
use std::cell::RefCell;

fn parse(values: &[&str]) -> Result<Option<DesktopArchiveIntent>, LaunchFailure> {
    parse_arguments(
        values.iter().map(|value| OsString::from(*value)),
        Path::new(r"C:\sender"),
    )
}

#[test]
fn actual_startup_branch_forwards_archive_before_any_ordinary_return() {
    let intent = parse(&["--archive-open", "a.zip"])
        .expect("parse")
        .expect("intent");
    let calls = RefCell::new(Vec::new());
    assert!(
        route_before_startup(
            Some(&intent),
            |_| {
                calls.borrow_mut().push("archive");
                Ok(true)
            },
            || {
                calls.borrow_mut().push("summon");
                Ok(true)
            }
        )
        .expect("handoff")
    );
    assert_eq!(*calls.borrow(), ["archive"]);
}

#[test]
fn actual_startup_branch_does_not_swallow_failed_handoff_or_race() {
    let intent = parse(&["--archive-test", "a.7z"])
        .expect("parse")
        .expect("intent");
    assert!(
        route_before_startup(
            Some(&intent),
            |_| Err(anyhow::anyhow!("typed failure")),
            || panic!("must not summon")
        )
        .is_err()
    );
    let conflict = anyhow::Error::new(crate::runtime::DesktopInstanceConflict);
    assert!(recover_instance_race(&conflict, Some(&intent), |_| Ok(true)).expect("race handoff"));
    assert!(recover_instance_race(&conflict, Some(&intent), |_| Ok(false)).is_err());
    assert!(
        !recover_instance_race(
            &anyhow::anyhow!("unrelated startup error"),
            Some(&intent),
            |_| panic!("not a conflict")
        )
        .expect("not conflict")
    );
}

#[test]
fn archive_parser_uses_sender_cwd_and_explicit_arity_and_modes() {
    let open = parse(&["--archive-open", "folder/a.zip"])
        .expect("parse")
        .expect("intent");
    assert_eq!(
        Path::new(&open.paths[0]),
        Path::new(r"C:\sender\folder/a.zip")
    );
    for mode in ["smart", "here", "name", "dialog"] {
        let extract = parse(&["--archive-extract", mode, "a.zip", "b.7z"])
            .expect("extract")
            .expect("intent");
        assert_eq!(extract.paths.len(), 2);
        assert_eq!(extract.action, ArchiveIntentAction::Extract);
    }
    for values in [
        vec!["--archive-open"],
        vec!["--archive-open", "a.zip", "b.zip"],
        vec!["--archive-test", "a.zip", "b.zip"],
        vec!["--archive-extract", "unknown", "a.zip"],
        vec!["--archive-compress"],
        vec!["--archive-open", "--password", "secret"],
    ] {
        assert!(parse(&values).is_err());
    }
    assert!(parse(&["a.tar.gz"]).expect("bare").is_some());
    assert!(parse(&["notes.txt"]).expect("ordinary").is_none());
    assert!(parse(&["--background"]).expect("background").is_none());
    assert!(
        parse(&["--silent-start", "--archive-open", "a.zip"])
            .expect("silent")
            .is_some()
    );
    assert!(parse(&["--background", "--archive-open", "a.zip"]).is_err());
    assert!(parse(&["--browser-dialog", "--archive-open", "a.zip"]).is_err());
}

#[test]
fn archive_parser_never_truncates_multi_selection() {
    let mut values = vec![OsString::from("--archive-compress")];
    values.extend((0..256).map(|index| OsString::from(format!("source{index}"))));
    assert_eq!(
        parse_arguments(values.clone(), Path::new(r"C:\sender"))
            .expect("256")
            .expect("intent")
            .paths
            .len(),
        256
    );
    values.push("overflow".into());
    assert_eq!(
        parse_arguments(values, Path::new(r"C:\sender")),
        Err(LaunchFailure::InvalidRequest)
    );
}

#[test]
fn actual_main_window_creation_policy_never_flashes_for_archive_origin() {
    assert_eq!(
        main_window_presentation(crate::DesktopLaunchMode::Normal, true),
        (false, false)
    );
    assert_eq!(
        main_window_presentation(crate::DesktopLaunchMode::Normal, false),
        (true, true)
    );
    assert_eq!(
        main_window_presentation(crate::DesktopLaunchMode::SilentStart, false),
        (false, false)
    );
    assert_eq!(
        main_window_presentation(
            crate::DesktopLaunchMode::BrowserDialog {
                correlation_id: Uuid::new_v4()
            },
            false
        ),
        (false, false)
    );
}

#[gpui_kit::test]
fn production_ingress_waits_for_both_owners_and_defers_immediate_completion(
    cx: &mut gpui_kit::TestAppContext,
) {
    let (endpoint, receiver) = nxtools_ipc::desktop_archive_intent::archive_intent_channel();
    let (bridge, native) = nxtools_ipc::platform_bridge(4);
    let request = parse(&["--archive-open", "a.zip"])
        .expect("parse")
        .expect("intent");
    endpoint.submit(request.clone());
    cx.update(|app| start(receiver, Rc::new(bridge), app));
    cx.run_until_parked();
    let calls = Rc::new(std::cell::Cell::new(0usize));
    let calls_for_handler = calls.clone();
    cx.update(|app| {
        register_handler(
            move |_, completion, _| {
                calls_for_handler.set(calls_for_handler.get() + 1);
                assert!(completion.finish(Ok(())));
                assert!(!completion.finish(Ok(())));
            },
            app,
        )
    });
    cx.executor().advance_clock(Duration::from_millis(50));
    cx.run_until_parked();
    assert_eq!(
        calls.get(),
        0,
        "registered owner alone is not native readiness"
    );
    native.set_connected_for_contract_test(true);
    cx.executor().advance_clock(Duration::from_millis(50));
    cx.run_until_parked();
    assert_eq!(calls.get(), 1);
    assert_eq!(
        endpoint.receipt(request.request_id).expect("receipt").state,
        nxtools_protocol::desktop_archive_intent::LaunchReceiptState::Handled
    );
    endpoint.submit(request.clone());
    cx.executor().advance_clock(Duration::from_millis(50));
    cx.run_until_parked();
    assert_eq!(calls.get(), 1, "duplicate replay cannot invoke owner again");
}

#[test]
fn launch_modes_and_restart_arguments_are_not_consumed_as_paths_or_secrets() {
    let request = parse(&[
        "--wait-for-instance-release-ms",
        "10000",
        "--archive-open",
        "a.zip",
    ])
    .expect("protected args")
    .expect("intent");
    assert_eq!(request.paths.len(), 1);
    assert!(parse(&["--password", "a.zip"]).is_err());
    let browser_id = Uuid::new_v4().to_string();
    assert!(
        parse(&["--browser-dialog", "--browser-correlation-id", &browser_id])
            .expect("browser unchanged")
            .is_none()
    );
    assert!(!format!("{request:?}").contains("a.zip"));
}

#[cfg(windows)]
#[test]
fn os_arguments_reject_non_unicode_empty_and_ambiguous_drive_paths_without_lossy_conversion() {
    use std::os::windows::ffi::OsStringExt;
    let malformed = OsString::from_wide(&[0xD800]);
    assert_eq!(
        parse_arguments(
            ["--archive-open".into(), malformed],
            Path::new(r"C:\sender")
        ),
        Err(LaunchFailure::InvalidRequest)
    );
    for path in ["", "C:relative.zip", r"\root-relative.zip", "a\0.zip"] {
        assert_eq!(
            parse(&["--archive-open", path]),
            Err(LaunchFailure::InvalidRequest)
        );
    }
}
