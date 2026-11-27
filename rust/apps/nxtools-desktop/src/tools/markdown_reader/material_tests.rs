use super::*;
use crate::archive::ordinary_material_test_support as support;
use gpui_kit::TestAppContext;

#[gpui_kit::test]
fn material_reader_reuse_waits_and_missing_handle_closes(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let (bridge, endpoint) = nxtools_ipc::platform_bridge(32);
    endpoint.set_connected_for_contract_test(true);
    let bridge = Rc::new(bridge);
    let handle = cx.update(|cx| {
        let handle = show(Some(bridge.clone()), cx).expect("reader");
        assert_eq!(show(Some(bridge), cx), Some(handle));
        handle
            .update(cx, |root, _, cx| {
                let root = root.downcast::<gpui_kit::base::Root>().expect("Root");
                let reader = root
                    .read(cx)
                    .view()
                    .clone()
                    .downcast::<Reader>()
                    .expect("Reader");
                assert!(reader.read(cx).material_pending);
                assert!(reader.read(cx).foreground.is_none());
            })
            .expect("live hidden reader");
        handle
    });
    cx.run_until_parked();
    assert!(!cx.update(|cx| cx.windows().contains(&handle)));
}

#[gpui_kit::test]
fn material_reader_platform_none_keeps_original_lifecycle(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let first = cx.update(|cx| show(None, cx).expect("isolated reader"));
    cx.run_until_parked();
    assert_eq!(cx.update(|cx| show(None, cx)), Some(first));
}

#[gpui_kit::test]
fn material_reader_receipt_releases_same_reader_and_retains_runtime(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.executor().allow_parking();
    let (bridge, endpoint) = nxtools_ipc::platform_bridge(32);
    endpoint.set_connected_for_contract_test(true);
    let bridge = Rc::new(bridge);
    let session = Uuid::new_v4();
    let handle = cx.update(|cx| {
        support::facts(session, 1, cx);
        let handle = show(Some(bridge.clone()), cx).expect("reader");
        crate::ui::material::runtime::set_test_window_handle(handle, 44, cx);
        handle
    });
    cx.run_until_parked();
    let (id, command) = support::next(&endpoint);
    cx.update(|cx| {
        assert!(crate::ui::material::runtime::is_waiting(handle, cx));
        assert_eq!(show(Some(bridge.clone()), cx), Some(handle));
        support::accept(&endpoint, id, command, cx);
    });
    cx.run_until_parked();
    cx.update(|cx| {
        assert!(!crate::ui::material::runtime::is_waiting(handle, cx));
        assert!(cx.windows().contains(&handle));
        handle
            .update(cx, |root, _, cx| {
                let root = root.downcast::<gpui_kit::base::Root>().expect("Root");
                let reader = root
                    .read(cx)
                    .view()
                    .clone()
                    .downcast::<Reader>()
                    .expect("Reader");
                assert!(!reader.read(cx).material_pending);
            })
            .expect("same live reader");
        support::facts(session, 2, cx);
    });
    cx.run_until_parked();
    assert_eq!(support::next(&endpoint).1.window_handle, 44);
}
