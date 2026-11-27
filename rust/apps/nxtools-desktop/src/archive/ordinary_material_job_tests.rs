use super::*;
use crate::archive::ordinary_material_test_support as support;
use gpui_kit::TestAppContext;

fn submit(app: &mut ArchiveApplication, cx: &mut Context<ArchiveApplication>) -> Uuid {
    app.submit_job(
        ArchiveJobRequest {
            kind: ArchiveJobKind::Test,
            paths: vec![r"C:\owned\material-job.zip".into()],
            destination: None,
            codepage: 0,
            limits: Default::default(),
            extract: None,
            create: None,
            readmission: None,
            modify: None,
        },
        FormCompletion {
            open_after: false,
            keep_open: true,
            remember: None,
            remember_baseline: None,
            additional_requests: vec![],
        },
        cx,
    )
    .expect("original queue admission")
}

#[gpui_kit::test]
fn material_job_reuse_waits_and_receipt_keeps_original_work_and_window(cx: &mut TestAppContext) {
    let (owner, endpoint) = super::super::super::tests::setup_unprepared(cx);
    let (job, window) = owner.update(cx, |app, cx| {
        support::facts(Uuid::new_v4(), 1, cx);
        let job = submit(app, cx);
        let window = app.jobs.records[&job].window.as_ref().expect("progress").0;
        crate::ui::material::runtime::set_test_window_handle(window, 46, cx);
        app.show_job(job, cx);
        assert_eq!(app.jobs.material_pending.get(&job), Some(&window));
        assert!(app.jobs.activations.is_empty());
        (job, window)
    });
    cx.run_until_parked();
    let (id, command) = support::next(&endpoint);
    cx.update(|cx| support::accept(&endpoint, id, command, cx));
    cx.run_until_parked();
    owner.read_with(cx, |app, _| {
        assert!(app.jobs.material_pending.is_empty());
        assert_eq!(
            app.jobs.records[&job]
                .window
                .as_ref()
                .expect("same progress")
                .0,
            window
        );
        assert!(app.jobs.records[&job].terminal.is_none());
    });
}

#[gpui_kit::test]
fn material_job_failure_retires_display_without_cancelling_work(cx: &mut TestAppContext) {
    let (owner, _endpoint) = super::super::super::tests::setup_unprepared(cx);
    let job = owner.update(cx, submit);
    cx.run_until_parked();
    owner.read_with(cx, |app, _| {
        let record = &app.jobs.records[&job];
        assert!(record.window.is_none());
        assert!(record.terminal.is_none());
        assert!(app.jobs.material_pending.is_empty());
    });
}

#[gpui_kit::test]
fn material_job_background_before_receipt_cannot_reopen(cx: &mut TestAppContext) {
    let (owner, endpoint) = super::super::super::tests::setup_unprepared(cx);
    let job = owner.update(cx, |app, cx| {
        support::facts(Uuid::new_v4(), 1, cx);
        let job = submit(app, cx);
        let window = app.jobs.records[&job].window.as_ref().expect("progress").0;
        crate::ui::material::runtime::set_test_window_handle(window, 47, cx);
        app.background_job(job, cx);
        job
    });
    cx.run_until_parked();
    owner.read_with(cx, |app, _| {
        assert!(app.jobs.records[&job].window.is_none());
        assert!(app.jobs.material_pending.is_empty());
    });
    while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
        assert!(!matches!(
            envelope.message,
            nxtools_protocol::Message::PlatformCommand(PlatformCommand::ActivateWindow(_))
        ));
    }
}
