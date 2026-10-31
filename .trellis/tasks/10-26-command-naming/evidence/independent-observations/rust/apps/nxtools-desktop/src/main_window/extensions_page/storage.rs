//! Original manager surface, official Settings and original deferred Dialog confirmation.
use super::*;
use crate::extensions::storage_management::StorageChange;
use gpui_kit::component::{
    Size, WindowExt,
    dialog::{DialogAction, DialogClose, DialogFooter},
};
use nxtools_extensions::storage::RetainedStorageEntry;
use std::cell::Cell;

impl ExtensionManagerView {
    pub(super) fn open_storage(&mut self, package: Option<String>, cx: &mut Context<Self>) {
        self.storage_open = true;
        let owner = self.application.downgrade();
        cx.defer(move |cx| {
            let _ = owner.update(cx, |app, cx| app.load_storage_page(package, None, cx));
        });
        cx.notify();
    }
    pub(super) fn storage_entry_group(
        &self,
        package: &nxtools_extensions::InstalledPackage,
        cx: &Context<Self>,
    ) -> SettingGroup {
        let view = cx.entity().downgrade();
        let package = package.manifest.id.clone();
        let busy = self.application.read(cx).busy;
        SettingGroup::new()
            .title("扩展数据")
            .description("数据未加密，不适合保存密码；移除时默认保留，重新导入不会自动继承。")
            .item(
                SettingItem::render(move |_, _, _| {
                    let package = package.clone();
                    extensions_settings::view_button(
                        "extension-package-storage-open",
                        "管理保留数据",
                        busy,
                        view.clone(),
                        move |view, _, cx| view.open_storage(Some(package.clone()), cx),
                    )
                    .with_size(Size::Small)
                })
                .keywords(["数据", "恢复", "清理"]),
            )
    }
    pub(super) fn render_storage(&self, cx: &mut Context<Self>) -> AnyElement {
        let app = self.application.read(cx);
        let state = &app.storage_management;
        let busy = app.busy;
        let view = cx.entity().downgrade();
        let owner = self.application.downgrade();
        let package = state.package.clone();
        let next = state.page.next_after;
        let current = state.reviewed.as_ref() == Some(&app.packages);
        let pending = state.pending();
        let cancelling = state.cancelling();
        let message = state.message;
        let mut items = Vec::new();
        for (index, entry) in state.page.items.iter().cloned().enumerate() {
            let reviewed = state.reviewed.clone();
            let restore_allowed = !entry.storage.cleanup_pending
                && app.packages.packages.iter().any(|package| {
                    package.manifest.id == entry.package
                        && package.trust.is_some()
                        && package
                            .manifest
                            .runtime
                            .as_ref()
                            .is_some_and(|runtime| runtime.protocol_version == 2)
                });
            let view = view.clone();
            let selector = format!("extension-storage-record-{index}");
            items.push(SettingItem::render(move |_, _, _| {
                let clear_entry = entry.clone();
                let restore_entry = entry.clone();
                let clear_review = reviewed.clone();
                let restore_review = reviewed.clone();
                div()
                    .debug_selector({
                        let selector = selector.clone();
                        move || selector.clone()
                    })
                    .w_full()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(format!(
                        "{} · 修订 {} · {} 字节 · {}",
                        entry.package,
                        entry.storage.revision,
                        entry.storage.bytes,
                        if entry.storage.cleanup_pending {
                            "待清理（不可恢复）"
                        } else {
                            "已保留"
                        }
                    ))
                    .child(
                        div()
                            .text_sm()
                            .child(format!("数据代次：{}", entry.storage.namespace)),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_2()
                            .child(
                                extensions_settings::view_button(
                                    format!("extension-storage-restore-{index}"),
                                    "恢复到当前安装",
                                    busy || !current || !restore_allowed,
                                    view.clone(),
                                    move |view, window, cx| {
                                        if let Some(reviewed) = restore_review.clone() {
                                            view.confirm_storage(
                                                reviewed,
                                                restore_entry.clone(),
                                                true,
                                                window,
                                                cx,
                                            );
                                        }
                                    },
                                )
                                .with_size(Size::Small),
                            )
                            .child(
                                extensions_settings::view_button(
                                    format!("extension-storage-clear-{index}"),
                                    if entry.storage.cleanup_pending {
                                        "重试清理"
                                    } else {
                                        "清理数据"
                                    },
                                    busy || !current,
                                    view.clone(),
                                    move |view, window, cx| {
                                        if let Some(reviewed) = clear_review.clone() {
                                            view.confirm_storage(
                                                reviewed,
                                                clear_entry.clone(),
                                                false,
                                                window,
                                                cx,
                                            );
                                        }
                                    },
                                )
                                .with_size(Size::Small),
                            ),
                    )
            }));
        }
        let header = div()
            .flex_shrink_0()
            .w_full()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(extensions_settings::view_button(
                "extension-storage-back",
                "返回",
                false,
                view.clone(),
                |view, _, _| view.storage_open = false,
            ))
            .child(div().flex_1().child("保留的扩展数据"))
            .child(
                extensions_settings::app_button(
                    "extension-storage-refresh",
                    "刷新首批",
                    busy,
                    owner.clone(),
                    {
                        let package = package.clone();
                        move |app, cx| app.load_storage_page(package.clone(), None, cx)
                    },
                )
                .with_size(Size::Small),
            )
            .child(
                extensions_settings::app_button(
                    "extension-storage-next",
                    "下一批",
                    busy || !current || next.is_none(),
                    owner.clone(),
                    {
                        let package = package.clone();
                        move |app, cx| app.load_storage_page(package.clone(), next, cx)
                    },
                )
                .with_size(Size::Small),
            )
            .child(
                extensions_settings::view_button(
                    "extension-storage-retry-cleanup",
                    "重试待清理",
                    busy || !current,
                    view.clone(),
                    |view, window, cx| {
                        let reviewed = view.application.read(cx).packages.clone();
                        view.confirm_storage_change(
                            reviewed,
                            StorageChange::RetryCleanup,
                            window,
                            cx,
                        );
                    },
                )
                .with_size(Size::Small),
            );
        let empty = items.is_empty();
        let controls = SettingGroup::new().item(SettingItem::render(move |_, _, _| {
            let mut status = div().w_full().flex().flex_col().gap_2()
                .child("仅显示每批最多 16 条元数据，不读取数据值。恢复要求同标识的受信任 v2 安装且目标为空，不合并或覆盖。清理不会删除 Windows 凭据，也不承诺安全擦除。")
                .when(pending, |body| body.child(if cancelling { "已请求取消；等待实际工作结束。" } else { "正在处理，请勿重复提交。" }))
                .when(pending, |body| body.child(extensions_settings::app_button("extension-storage-cancel-operation", "请求取消", cancelling, owner.clone(), |app,cx| app.cancel_storage_management(cx)).with_size(Size::Small)));
            if let Some((message,error)) = message {
                status = status.child(if error { Alert::error("extension-storage-status",message).small() } else { Alert::info("extension-storage-status",message).small() });
            }
            if !pending && !current {
                status = status.child(Alert::warning("extension-storage-review-required", "请刷新当前记录后再操作；旧记录不能授权恢复或清理。").small());
            } else if !pending && empty {
                status = status.child("当前批次没有保留数据。");
            }
            status
        }));
        // Each record remains an individual official item; no second list/query controller.
        let records = SettingGroup::new().title("数据记录").items(items);
        div()
            .size_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_2()
            .child(header)
            .child(
                div().flex_1().min_h_0().child(
                    Settings::new("extension-storage-settings")
                        .header_style(&gpui_kit::StyleRefinement {
                            display: Some(gpui_kit::Display::None),
                            ..Default::default()
                        })
                        .pages([SettingPage::new("保留数据")
                            .resettable(false)
                            .groups([controls, records])]),
                ),
            )
            .into_any_element()
    }
    fn confirm_storage(
        &self,
        reviewed: nxtools_extensions::PackageRegistry,
        entry: RetainedStorageEntry,
        restore: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let change = if restore {
            StorageChange::Restore(entry)
        } else {
            StorageChange::Clear(entry)
        };
        self.confirm_storage_change(reviewed, change, window, cx);
    }
    fn confirm_storage_change(
        &self,
        reviewed: nxtools_extensions::PackageRegistry,
        change: StorageChange,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let current = |app: &ExtensionApplication| match &change {
            StorageChange::Restore(entry) | StorageChange::Clear(entry) => {
                app.storage_review_current(&reviewed, entry)
            }
            StorageChange::RetryCleanup => app.storage_registry_current(&reviewed),
        };
        if !current(self.application.read(cx)) {
            return;
        }
        let owner = self.application.downgrade();
        let source = window.window_handle();
        cx.defer(move |cx|{let _=source.update(cx,|_,window,cx| {
            let submitted=Rc::new(Cell::new(false));
            window.open_dialog(cx,move |dialog,window,cx| {
                let current=owner.upgrade().is_some_and(|owner|match &change {StorageChange::Restore(entry)|StorageChange::Clear(entry)=>owner.read(cx).storage_review_current(&reviewed,entry),StorageChange::RetryCleanup=>owner.read(cx).storage_registry_current(&reviewed)});
                let confirm_owner=owner.clone();let confirm_review=reviewed.clone();let confirm_change=change.clone();let latch=submitted.clone();
                let restore=matches!(change,StorageChange::Restore(_));
                let retry=matches!(change,StorageChange::RetryCleanup);
                let description=if restore {"只会恢复到当前受信任且为空的同标识安装，并使用新的数据标识。不会合并、覆盖或恢复旧信任/设置；取消或失败不自动重试。"} else if retry {"仅重试此前已明确授权的待清理文件（包括恢复后旧锁文件），不会授权删除普通保留数据、当前安装或 Windows 凭据。文件仍被占用时保留容量；不会自动重试。"} else {"此操作不可恢复，只清理所审查的数据代次，不修改插件配置或 Windows 凭据。清理受阻时仍保留占用，需刷新记录后明确重试。"};
                dialog.title(if restore {"确认恢复扩展数据"}else if retry {"确认重试已授权清理"}else{"确认清理扩展数据"}).overlay_closable(false).w((window.viewport_size().width-px(32.0)).min(px(560.0)))
                    .child(div().id("extension-storage-confirm-scroll").w_full().max_h((window.viewport_size().height-px(190.0)).max(px(80.0))).overflow_y_scroll().flex().flex_col().gap_2()
                        .child(match &change {StorageChange::Restore(entry)|StorageChange::Clear(entry)=>format!("{} · 修订 {} · {} 字节",entry.package,entry.storage.revision,entry.storage.bytes),StorageChange::RetryCleanup=>"仅处理已经授权的待清理记录；不删除普通保留数据。".into()})
                        .child(Alert::warning("extension-storage-confirm-warning",description).small())
                        .when(!current,|body|body.child(Alert::error("extension-storage-confirm-stale","记录或扩展配置已变化；请取消并刷新。").small())))
                    .footer(DialogFooter::new()
                        .child(DialogClose::new().trigger(|button|button.debug_selector(||"extension-storage-cancel".into()).label("取消").outline()))
                        .child(DialogAction::new().child(Button::new("extension-storage-confirm").debug_selector(||"extension-storage-confirm".into()).label(if restore {"确认恢复"}else if retry {"确认重试"}else{"确认清理"}).danger().disabled(!current||submitted.get())
                            .on_click(|event,window,cx| {
                                if matches!(event,gpui_kit::ClickEvent::Keyboard(_)) {
                                    window.dispatch_action(Box::new(gpui_kit::component::dialog::Confirm {secondary:false}),cx);
                                }
                            }))))
                    .on_ok(move |_,_,cx| {
                        if latch.get()||!confirm_owner.upgrade().is_some_and(|owner|match &confirm_change {StorageChange::Restore(entry)|StorageChange::Clear(entry)=>owner.read(cx).storage_review_current(&confirm_review,entry),StorageChange::RetryCleanup=>owner.read(cx).storage_registry_current(&confirm_review)}){return false;}
                        latch.set(true);
                        let owner=confirm_owner.clone();let reviewed=confirm_review.clone();let change=confirm_change.clone();
                        cx.defer(move |cx|{let _=owner.update(cx,|app,cx|app.change_retained_storage(reviewed,change,cx));});
                        true
                    })
            });
        });});
    }
}
