//! Bounded selected-detail IO on the original engine/runtime and application owner.
use super::*;
use nxtools_download::{DownloadFault, DownloadFaultCode};
use std::cell::Cell;

#[derive(Clone)]
pub(crate) struct DownloadDetailScope {
    task_id: DownloadTaskId,
    live: Rc<Cell<bool>>,
}
impl DownloadDetailScope {
    pub(crate) fn new(task_id: DownloadTaskId) -> Self {
        Self {
            task_id,
            live: Rc::new(Cell::new(true)),
        }
    }
    pub(crate) fn task_id(&self) -> DownloadTaskId {
        self.task_id
    }
    pub(crate) fn retire(&self) {
        self.live.set(false);
    }
    pub(crate) fn is_live(&self) -> bool {
        self.live.get()
    }
    fn same(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.live, &other.live)
    }
}
#[derive(Clone)]
struct DetailRequest {
    task_id: DownloadTaskId,
    generation: u64,
    correlation_id: uuid::Uuid,
    engine_generation: u64,
    scope: DownloadDetailScope,
    started_at: Instant,
}
#[derive(Default)]
pub(super) struct DetailRequests {
    generation: u64,
    target: Option<DetailRequest>,
    pending: Option<DetailRequest>,
    running: Option<DetailRequest>,
    rejection_receipts: HashSet<uuid::Uuid>,
    cache_dirty: bool,
    #[cfg(test)]
    dispatched: Vec<(DownloadTaskId, u64)>,
    #[cfg(test)]
    published: Vec<(DownloadTaskId, u64)>,
    #[cfg(test)]
    discarded: Vec<(DownloadTaskId, u64)>,
}
impl DownloadApplication {
    pub(super) fn claim_detail_rejection(
        &mut self,
        request_id: uuid::Uuid,
        cx: &mut Context<Self>,
    ) -> bool {
        let claimed = self.detail_requests.rejection_receipts.remove(&request_id);
        if claimed {
            self.start_pending_task_detail(cx);
        }
        claimed
    }

    #[cfg(test)]
    pub(crate) fn detail_request_state_for_test(&self) -> (u64, bool, bool) {
        (
            self.detail_requests.generation,
            self.detail_requests.running.is_some(),
            self.detail_requests.pending.is_some(),
        )
    }
    #[cfg(test)]
    pub(crate) fn detail_request_events_for_test(&self) -> (usize, usize, usize) {
        (
            self.detail_requests.dispatched.len(),
            self.detail_requests.published.len(),
            self.detail_requests.discarded.len(),
        )
    }
    pub(crate) fn request_task_detail(&mut self, task_id: DownloadTaskId, cx: &mut Context<Self>) {
        let scope = self
            .detail_requests
            .target
            .as_ref()
            .filter(|request| request.task_id == task_id && request.scope.is_live())
            .map(|request| request.scope.clone())
            .unwrap_or_else(|| DownloadDetailScope::new(task_id));
        self.request_task_detail_scoped(task_id, scope, false, cx);
    }

    pub(crate) fn bind_task_detail_scope(&mut self, scope: DownloadDetailScope) {
        if !scope.is_live() {
            return;
        }
        if let Some(previous) = self.detail_requests.target.as_ref()
            && !previous.scope.same(&scope)
        {
            previous.scope.retire();
        }
        // Bind before the original request entry. Pending/dispatch is installed there.
        if self
            .detail_requests
            .target
            .as_ref()
            .is_none_or(|request| !request.scope.same(&scope))
        {
            self.detail_requests.generation =
                self.detail_requests.generation.wrapping_add(1).max(1);
            self.detail_requests.target = Some(DetailRequest {
                task_id: scope.task_id,
                generation: self.detail_requests.generation,
                correlation_id: uuid::Uuid::new_v4(),
                engine_generation: self.event_generation,
                scope,
                started_at: Instant::now(),
            });
        }
    }

    fn request_task_detail_scoped(
        &mut self,
        task_id: DownloadTaskId,
        scope: DownloadDetailScope,
        force: bool,
        cx: &mut Context<Self>,
    ) {
        if !scope.is_live() || scope.task_id != task_id {
            return;
        }
        if force {
            self.detail_requests.cache_dirty = true;
        }
        if !force
            && self
                .detail_requests
                .target
                .as_ref()
                .is_some_and(|target| target.task_id == task_id && target.scope.same(&scope))
            && (self
                .detail_requests
                .pending
                .as_ref()
                .is_some_and(|request| request.generation == self.detail_requests.generation)
                || self
                    .detail_requests
                    .running
                    .as_ref()
                    .is_some_and(|request| request.generation == self.detail_requests.generation))
        {
            return;
        }
        self.bind_task_detail_scope(scope.clone());
        self.detail_requests.generation = self.detail_requests.generation.wrapping_add(1).max(1);
        let request = DetailRequest {
            task_id,
            generation: self.detail_requests.generation,
            correlation_id: uuid::Uuid::new_v4(),
            engine_generation: self.event_generation,
            scope,
            started_at: Instant::now(),
        };
        self.detail_requests.target = Some(request.clone());
        self.detail_requests.pending = None;
        if !force && !self.detail_requests.cache_dirty && self.task_detail(task_id).is_some() {
            info!(event="download.desktop.detail.cache_hit", correlation_id=%request.correlation_id, task_id=%task_id.as_uuid(), generation=request.generation, "selected detail cache hit");
            return;
        }
        if self
            .task_detail
            .as_ref()
            .is_some_and(|detail| detail.task.task.id() != task_id)
        {
            self.task_detail = None;
        }
        self.task_detail_error = None;
        // Install pending before any dispatch/fast completion can be observed.
        self.detail_requests.pending = Some(request.clone());
        info!(event="download.desktop.detail.accepted", correlation_id=%request.correlation_id, task_id=%task_id.as_uuid(), generation=request.generation, "selected detail accepted");
        self.start_pending_task_detail(cx);
        cx.notify();
    }

