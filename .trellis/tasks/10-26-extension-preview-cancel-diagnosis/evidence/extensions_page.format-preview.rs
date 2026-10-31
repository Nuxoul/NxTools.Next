use super::*;
use crate::extensions::ExtensionApplication;
use gpui_kit::component::alert::Alert;
use gpui_kit::component::form::{Field, Form};
use gpui_kit::component::setting::{SettingGroup, SettingItem, SettingPage, Settings};
#[path = "extensions_settings.rs"]
mod extensions_settings;
use extensions_settings::{ChainDraft, PackageSettingsDraft};
#[path = "extensions_page/activity.rs"]
mod activity;
#[path = "extensions_page/catalog.rs"]
mod catalog;
#[path = "extensions_page/environment.rs"]
mod environment;
#[path = "extensions_page/import_confirmation.rs"]
mod import_confirmation;
#[path = "extensions_page/removal.rs"]
mod removal;
#[path = "extensions_page/trust_confirmation.rs"]
mod trust_confirmation;
#[path = "extensions_page/usage.rs"]
mod usage;
#[path = "extensions_page/workspace.rs"]
mod workspace;

const TRUST_WARNING: &str =
    "信任后，此插件可使用当前用户的全部权限。以下权限由插件作者声明，并非系统访问限制。";

pub(crate) struct ExtensionManagerView {
    pub(crate) application: Entity<ExtensionApplication>,
    pub(crate) path: Entity<InputState>,
    search: Entity<InputState>,
    import_open: bool,
    import_dialog: Option<crate::extensions::ImportCandidateIdentity>,
    workspace_scroll: gpui_kit::ScrollHandle,
    _search_subscription: Subscription,
    aliases: Entity<InputState>,
    alias_target: Option<(String, String)>,
    selected_package: Option<String>,
    detail_initial_page: usize,
    detail_navigation_revision: u64,
    settings_draft: Option<PackageSettingsDraft>,
    parked_settings: std::collections::HashMap<String, PackageSettingsDraft>,
    seen_commit_revision: u64,
    environment: Option<environment::EnvironmentDraft>,
    runtime_path: Entity<InputState>,
    trust_target: Option<String>,
    trust_dialog: Option<Rc<RefCell<trust_confirmation::TrustDraft>>>,
    chain_host: Entity<InputState>,
    chain_draft: Option<ChainDraft>,
    draft_error: Option<String>,
    control_scopes: Rc<RefCell<std::collections::HashMap<String, FocusHandle>>>,
    _subscription: Subscription,
}

impl ExtensionManagerView {
    pub(crate) fn new(
        application: Entity<ExtensionApplication>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let path = cx.new(|cx| InputState::new(window, cx).placeholder("ZIP 扩展包路径"));
        let search =
            cx.new(|cx| InputState::new(window, cx).placeholder("搜索扩展名称、功能或关键词"));
        let search_subscription = cx.observe(&search, |_, _, cx| cx.notify());
        let aliases = cx.new(|cx| InputState::new(window, cx).placeholder("关键词，以空格分隔"));
        let runtime_path = cx
            .new(|cx| InputState::new(window, cx).placeholder("已安装解释器或包内 EXE 的完整路径"));
        let subscription = cx.observe(&application, |view, _, cx| {
            view.reconcile_settings(cx);
            view.reconcile_environment(cx);
            cx.notify();
        });
        let chain_host = cx.new(|cx| {
            InputState::new(window, cx).placeholder("例如 example.com（仅域名，不含 URL 或路径）")
        });
        let seen_commit_revision = application.read(cx).commit_revision;
        Self {
            application,
            path,
            search,
            import_open: false,
            import_dialog: None,
            workspace_scroll: Default::default(),
            _search_subscription: search_subscription,
            aliases,
            alias_target: None,
            selected_package: None,
            detail_initial_page: 0,
            detail_navigation_revision: 0,
            settings_draft: None,
            parked_settings: Default::default(),
            environment: None,
            seen_commit_revision,
            runtime_path,
            trust_target: None,
            trust_dialog: None,
            chain_host,
            chain_draft: None,
            draft_error: None,
            control_scopes: Default::default(),
            _subscription: subscription,
        }
    }
    #[cfg(debug_assertions)]
    pub(crate) fn initialize_preview(&mut self, import: bool, package: Option<String>) {
        self.import_open = import;
        if let Some(package) = package {
            self.open_package(package);
        }
    }

