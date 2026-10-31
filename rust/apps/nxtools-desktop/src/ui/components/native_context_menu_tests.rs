use super::*;
use gpui_kit::{Context, Render, TestAppContext, VisualTestContext, size};

struct Source {
    focus: FocusHandle,
}
impl Render for Source {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().track_focus(&self.focus)
    }
}
struct Fixture {
    id: Uuid,
    popup: AnyWindowHandle,
    trigger: Rc<Cell<bool>>,
    endpoint: nxtools_ipc::PlatformServerEndpoint,
    focus: FocusHandle,
    source: AnyWindowHandle,
}
fn fixture(cx: &mut TestAppContext) -> Fixture {
    if cx.read(|cx| cx.try_global::<NativeMenuState>().is_none()) {
        cx.update(gpui_kit::init);
        cx.update(crate::theme::apply_component_theme);
    }
    let mut previous = None;
    let (_, source) = cx.add_window_view(|window, cx| {
        let focus = cx.focus_handle();
        previous = Some(focus.clone());
        focus.focus(window, cx);
        gpui_kit::base::Root::new(cx.new(|_| Source { focus }), window, cx)
    });
    let focus = previous.expect("actual source focus owner");
    let (platform, endpoint) = nxtools_ipc::platform_bridge(16);
    endpoint.set_connected_for_contract_test(true);
    let platform = Rc::new(platform);
    let id = Uuid::new_v4();
    let trigger = Rc::new(Cell::new(true));
    let (popup, focus, source) = source.update(|window, cx| {
        focus.focus(window, cx);
        let surface = styled_menu_surface::open(
            id,
            vec![MenuItem::new("prepared-copy", "复制").on_click(|_, _, _| {})],
            window,
            cx,
        )
        .expect("production hidden popup");
        let popup = surface.window;
        let source = window.window_handle();
        cx.set_global(NativeMenuState {
            pending: Some(Pending {
                id,
                window: source,
                command_id: id,
                source_hwnd: 1,
                source_session: None,
                trigger: Rc::downgrade(&trigger),
                callbacks: HashMap::new(),
                previous_focus: Some(focus.clone()),
                platform,
                surface: Some(surface),
                pointer: false,
                pointer_inside_source: false,
                source_bounds: gpui_kit::Bounds::new(
                    point(gpui_kit::DevicePixels(0), gpui_kit::DevicePixels(0)),
                    gpui_kit::size(gpui_kit::DevicePixels(200), gpui_kit::DevicePixels(200)),
                ),
                ready: false,
                step: 0,
                started: Instant::now(),
            }),
            source_dismiss: None,
        });
        (popup, focus, source)
    });
    // Native preparation's WM_SIZE, not a platform frame. TestPlatform defaults to 2x DPI.
    cx.simulate_window_scale_factor_change(popup, 1.0);
    cx.simulate_window_resize(popup, size(px(1200.0), px(800.0)));
    Fixture {
        id,
        popup,
        trigger,
        endpoint,
        focus,
        source,
    }
}
fn result(id: Uuid, data: Option<&str>, cx: &mut App) {
    handle_notification(
        &PlatformNotification::CommandResult {
            correlation_id: id,
            result: nxtools_protocol::PlatformCommandResult {
                success: true,
                error: None,
                native_error_code: None,
                data: data.map(str::to_owned),
            },
        },
        cx,
    );
}
const PLACEMENT: &str = r#"{"width":1200,"height":800,"anchor_x":299,"anchor_y":30}"#;

#[gpui_kit::test]
fn hidden_menu_preparation_builds_scene_without_platform_frame_before_reveal(
    cx: &mut TestAppContext,
) {
    let fixture = fixture(cx);
    cx.update(|cx| result(fixture.id, Some(PLACEMENT), cx));
    // Deliberately never deliver simulate_next_frame: a hidden Windows HWND has no frame loop.
    let show = fixture
        .endpoint
        .try_receive_command_for_contract_test()
        .expect("prepared hidden menu must reveal without waiting for a hidden HWND frame");
    assert_ne!(
        show.id, fixture.id,
        "prepare and reveal have distinct phase IDs"
    );
    assert!(matches!(
        show.message,
        nxtools_protocol::Message::PlatformCommand(PlatformCommand::SetWindowVisibility(
            SetWindowVisibilityCommand { visible: true, .. }
        ))
    ));
    let mut popup = VisualTestContext::from_window(fixture.popup, cx);
    assert!(
        popup.debug_bounds("prepared-copy").is_some(),
        "real official menu scene exists before native reveal"
    );
    cx.update(|cx| result(fixture.id, Some(PLACEMENT), cx));
    assert!(
        fixture
            .endpoint
            .try_receive_command_for_contract_test()
            .is_err(),
        "duplicate prepare cannot reveal twice"
    );
    cx.update(|cx| result(show.id, None, cx));
    let activate = fixture
        .endpoint
        .try_receive_command_for_contract_test()
        .expect("activation after acknowledged show");
    assert_ne!(activate.id, show.id);
    assert!(matches!(
        activate.message,
        nxtools_protocol::Message::PlatformCommand(PlatformCommand::ActivateWindow(_))
    ));
    cx.update(|cx| result(show.id, None, cx));
    assert!(
        fixture
            .endpoint
            .try_receive_command_for_contract_test()
            .is_err()
    );
    cx.update(|cx| result(activate.id, None, cx));
    assert!(cx.read(|cx| {
        cx.global::<NativeMenuState>()
            .pending
            .as_ref()
            .expect("live menu")
            .ready
    }));
    let mut popup = VisualTestContext::from_window(fixture.popup, cx);
    popup.simulate_keystrokes("escape");
    popup.run_until_parked();
    assert!(
        cx.read(|cx| cx.global::<NativeMenuState>().pending.is_none()),
        "official Escape retires the session"
    );
    let mut source = VisualTestContext::from_window(fixture.source, cx);
    source.update(|window, _| {
        assert!(
            fixture.focus.is_focused(window),
            "Escape restores original source focus"
        )
    });
}

