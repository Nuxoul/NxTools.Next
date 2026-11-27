use super::*;
use crate::archive::ordinary_material_test_support as support;
use gpui_kit::TestAppContext;

#[gpui_kit::test]
fn material_join_pending_reuse_and_missing_hwnd_keep_original_close_owner(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let (bridge, endpoint) = nxtools_ipc::platform_bridge(32);
    endpoint.set_connected_for_contract_test(true);
    let bridge = Rc::new(bridge);
    let first = cx.update(|cx| {
        let first = open(Some(bridge.clone()), None, cx).expect("join");
        assert_eq!(open(Some(bridge), None, cx), Some(first));
        let Target(view, _) = cx.global::<Target>();
        assert!(
            view.read_with(cx, |view, _| view.material_pending)
                .expect("live view")
        );
        first
    });
    cx.run_until_parked();
    assert!(!cx.update(|cx| cx.windows().contains(&first)));
}

#[gpui_kit::test]
fn material_join_platform_none_keeps_original_reuse(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let first = cx.update(|cx| open(None, None, cx).expect("isolated join"));
    cx.run_until_parked();
    assert_eq!(cx.update(|cx| open(None, None, cx)), Some(first));
}

#[gpui_kit::test]
fn material_join_receipt_releases_original_target_without_recreation(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let (bridge, endpoint) = nxtools_ipc::platform_bridge(32);
    endpoint.set_connected_for_contract_test(true);
    let bridge = Rc::new(bridge);
    let session = Uuid::new_v4();
    let (window, view) = cx.update(|cx| {
        support::facts(session, 1, cx);
        let window = open(Some(bridge.clone()), None, cx).expect("join");
        crate::ui::material::runtime::set_test_window_handle(window, 45, cx);
        (window, cx.global::<Target>().0.clone())
    });
    cx.run_until_parked();
    let (receipt, command) = support::next(&endpoint);
    cx.update(|cx| {
        assert_eq!(open(Some(bridge), None, cx), Some(window));
        assert!(
            view.read_with(cx, |view, _| view.material_pending)
                .expect("pending join")
        );
        support::accept(&endpoint, receipt, command, cx);
    });
    cx.run_until_parked();
    cx.update(|cx| {
        assert_eq!(cx.global::<Target>().0, view);
        assert!(
            !view
                .read_with(cx, |view, _| view.material_pending)
                .expect("ready join")
        );
        support::facts(session, 2, cx);
    });
    cx.run_until_parked();
    assert_eq!(support::next(&endpoint).1.window_handle, 45);
}

/// Material is fixed input to the older menu/reuse test, not its tested behavior.
pub(super) fn prepare_presentation_fixture(platform: Rc<PlatformBridge>, cx: &mut App) {
    use nxtools_protocol::window_material::*;
    support::facts(Uuid::new_v4(), 1, cx);
    let next_hwnd = std::cell::Cell::new(9000_i64);
    cx.observe_new::<gpui_kit::base::Root>(move |root, window, cx| {
        if root.view().clone().downcast::<JoinWindow>().is_err() {
            return;
        }
        let Some(window) = window else {
            return;
        };
        let handle = window.window_handle();
        let hwnd = next_hwnd.get();
        next_hwnd.set(hwnd + 1);
        crate::ui::material::runtime::set_test_window_handle(handle, hwnd, cx);
        let command = crate::ui::material::command(hwnd, MaterialSurface::Backdrop, cx)
            .expect("synthetic current material facts");
        let receipt = AppliedWindowMaterial {
            session_id: command.session_id,
            facts_revision: command.facts_revision,
            window_handle: hwnd,
            generation: command.generation,
            style: command.style,
            surface: command.surface,
            dark: command.dark,
            effective: if command.style == WindowMaterialStyle::Solid {
                EffectiveWindowMaterial::Solid
            } else {
                EffectiveWindowMaterial::Acrylic
            },
            fallback_reason: MaterialFallbackReason::None,
            native_error_code: 0,
        };
        let preparation =
            crate::ui::material::Preparation::confirmed(Uuid::new_v4(), command, receipt, cx)
                .expect("coherent synthetic material preparation");
        assert!(crate::ui::material::runtime::track_prepared(
            handle,
            platform.clone(),
            preparation,
            cx,
        ));
    })
    .detach();
}