    pub(crate) fn retire_task_detail_scope(&mut self, scope: &DownloadDetailScope) {
        scope.retire();
        if self
            .detail_requests
            .target
            .as_ref()
            .is_some_and(|request| request.scope.same(scope))
        {
            self.detail_requests.generation =
                self.detail_requests.generation.wrapping_add(1).max(1);
            self.detail_requests.target = None;
            self.detail_requests.pending = None;
        }
        // Actual running IO remains occupied until its reply/terminal failure.
    }

    pub(super) fn refresh_selected_task_detail(&mut self, cx: &mut Context<Self>) {
        let Some(target) = self.detail_requests.target.clone() else {
            return;
        };
        if !target.scope.is_live()
            || !self
                .tasks
                .iter()
                .any(|task| task.task.id() == target.task_id)
        {
            self.retire_task_detail_scope(&target.scope);
            self.task_detail = None;
            self.task_detail_error = None;
            return;
        }
        self.request_task_detail_scoped(target.task_id, target.scope, true, cx);
    }

    fn start_pending_task_detail(&mut self, cx: &mut Context<Self>) {
        if self.detail_requests.running.is_some()
            || self.detail_requests.rejection_receipts.len() >= 8
        {
            return;
        }
        let Some(request) = self.detail_requests.pending.take() else {
            return;
        };
        if !request.scope.is_live() {
            return;
        }
        self.detail_requests.running = Some(request.clone());
        self.detail_requests
            .rejection_receipts
            .insert(request.correlation_id);
        #[cfg(test)]
        self.detail_requests
            .dispatched
            .push((request.task_id, request.generation));
        info!(event="download.desktop.detail.started", correlation_id=%request.correlation_id, task_id=%request.task_id.as_uuid(), generation=request.generation, "selected detail IO started");
        let receiver = self
            .engine
            .request_task_detail_background(request.task_id, request.correlation_id);
        if receiver.is_err() {
            self.detail_requests
                .rejection_receipts
                .remove(&request.correlation_id);
        }
        cx.spawn(async move |this, cx| {
            let result = match receiver {
                Ok(receiver) => receiver.recv().await.unwrap_or_else(|_| {
                    Err(DownloadFault {
                        code: DownloadFaultCode::Unavailable,
                        message: "下载详情读取已中断，请重新选择任务重试".into(),
                        retryable: true,
                    })
                }),
                Err(fault) => Err(fault),
            };
            let deferred = this.clone();
            let _ = this.update(cx, |_, cx| {
                cx.defer(move |cx| {
                    let _ =
                        deferred.update(cx, |app, cx| app.finish_task_detail(request, result, cx));
                });
            });
        })
        .detach();
    }

    fn finish_task_detail(
        &mut self,
        request: DetailRequest,
        result: Result<DownloadTaskDetail, DownloadFault>,
        cx: &mut Context<Self>,
    ) {
        if self
            .detail_requests
            .running
            .as_ref()
            .is_none_or(|running| running.correlation_id != request.correlation_id)
        {
            return;
        }
        self.detail_requests.running = None;
        if result.is_ok() {
            self.detail_requests
                .rejection_receipts
                .remove(&request.correlation_id);
        }
        let current = request.scope.is_live()
            && self.detail_requests.target.as_ref().is_some_and(|target| {
                target.task_id == request.task_id
                    && target.generation == request.generation
                    && target.correlation_id == request.correlation_id
            })
            && request.engine_generation == self.event_generation
            && self
                .tasks
                .iter()
                .any(|task| task.task.id() == request.task_id);
        if current {
            #[cfg(test)]
            self.detail_requests
                .published
                .push((request.task_id, request.generation));
            match result {
                Ok(mut detail) if detail.task.task.id() == request.task_id => {
                    self.detail_requests.cache_dirty = false;
                    if let Some(progress) = self.latest_progress.get(&request.task_id) {
                        apply_progress_to_task_detail(&mut detail, progress);
                    }
                    self.task_detail = Some(detail);
                    self.task_detail_error = None;
                    info!(event="download.desktop.detail.completed", correlation_id=%request.correlation_id, task_id=%request.task_id.as_uuid(), generation=request.generation, elapsed_ms=request.started_at.elapsed().as_millis(), "selected detail published");
                }
                outcome => {
                    let fault = outcome.err().unwrap_or(DownloadFault {
                        code: DownloadFaultCode::Internal,
                        message: "下载详情身份不匹配，请重新选择任务重试".into(),
                        retryable: true,
                    });
                    warn!(event="download.desktop.detail.failed", correlation_id=%request.correlation_id, task_id=%request.task_id.as_uuid(), generation=request.generation, fault_code=?fault.code, retryable=fault.retryable, elapsed_ms=request.started_at.elapsed().as_millis(), "selected detail read failed");
                    self.task_detail_error = Some((request.task_id, fault.message));
                }
            }
        } else {
            #[cfg(test)]
            self.detail_requests
                .discarded
                .push((request.task_id, request.generation));
            info!(event="download.desktop.detail.retired", correlation_id=%request.correlation_id, task_id=%request.task_id.as_uuid(), generation=request.generation, elapsed_ms=request.started_at.elapsed().as_millis(), "obsolete selected detail discarded");
        }
        self.start_pending_task_detail(cx);
        cx.notify();
    }
}