#[gpui_kit::test]
fn hidden_menu_mismatched_native_viewport_never_reveals(cx: &mut TestAppContext) {
    let fixture = fixture(cx);
    cx.simulate_window_resize(fixture.popup, size(px(640.0), px(480.0)));
    cx.update(|cx| result(fixture.id, Some(PLACEMENT), cx));
    assert!(
        cx.read(|cx| cx.global::<NativeMenuState>().pending.is_none()),
        "unconfirmed viewport must fail closed, not show a clipped or default-sized menu"
    );
    let cancel = fixture
        .endpoint
        .try_receive_command_for_contract_test()
        .expect("cancel exact failed session");
    assert!(
        matches!(cancel.message, nxtools_protocol::Message::PlatformCommand(
        PlatformCommand::CancelContextMenu(CancelContextMenuCommand { session_id })) if session_id == fixture.id)
    );
    while let Ok(command) = fixture.endpoint.try_receive_command_for_contract_test() {
        assert!(!matches!(
            command.message,
            nxtools_protocol::Message::PlatformCommand(PlatformCommand::SetWindowVisibility(_))
        ));
    }
}

#[gpui_kit::test]
fn hidden_menu_retired_trigger_or_disconnect_cannot_reveal(cx: &mut TestAppContext) {
    for disconnect in [false, true] {
        let fixture = fixture(cx);
        let trigger = fixture.trigger.clone();
        cx.update(|cx| {
            result(fixture.id, Some(PLACEMENT), cx);
            // This runs after hidden draw but before its deferred reveal.
            cx.defer(move |cx| {
                if disconnect {
                    handle_notification(
                        &PlatformNotification::Disconnected {
                            generation: 1,
                            pending_command_ids: vec![],
                        },
                        cx,
                    );
                } else {
                    trigger.set(false);
                }
            });
        });
        assert!(cx.read(|cx| cx.global::<NativeMenuState>().pending.is_none()));
        cx.update(|cx| result(fixture.id, Some(PLACEMENT), cx));
        while let Ok(command) = fixture.endpoint.try_receive_command_for_contract_test() {
            assert!(
                !matches!(
                    command.message,
                    nxtools_protocol::Message::PlatformCommand(
                        PlatformCommand::SetWindowVisibility(_)
                    )
                ),
                "late/dead preparation cannot reveal"
            );
        }
    }
}

#[gpui_kit::test]
fn runtime_feedback_source_pointer_dismiss_restores_source_not_external(cx: &mut TestAppContext) {
    let fixture = fixture(cx);
    cx.update(|cx| {
        cx.global_mut::<NativeMenuState>()
            .pending
            .as_mut()
            .expect("live menu")
            .ready = true;
        handle_notification(
            &PlatformNotification::Event(PlatformEvent::GlobalPointerPressed(
                nxtools_protocol::PointerPressContext {
                    window_handle: 1,
                    root_window_handle: 1,
                    process_id: std::process::id(),
                    sampled_at_unix_ms: nxtools_ipc::unix_time_ms(),
                    event_sequence: 1,
                },
            )),
            cx,
        );
    });
    cx.run_until_parked();
    let mut restored = false;
    while let Ok(command) = fixture.endpoint.try_receive_command_for_contract_test() {
        restored |= matches!(
            command.message,
            nxtools_protocol::Message::PlatformCommand(PlatformCommand::ActivateWindow(
                ActivateWindowCommand { window_handle: 1 }
            ))
        );
    }
    assert!(
        restored,
        "clicking the source panel closes only its menu and restores source focus"
    );
    assert!(cx.read(|cx| cx.global::<NativeMenuState>().pending.is_none()));
}

#[gpui_kit::test]
fn runtime_feedback_menu_overlay_click_inside_source_only_dismisses_menu(cx: &mut TestAppContext) {
    let fixture = fixture(cx);
    cx.update(|cx| result(fixture.id, Some(PLACEMENT), cx));
    while fixture
        .endpoint
        .try_receive_command_for_contract_test()
        .is_ok()
    {}
    let mut popup = VisualTestContext::from_window(fixture.popup, cx);
    popup.simulate_mouse_down(
        point(px(25.0), px(25.0)),
        gpui_kit::MouseButton::Left,
        Default::default(),
    );
    cx.run_until_parked();
    let mut restored = false;
    while let Ok(command) = fixture.endpoint.try_receive_command_for_contract_test() {
        restored |= matches!(
            command.message,
            nxtools_protocol::Message::PlatformCommand(PlatformCommand::ActivateWindow(_))
        );
    }
    assert!(
        restored,
        "menu overlay point inside source restores panel instead of outside close"
    );
}