    fn open_settings(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        self.open_package(id.clone());
        self.detail_initial_page = 2;
        self.detail_navigation_revision = self.detail_navigation_revision.wrapping_add(1);
        if self.settings_draft.is_none()
            && let Some(package) = self
                .application
                .read(cx)
                .packages
                .packages
                .iter()
                .find(|package| package.manifest.id == id)
                .cloned()
                .filter(|package| !package.manifest.settings.is_empty())
        {
            self.settings_draft = Some(PackageSettingsDraft::new(&package, window, cx));
        }
    }
    fn open_package(&mut self, id: String) {
        if let Some(draft) = self.settings_draft.take() {
            self.parked_settings
                .insert(draft.package_id().to_owned(), draft);
        }
        self.settings_draft = self.parked_settings.remove(&id);
        self.selected_package = Some(id);
        self.detail_initial_page = 0;
        self.draft_error = None;
    }
}

impl Render for ExtensionManagerView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.reconcile_import_dialog(window, cx);
        self.reconcile_trust_dialog(window, cx);
        if self.environment.is_some() {
            return self.render_environment(cx);
        }
        if let Some(editor) = self.render_w9_editor(window, cx) {
            return editor;
        }
        self.render_workspace(window, cx)
    }
}

fn runtime_description(runtime: Option<&nxtools_extensions::ScriptRuntime>) -> String {
    runtime.map_or_else(
        || "运行时：声明式动作（不启动脚本）".into(),
        |runtime| {
            let timeout = runtime.timeout_ms.map_or_else(
                || "按入口默认：搜索 3000 / 解析 10000 / 动作与事件 30000 ms".into(),
                |timeout| format!("声明超时 {timeout} ms（受入口上限约束）"),
            );
            format!("运行时：{:?}；{timeout}；搜索上限 10s / 解析 30s / 动作 120s；每包 1 / 全局 4 / 排队 32；每次 512MiB / 8 进程；帧 256KiB / 输出 4MiB；需独立信任与启用", runtime.program)
        },
    )
}
fn action_description(action: &nxtools_extensions::DeclarativeAction) -> String {
    use nxtools_extensions::DeclarativeAction;
    match action {
        DeclarativeAction::Website { url_template } => format!("打开网站 {url_template}"),
        DeclarativeAction::Folder { path } => format!("打开文件夹 {path}"),
        DeclarativeAction::Program {
            executable,
            argument_template,
            working_directory,
        } => format!("运行 {executable}；参数 {argument_template}；工作目录 {working_directory}"),
        DeclarativeAction::Script { entry_file } => {
            format!("脚本入口 {entry_file}（当前用户权限）")
        }
        DeclarativeAction::Items { items } => items
            .iter()
            .map(|item| format!("{}：{}", item.title, action_description(&item.action)))
            .collect::<Vec<_>>()
            .join("；"),
    }
}
#[cfg(test)]
fn write_zip(path: &std::path::Path, manifest: &[u8]) {
    use std::io::Write;
    let mut writer = zip::ZipWriter::new(std::fs::File::create(path).expect("owned ZIP"));
    writer
        .start_file("extension.json", zip::write::SimpleFileOptions::default())
        .expect("ZIP manifest");
    writer.write_all(manifest).expect("manifest bytes");
    writer.finish().expect("finish ZIP");
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{TestAppContext, size};
    #[gpui_kit::test]
    fn extension_text_actions_use_default_kit_form_tier(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let app = cx.new(|_| {
            ExtensionApplication::new(
                nxtools_core::AppConfig::default(),
                std::env::temp_dir().join(format!("button-tier-{}.json", uuid::Uuid::new_v4())),
                None,
                gpui_kit::WeakEntity::new_invalid(),
            )
        });
        let (_, surface) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|cx| ExtensionManagerView::new(app, window, cx));
            gpui_kit::base::Root::new(view, window, cx)
        });
        surface.update(|window, cx| window.draw(cx).clear(cx));
        for selector in [
            "extension-import-open",
            "extension-environment-open",
            "extension-empty-action",
        ] {
            assert_eq!(
                surface
                    .debug_bounds(selector)
                    .expect("text action")
                    .size
                    .height,
                px(32.0),
                "official default form tier: {selector}"
            );
        }
    }
    #[gpui_kit::test]
    fn import_confirmation_uses_official_dialog_and_keeps_default_footer_visible(
        cx: &mut TestAppContext,
    ) {
        use gpui_kit::component::WindowExt;
        cx.update(gpui_kit::init);
        let preview=nxtools_extensions::ImportPreview::from_json(r#"{"schema_version":1,"id":"fixture.confirm","name":"剪贴板文本整理","version":"1.0.0","description":"整理文本并保留原文","handlers":[{"id":"web","title":"查看说明","feature":"quick_launch","entry":"command","keywords":["confirm"],"action":{"type":"website","url_template":"https://example.test/{query}"}}]}"#.as_bytes()).expect("preview");
        let app = cx.new(|_| {
            let mut app = ExtensionApplication::new(
                nxtools_core::AppConfig::default(),
                std::env::temp_dir().join(format!("confirmation-{}.json", uuid::Uuid::new_v4())),
                None,
                gpui_kit::WeakEntity::new_invalid(),
            );
            app.preview = Some((preview, "C:\\Synthetic\\cleaner.zip".into()));
            app
        });
        let (_, surface) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|cx| ExtensionManagerView::new(app, window, cx));
            gpui_kit::base::Root::new(view, window, cx)
        });
        surface.simulate_resize(size(px(744.0), px(430.0)));
        surface.update(|window, cx| window.draw(cx).clear(cx));
        surface.run_until_parked();
        surface.update(|window, cx| window.draw(cx).clear(cx));
        assert!(
            surface.update(|window, cx| window.has_active_dialog(cx)),
            "preview must use the official modal Dialog, not inline raw text"
        );
        let confirm = surface
            .debug_bounds("extension-confirm-import")
            .expect("confirm footer");
        let cancel = surface
            .debug_bounds("extension-cancel-import")
            .expect("cancel footer");
        assert_eq!(confirm.size.height, px(32.0));
        assert_eq!(cancel.size.height, px(32.0));
        assert!(confirm.bottom() <= px(430.0) && cancel.bottom() <= px(430.0));
        let warning = surface
            .debug_bounds("extension-import-permission-warning")
            .expect("visible authority warning");
        assert!(
            warning.top() >= px(0.0) && warning.bottom() <= confirm.top(),
            "risk remains visible above footer at minimum viewport"
        );
    }

    #[gpui_kit::test]
    fn w6_manager_exposes_real_import_preview_action_in_workspace(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let config_path =
            std::env::temp_dir().join(format!("nxtools-w6-view-{}.json", uuid::Uuid::new_v4()));
        let application = cx.new(|_| {
            ExtensionApplication::new(
                nxtools_core::AppConfig::default(),
                config_path,
                None,
                gpui_kit::WeakEntity::new_invalid(),
            )
        });
        let (_root, window) = cx.add_window_view(move |window, cx| {
            window.resize(size(px(720.0), px(440.0)));
            let view = cx.new(|cx| ExtensionManagerView::new(application, window, cx));
            gpui_kit::base::Root::new(view, window, cx)
        });
        window.refresh().expect("manager render");
        let inputs = window.update(|window, _| {
            gpui_kit::base::test_support::snapshots(window)
                .into_iter()
                .filter(|snapshot| snapshot.role() == Some(gpui_kit::Role::TextInput))
                .count()
        });
        assert_eq!(
            inputs, 1,
            "overview has one real search Input, not a Settings sidebar"
        );
        assert!(
            window.debug_bounds("extension-cancel-manual").is_none(),
            "no stop action without pending manual work"
        );
        assert!(
            window.debug_bounds("extension-import-preview").is_none(),
            "import form starts collapsed"
        );
        let open = window
            .debug_bounds("extension-import-open")
            .expect("import action");
        window.simulate_click(open.center(), gpui_kit::Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("expanded import");
        assert!(
            window.debug_bounds("extension-import-preview").is_some(),
            "actual import preview control"
        );
        assert!(
            window
                .debug_bounds("extension-permission-warning")
                .is_some(),
            "full authority disclosure is visible"
        );
    }

    #[gpui_kit::test]
    fn import_path_actions_match_input_tier_and_open_import_has_no_duplicate_invitation(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let application = cx.new(|_| {
            ExtensionApplication::new(
                nxtools_core::AppConfig::default(),
                std::env::temp_dir()
                    .join(format!("extension-layout-{}.json", uuid::Uuid::new_v4())),
                None,
                gpui_kit::WeakEntity::new_invalid(),
            )
        });
        let (_, surface) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|cx| {
                let mut view = ExtensionManagerView::new(application, window, cx);
                view.import_open = true;
                view
            });
            gpui_kit::base::Root::new(view, window, cx)
        });
        for viewport in [size(px(884.0), px(600.0)), size(px(744.0), px(430.0))] {
            surface.simulate_resize(viewport);
            surface.update(|window, cx| window.draw(cx).clear(cx));
            let input = surface
                .debug_bounds("extension-import-path-control")
                .expect("original path input");
            for selector in ["extension-pick-file", "extension-import-preview"] {
                let button = surface.debug_bounds(selector).expect("import action");
                assert_eq!(
                    button.size.height, input.size.height,
                    "same official size tier: {selector}"
                );
                assert_eq!(
                    button.center().y,
                    input.center().y,
                    "same row center: {selector}"
                );
            }
            assert!(
                surface.debug_bounds("extension-empty-action").is_none(),
                "opened import has no second import CTA"
            );
        }
    }

    #[gpui_kit::test(iterations = 8)]
    fn w6_manager_pointer_preview_confirm_and_toggle_commit_real_config(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        // This test exercises business commits, not transient Dialog entrance coordinates.
        cx.update(|cx| cx.set_reduce_motion(true));
        cx.executor().allow_parking();
        let temp = tempfile::tempdir().expect("fixture");
        let config_path = temp.path().join("config.json");
        nxtools_core::AppConfig::default()
            .save_to(&config_path)
            .expect("config");
        let original_config = std::fs::read(&config_path).expect("initial config bytes");
        let manifest_path = temp.path().join("package.zip");
        write_zip(&manifest_path, br#"{"schema_version":1,"id":"example.web","name":"Web","version":"1.0.0","handlers":[{"id":"web","title":"Web","feature":"quick_launch","entry":"command","keywords":["web"],"action":{"type":"website","url_template":"https://example.test/?q={query}"}}]}"#);
        let application = cx.new(|_| {
            ExtensionApplication::new(
                nxtools_core::AppConfig::default(),
                config_path.clone(),
                None,
                gpui_kit::WeakEntity::new_invalid(),
            )
        });
        let observer = application.clone();
        let retained = Rc::new(RefCell::new(None));
        let slot = retained.clone();
        let (_root, window) = cx.add_window_view(move |window, cx| {
            window.resize(size(px(740.0), px(570.0)));
            let view = cx.new(|cx| ExtensionManagerView::new(application, window, cx));
            *slot.borrow_mut() = Some(view.clone());
            gpui_kit::base::Root::new(view, window, cx)
        });
        let view = retained.borrow().clone().expect("view");
        window.refresh().expect("workspace");
        let open = window
            .debug_bounds("extension-import-open")
            .expect("open import");
        window.simulate_click(open.center(), gpui_kit::Modifiers::default());
        window.run_until_parked();
        view.update_in(window, |view, window, cx| {
            view.path.update(cx, |input, cx| {
                input.set_value(manifest_path.to_string_lossy().into_owned(), window, cx)
            })
        });
        window.refresh().expect("render");
        let preview = window
            .debug_bounds("extension-import-preview")
            .expect("preview");
        window.simulate_click(preview.center(), gpui_kit::Modifiers::default());
        wait_for_idle(&observer, window);
        assert!(window.read(|cx| observer.read(cx).preview.is_some()));
        assert!(
            nxtools_core::AppConfig::load_from(&config_path)
                .expect("not imported")
                .extensions
                .packages
                .is_empty()
        );
        window.refresh().expect("preview render");
        let cancel_import = window
            .debug_bounds("extension-cancel-import")
            .expect("cancel preview");
        window.simulate_mouse_down(
            cancel_import.center(),
            MouseButton::Left,
            Default::default(),
        );
        window
            .executor()
            .advance_clock(std::time::Duration::from_millis(50));
        window.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(
            window.debug_bounds("extension-cancel-import"),
            Some(cancel_import),
            "business fixture keeps the original Cancel hit target stable through release"
        );
        window.simulate_mouse_up(
            cancel_import.center(),
            MouseButton::Left,
            Default::default(),
        );
        window.run_until_parked();
        assert!(window.read(|cx| observer.read(cx).preview.is_none()));
        assert!(
            nxtools_core::AppConfig::load_from(&config_path)
                .expect("cancel import")
                .extensions
                .packages
                .is_empty()
        );
        assert_eq!(
            std::fs::read(&config_path).expect("cancel config bytes"),
            original_config,
            "preview cancellation never writes configuration"
        );
        window.refresh().expect("preview again");
        let preview = window
            .debug_bounds("extension-import-preview")
            .expect("preview again");
        window.simulate_click(preview.center(), gpui_kit::Modifiers::default());
        wait_for_idle(&observer, window);
        window.refresh().expect("confirmed preview");
        let confirm = window
            .debug_bounds("extension-confirm-import")
            .expect("confirm");
        window.simulate_click(confirm.center(), gpui_kit::Modifiers::default());
        wait_for_idle(&observer, window);
        std::fs::remove_file(&manifest_path).expect("remove original ZIP");
        let installed = nxtools_core::AppConfig::load_from(&config_path).expect("reopen installed");
        let managed = installed.extensions.packages[0]
            .code_directory
            .as_ref()
            .expect("managed directory");
        assert!(
            managed.starts_with(
                std::fs::canonicalize(temp.path().join("extensions")).expect("managed root")
            ) && managed.join("extension.json").is_file()
        );
        assert!(
            !nxtools_core::AppConfig::load_from(&config_path)
                .expect("imported")
                .extensions
                .packages[0]
                .enabled
        );
        for enabled in [true, false] {
            window.refresh().expect("package row");
            let toggle = window
                .debug_bounds("extension-toggle-example.web")
                .expect("toggle");
            window.simulate_click(toggle.center(), gpui_kit::Modifiers::default());
            wait_for_idle(&observer, window);
            assert_eq!(
                nxtools_core::AppConfig::load_from(&config_path)
                    .expect("saved")
                    .extensions
                    .packages[0]
                    .enabled,
                enabled
            );
        }
        window.refresh().expect("workspace package");
        let manage = window
            .debug_bounds("extension-manage-example.web")
            .expect("manage package");
        window.simulate_click(manage.center(), gpui_kit::Modifiers::default());
        window.run_until_parked();
        extensions_settings::tests::select_page(window, "命令与工具");
        window.refresh().expect("edit action");
        let edit = window
            .debug_bounds("extension-edit-example.web-web")
            .expect("edit aliases");
        window.simulate_click(edit.center(), gpui_kit::Modifiers::default());
        window.run_until_parked();
        view.update_in(window, |view, window, cx| {
            view.aliases
                .update(cx, |input, cx| input.set_value("lookup", window, cx))
        });
        window.refresh().expect("alias editor");
        let cancel = window
            .debug_bounds("extension-cancel-aliases")
            .expect("cancel");
        window.simulate_click(cancel.center(), gpui_kit::Modifiers::default());
        window.run_until_parked();
        assert!(
            nxtools_core::AppConfig::load_from(&config_path)
                .expect("cancel saved nothing")
                .extensions
                .packages[0]
                .aliases
                .is_empty()
        );
        window.refresh().expect("edit again");
        let edit = window
            .debug_bounds("extension-edit-example.web-web")
            .expect("edit again");
        window.simulate_click(edit.center(), gpui_kit::Modifiers::default());
        window.run_until_parked();
        view.update_in(window, |view, window, cx| {
            view.aliases
                .update(cx, |input, cx| input.set_value("lookup", window, cx))
        });
        window.refresh().expect("save alias");
        let save = window.debug_bounds("extension-save-aliases").expect("save");
        window.simulate_click(save.center(), gpui_kit::Modifiers::default());
        wait_for_idle(&observer, window);
        assert_eq!(
            nxtools_core::AppConfig::load_from(&config_path)
                .expect("alias saved")
                .extensions
                .packages[0]
                .aliases["web"],
            ["lookup"]
        );
        extensions_settings::tests::select_page(window, "信任与启用");
        window.refresh().expect("remove row");
        let remove = window
            .debug_bounds("extension-remove-example.web")
            .expect("remove");
        window.simulate_click(remove.center(), gpui_kit::Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("review removal");
        let confirm = window
            .debug_bounds("extension-remove-confirm")
            .expect("explicit confirmation");
        window.simulate_click(confirm.center(), gpui_kit::Modifiers::default());
        wait_for_idle(&observer, window);
        assert!(
            nxtools_core::AppConfig::load_from(&config_path)
                .expect("removed")
                .extensions
                .packages
                .is_empty()
        );
    }

    #[gpui_kit::test(iterations = 4)]
    fn preview_cancel_after_animated_entrance_keeps_reopened_identity_and_config(
        cx: &mut TestAppContext,
    ) {
        use gpui_kit::component::WindowExt as _;
        cx.update(gpui_kit::init);
        cx.update(|cx| cx.set_reduce_motion(false));
        cx.executor().allow_parking();
        let temp = tempfile::tempdir().expect("owned animated preview");
        let config_path = temp.path().join("config.json");
        nxtools_core::AppConfig::default()
            .save_to(&config_path)
            .expect("seed owned config");
        let original = std::fs::read(&config_path).expect("original config bytes");
        let zip = temp.path().join("candidate.zip");
        write_zip(&zip, br#"{"schema_version":1,"id":"fixture.animated","name":"Animated","version":"1","handlers":[{"id":"open","title":"Open","feature":"tools","entry":"action","action":{"type":"website","url_template":"https://synthetic.invalid"}}]}"#);
        let app = cx.new(|_| {
            ExtensionApplication::new(
                nxtools_core::AppConfig::default(),
                config_path.clone(),
                None,
                gpui_kit::WeakEntity::new_invalid(),
            )
        });
        let owner = app.clone();
        let retained = Rc::new(RefCell::new(None));
        let slot = retained.clone();
        let (_, surface) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|cx| ExtensionManagerView::new(owner, window, cx));
            *slot.borrow_mut() = Some(view.clone());
            gpui_kit::base::Root::new(view, window, cx)
        });
        surface.simulate_resize(size(px(744.0), px(570.0)));
        surface.refresh().expect("production workspace");
        let open = surface
            .debug_bounds("extension-import-open")
            .expect("original import command");
        surface.simulate_click(open.center(), Default::default());
        surface.run_until_parked();
        let view = retained.borrow().clone().expect("production manager");
        view.update_in(surface, |view, window, cx| {
            view.path.update(cx, |input, cx| {
                input.set_value(zip.to_string_lossy().into_owned(), window, cx)
            });
            cx.notify();
        });
        let mut previous = None;
        for _ in 0..2 {
            surface.refresh().expect("original preview control");
            let preview = surface
                .debug_bounds("extension-import-preview")
                .expect("preview");
            surface.simulate_click(preview.center(), Default::default());
            wait_for_idle(&app, surface);
            surface.update(|window, cx| window.draw(cx).clear(cx));
            surface.run_until_parked();
            surface.update(|window, cx| window.draw(cx).clear(cx));
            let identity = surface.read(|cx| {
                app.read(cx)
                    .import_candidate_identity()
                    .expect("current candidate")
            });
            assert_ne!(
                previous.as_ref(),
                Some(&identity),
                "reopen has a new generation"
            );
            // Use the released animation's own duration, not a wall-clock sleep.
            surface
                .executor()
                .advance_clock(*gpui_kit::component::dialog::ANIMATION_DURATION);
            surface.update(|window, cx| window.draw(cx).clear(cx));
            let cancel = surface
                .debug_bounds("extension-cancel-import")
                .expect("settled Cancel");
            surface.simulate_mouse_up(cancel.center(), MouseButton::Left, Default::default());
            assert_eq!(
                surface.read(|cx| app.read(cx).import_candidate_identity()),
                Some(identity.clone()),
                "an orphan release cannot cancel the reopened candidate"
            );
            surface.simulate_mouse_down(cancel.center(), MouseButton::Left, Default::default());
            surface
                .executor()
                .advance_clock(std::time::Duration::from_millis(50));
            surface.update(|window, cx| window.draw(cx).clear(cx));
            assert_eq!(
                surface.debug_bounds("extension-cancel-import"),
                Some(cancel)
            );
            surface.simulate_mouse_up(cancel.center(), MouseButton::Left, Default::default());
            surface.run_until_parked();
            assert!(surface.read(|cx| app.read(cx).preview.is_none()));
            surface.update(|window, cx| window.draw(cx).clear(cx));
            assert!(!surface.update(|window, cx| window.has_active_dialog(cx)));
            assert_eq!(
                std::fs::read(&config_path).expect("unchanged config"),
                original
            );
            assert!(
                !temp.path().join("extensions").exists(),
                "cancel never stages an install"
            );
            previous = Some(identity);
        }
    }

    fn wait_for_idle(
        application: &Entity<ExtensionApplication>,
        window: &mut gpui_kit::VisualTestContext,
    ) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            window.run_until_parked();
            if window.read(|cx| !application.read(cx).busy) {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "extension operation terminal deadline"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
}
