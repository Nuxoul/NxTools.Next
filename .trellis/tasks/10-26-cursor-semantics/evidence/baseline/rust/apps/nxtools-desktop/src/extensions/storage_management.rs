//! Host-only finite management state on the original application entity.
use super::*;
use nxtools_extensions::storage::{RetainedStorageEntry, RetainedStoragePage, StorageError};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Default)]
pub(crate) struct StorageManagement {
    pub(crate) page: RetainedStoragePage,
    pub(crate) reviewed: Option<PackageRegistry>,
    pub(crate) package: Option<String>,
    pub(crate) message: Option<(&'static str, bool)>,
    pending: Option<(uuid::Uuid, Arc<AtomicBool>)>,
}
impl StorageManagement {
    pub(crate) fn pending(&self) -> bool {
        self.pending.is_some()
    }
    pub(crate) fn cancelling(&self) -> bool {
        self.pending
            .as_ref()
            .is_some_and(|(_, cancel)| cancel.load(Ordering::Acquire))
    }
}
impl Drop for StorageManagement {
    fn drop(&mut self) {
        if let Some((_, cancel)) = &self.pending {
            cancel.store(true, Ordering::Release);
        }
    }
}
impl ExtensionApplication {
    pub(crate) fn load_storage_page(
        &mut self,
        package: Option<String>,
        after: Option<uuid::Uuid>,
        cx: &mut Context<Self>,
    ) {
        if self.busy {
            return;
        }
        let Some(service) = self.service.clone() else {
            self.storage_management.message =
                Some(("扩展数据通道不可用；未读取或清理数据。", true));
            cx.notify();
            return;
        };
        let id = uuid::Uuid::new_v4();
        let cancel = Arc::new(AtomicBool::new(false));
        self.busy = true;
        self.storage_management.pending = Some((id, cancel.clone()));
        self.storage_management.reviewed = None;
        self.storage_management.message = None;
        self.storage_management.package = package.clone();
        let reviewed = self.packages.clone();
        let path = self.config_path.clone();
        let started = Instant::now();
        info!(event="extension.storage.management.accepted",correlation_id=%id,action="review");
        let background = cx.background_executor().clone();
        cx.spawn(async move |this,cx| {
            let worker_review=reviewed.clone();
            let outcome=background.spawn(async move {
                let span=tracing::info_span!("extension.storage.management",correlation_id=%id,action="review"); let _entered=span.enter();
                info!(event="extension.storage.management.started");
                anyhow::ensure!(!cancel.load(Ordering::Acquire),StorageError::Cancelled);
                service.with_storage_management(|storage| AppConfig::review_retained_extension_data_checked(&path,&worker_review,storage,package,after))
            }).await;
            let _=this.update(cx,|app,cx| {
                if !app.storage_management.pending.as_ref().is_some_and(|(pending,_)|*pending==id) { return; }
                let Some((_,cancel))=app.storage_management.pending.take() else { return; };
                app.busy=false;
                let reason=if cancel.load(Ordering::Acquire) { "cancelled" } else {
                    match outcome {
                        Ok(page) if app.packages==reviewed => {
                            app.storage_management.page=page;
                            app.storage_management.reviewed=Some(reviewed);
                            "success"
                        }
                        _ => { app.storage_management.message=Some(("未能读取当前数据记录；原数据未改变，请刷新后重试。",true)); "review_failed" }
                    }
                };
                info!(event="extension.storage.management.terminal",correlation_id=%id,action="review",reason,elapsed_ms=started.elapsed().as_millis());
                cx.notify();
            });
        }).detach();
        cx.notify();
    }
    pub(crate) fn cancel_storage_management(&mut self, cx: &mut Context<Self>) {
        if let Some((id, cancel)) = &self.storage_management.pending {
            if !cancel.swap(true, Ordering::AcqRel) {
                info!(event="extension.storage.management.cancel_requested",correlation_id=%id);
            }
            cx.notify();
        }
    }
    pub(crate) fn storage_review_current(
        &self,
        reviewed: &PackageRegistry,
        entry: &RetainedStorageEntry,
    ) -> bool {
        !self.busy
            && &self.packages == reviewed
            && self.storage_management.reviewed.as_ref() == Some(reviewed)
            && self.storage_management.page.items.contains(entry)
    }
    pub(crate) fn change_retained_storage(
        &mut self,
        reviewed: PackageRegistry,
        entry: RetainedStorageEntry,
        restore: bool,
        cx: &mut Context<Self>,
    ) {
        if !self.storage_review_current(&reviewed, &entry) {
            return;
        }
        let Some(service) = self.service.clone() else {
            return;
        };
        let target = reviewed
            .packages
            .iter()
            .find(|package| package.manifest.id == entry.package)
            .cloned();
        if restore && (entry.storage.cleanup_pending || target.is_none()) {
            return;
        }
        let id = uuid::Uuid::new_v4();
        let cancel = Arc::new(AtomicBool::new(false));
        self.busy = true;
        self.storage_management.pending = Some((id, cancel.clone()));
        self.storage_management.reviewed = None;
        self.storage_management.message = None;
        let path = self.config_path.clone();
        let action = if restore { "restore" } else { "clear" };
        let started = Instant::now();
        info!(event="extension.storage.management.accepted",correlation_id=%id,action);
        let background = cx.background_executor().clone();
        cx.spawn(async move |this,cx| {
            let outcome=background.spawn(async move {
                let span=tracing::info_span!("extension.storage.management",correlation_id=%id,action); let _entered=span.enter();
                info!(event="extension.storage.management.started");
                anyhow::ensure!(!cancel.load(Ordering::Acquire),StorageError::Cancelled);
                if restore {
                    let storage=service.storage_owner().ok_or(StorageError::Unavailable)?;
                    let target=target.ok_or(StorageError::Stale)?;
                    service.commit_config_with_result(&path,|path|AppConfig::restore_extension_data_checked(path,&target,&entry.storage,&storage,&||cancel.load(Ordering::Acquire)))
                        .map(|(config,applied,pending)|(Some(config),applied,pending))
                } else {
                    service.with_storage_management(|storage| {
                        anyhow::ensure!(!cancel.load(Ordering::Acquire),StorageError::Cancelled);
                        AppConfig::clear_retained_extension_data_checked(&path,&entry.package,&reviewed,&entry.storage,storage)
                    }).map(|pending|(None,true,pending))
                }
            }).await;
            let _=this.update(cx,|app,cx| {
                if !app.storage_management.pending.as_ref().is_some_and(|(pending,_)|*pending==id) {return;}
                app.storage_management.pending=None;
                app.busy=false;
                app.storage_management.page=Default::default();
                let reason=match outcome {
                    Ok((config,applied,pending))=>{
                        if let Some(config)=config {
                            app.packages=config.extensions;
                            app.refresh_committed_storage_metadata();
                            app.commit_revision=app.commit_revision.wrapping_add(1);
                            app.last_commit=Some(("storage_restore",true,applied));
                            let quick=app.quick_launch.clone(); let snapshot=app.packages.clone();
                            cx.defer(move |cx|{let _=quick.update(cx,|quick,cx|quick.apply_extension_snapshot(snapshot,cx));});
                        }
                        let message=match (restore,applied,pending) {
                            (true,false,_)=>"数据恢复已提交，但运行时未能应用；请检查后刷新。",
                            (true,true,true)=>"数据已恢复到新安装标识，旧记录待清理；请刷新后检查。",
                            (true,true,false)=>"数据已恢复；旧标识已退役。请刷新查看当前记录。",
                            (false,_,true)=>"清理已确认，但数据仍待清理；请刷新记录并明确重试。",
                            (false,_,false)=>"保留数据已清理；配置和 Windows 凭据未改变。",
                        };
                        app.storage_management.message=Some((message,!applied));
                        app.notify_operation(format!("extension-storage-{id}"),if !applied{NotificationLevel::Error}else if pending{NotificationLevel::Info}else{NotificationLevel::Success},message,cx);
                        if pending {"cleanup_pending"} else if !applied {"committed_not_applied"} else {"success"}
                    }
                    Err(error)=>{
                        let cancelled=error.downcast_ref::<StorageError>()==Some(&StorageError::Cancelled);
                        app.storage_management.message=Some((if cancelled {"已请求取消；未确认的操作不会自动重试，请刷新当前记录。"}else{"操作未完成；可能是记录已变化、目标非空或配置/数据不可写。请刷新后重新确认。"},!cancelled));
                        if cancelled {"cancelled"}else{"review_or_io_failed"}
                    }
                };
                info!(event="extension.storage.management.terminal",correlation_id=%id,action,reason,elapsed_ms=started.elapsed().as_millis());
                cx.notify();
            });
        }).detach();
        cx.notify();
    }
}
