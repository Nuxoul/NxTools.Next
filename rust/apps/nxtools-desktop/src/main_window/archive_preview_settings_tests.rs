//! Production official NumberField -> deferred settings owner -> locked config save.
use super::*;

fn search_number(
    window: &mut VisualTestContext,
    title: &str,
) -> gpui_kit::Bounds<gpui_kit::Pixels> {
    window.update(|window, cx| {
        window.activate_window();
        window.draw(cx).clear(cx);
    });
    let search = window.update(|window, _| {
        gpui_kit::base::test_support::snapshots(window)
            .into_iter()
            .find(|node| {
                node.role() == Some(gpui_kit::Role::TextInput)
                    && node.path().contains(&"settings-sidebar".into())
            })
            .expect("official Settings search")
            .bounds()
    });
    window.simulate_click(search.center(), Modifiers::default());
    window.simulate_keystrokes("ctrl-a");
    window.simulate_input(title);
    window.run_until_parked();
    window.update(|window, cx| window.draw(cx).clear(cx));
    window.update(|window, _| {
        let nodes = gpui_kit::base::test_support::snapshots(window);
        nodes
            .iter()
            .find(|node| {
                node.label() == Some(title) && node.role() == Some(gpui_kit::Role::SpinButton)
            })
            .unwrap_or_else(|| panic!("official preview number field {title}: {nodes:?}"))
            .bounds()
    })
}
fn enter_number(window: &mut VisualTestContext, title: &str, value: u32) {
    let bounds = search_number(window, title);
    assert_eq!(
        bounds.size.height,
        px(30.),
        "official32DIP frame, two1DIP borders"
    );
    window.simulate_click(bounds.center(), Modifiers::default());
    window.simulate_keystrokes("ctrl-a");
    window.simulate_input(&value.to_string());
    window.run_until_parked();
    window.update(|window, cx| window.draw(cx).clear(cx));
}

#[gpui_kit::test]
fn preview_official_numbers_save_changed_leaves_and_preserve_failed_state(cx: &mut TestAppContext) {
    let directory = tempfile::tempdir().expect("owned preview settings");
    let path = directory.path().join("config.json");
    AppConfig::default().save_to(&path).expect("seed");
    let original = std::fs::read(&path).expect("initial bytes");
    let (_, view, mut window) = mount(cx, path.clone());
    select_page(&mut window, 6);
    window.update(|window, cx| window.draw(cx).clear(cx));
    assert_eq!(std::fs::read(&path).expect("render bytes"), original);
    AppConfig::update_section(&path, |config| {
        config.archive.preview.image_limit_mib = 64;
        config.archive.preview.text_max_lines = 40_000;
        config.download.default_connection_limit = 24;
    })
    .expect("concurrent settings owner");
    enter_number(&mut window, "文本大小上限（KiB）", 2048);
    let saved = AppConfig::load_from(&path).expect("real input persisted");
    assert_eq!(saved.archive.preview.text_limit_kib, 2048);
    assert_eq!(saved.archive.preview.text_max_lines, 40_000);
    assert_eq!(saved.archive.preview.image_limit_mib, 64);
    assert_eq!(saved.download.default_connection_limit, 24);
    for (title, value) in [
        ("文本大小上限（KiB）", 8192),
        ("文本最大行数", 100_000),
        ("图片文件大小上限（MiB）", 128),
        ("图片最大像素（百万）", 64),
        ("图片解码预算（MiB）", 1024),
        ("解压工作时限（秒）", 120),
    ] {
        enter_number(&mut window, title, value);
    }
    let saved = AppConfig::load_from(&path)
        .expect("six maxima persisted")
        .archive
        .preview;
    assert_eq!(
        (
            saved.text_limit_kib,
            saved.text_max_lines,
            saved.image_limit_mib,
            saved.image_max_megapixels,
            saved.image_decode_mib,
            saved.timeout_seconds
        ),
        (8192, 100_000, 128, 64, 1024, 120)
    );
    std::fs::write(&path, b"{broken").expect("corrupt fixture");
    enter_number(&mut window, "解压工作时限（秒）", 30);
    assert_eq!(
        view.read_with(&window, |view, _| view.settings.preview),
        saved
    );
    assert!(view.read_with(&window, |view, _| view.error.is_some()));
    assert_eq!(
        std::fs::read(&path).expect("unchanged corrupt file"),
        b"{broken"
    );
}

#[gpui_kit::test]
fn preview_official_numbers_remain_reachable_at_minimum_window(cx: &mut TestAppContext) {
    let directory = tempfile::tempdir().expect("owned preview settings");
    let path = directory.path().join("config.json");
    AppConfig::default().save_to(&path).expect("seed");
    let (_, _, mut window) = mount(cx, path.clone());
    select_page(&mut window, 6);
    for dimensions in [size(px(1060.), px(650.)), size(px(920.), px(480.))] {
        window.simulate_resize(dimensions);
        for title in [
            "文本大小上限（KiB）",
            "文本最大行数",
            "图片文件大小上限（MiB）",
            "图片最大像素（百万）",
            "图片解码预算（MiB）",
            "解压工作时限（秒）",
        ] {
            let bounds = search_number(&mut window, title);
            assert!(bounds.left() >= px(0.) && bounds.right() <= dimensions.width);
            assert!(bounds.top() >= px(0.) && bounds.bottom() <= dimensions.height);
        }
    }
    assert_eq!(
        AppConfig::load_from(&path)
            .expect("unchanged config")
            .archive
            .preview,
        nxtools_core::archive::ArchivePreviewSettings::default()
    );
}
