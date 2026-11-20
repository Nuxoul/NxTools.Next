use super::*;
use crate::tools::link_detector::{DetectedLink, detect_links};

#[cfg(test)]
type DetectionResolver = std::sync::Arc<dyn Fn(&str) -> Vec<DetectedLink> + Send + Sync>;

#[derive(Clone, Copy)]
struct DetectionTicket {
    id: uuid::Uuid,
    generation: u64,
    pause_generation: u64,
    started: Instant,
}

impl DetectionTicket {
    fn completed(
        self,
        outcome: &'static str,
        count: usize,
        failure: Option<(&'static str, Option<i32>)>,
    ) {
        info!(
            event = "tools.link_detector.detection_completed",
            correlation_id = %self.id,
            outcome,
            success = failure.is_none(),
            match_count = count,
            failure_kind = failure.map_or("none", |(kind, _)| kind),
            native_error_code = failure.and_then(|(_, code)| code),
            elapsed_ms = self.started.elapsed().as_millis() as u64,
            "link detection terminal result"
        );
    }
}

struct DetectionRequest {
    ticket: DetectionTicket,
    text: String,
}

#[derive(Default)]
pub(super) struct LinkDetectionState {
    generation: u64,
    active: Option<DetectionTicket>,
    queued: Option<DetectionRequest>,
    #[cfg(test)]
    resolver: Option<DetectionResolver>,
}

impl LinkDetectionState {
    fn resolver(&self) -> impl FnOnce(&str) -> Vec<DetectedLink> + Send + 'static {
        #[cfg(test)]
        let injected = self.resolver.clone();
        move |text| {
            #[cfg(test)]
            if let Some(resolver) = injected {
                return resolver(text);
            }
            detect_links(text)
        }
    }
}

enum DetectionFailure {
    Spawn(std::io::Error),
    WorkerClosed,
}

impl ClipboardApplication {
    #[cfg(test)]
    pub(crate) fn wait_for_link_detection(cx: &mut gpui_kit::TestAppContext, owner: &Entity<Self>) {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            cx.run_until_parked();
            if cx.read(|cx| {
                let detection = &owner.read(cx).link_detection;
                detection.active.is_none() && detection.queued.is_none()
            }) {
                break;
            }
            assert!(Instant::now() < deadline, "background detection completed");
            std::thread::sleep(Duration::from_millis(1));
        }
        cx.run_until_parked();
    }

    pub(super) fn invalidate_link_detection(&mut self) {
        self.link_detection.generation = self.link_detection.generation.wrapping_add(1);
        if let Some(request) = self.link_detection.queued.take() {
            request.ticket.completed("superseded_before_start", 0, None);
        }
    }

    fn link_detection_is_current(&self, ticket: DetectionTicket) -> bool {
        ticket.generation == self.link_detection.generation
            && ticket.pause_generation == self.capture_pause.generation
            && self.config.clipboard.enabled
            && self.config.clipboard.link_detector_enabled
            && !self.management_operation_active
            && !self.capture_pause.blocks(nxtools_ipc::unix_time_ms())
    }

    pub(super) fn queue_link_detection(&mut self, text: &str, cx: &mut Context<Self>) {
        self.invalidate_link_detection();
        let ticket = DetectionTicket {
            id: uuid::Uuid::new_v4(),
            generation: self.link_detection.generation,
            pause_generation: self.capture_pause.generation,
            started: Instant::now(),
        };
        if !self.link_detection_is_current(ticket) {
            return;
        }
        info!(
            event = "tools.link_detector.detection_accepted",
            correlation_id = %ticket.id,
            queued = self.link_detection.active.is_some(),
            "clipboard detection accepted"
        );
        self.link_detection.queued = Some(DetectionRequest {
            ticket,
            text: text.to_owned(),
        });
        self.start_link_detection(cx);
    }

    fn start_link_detection(&mut self, cx: &mut Context<Self>) {
        if self.link_detection.active.is_some() {
            return;
        }
        let Some(request) = self.link_detection.queued.take() else {
            return;
        };
        let ticket = request.ticket;
        if !self.link_detection_is_current(ticket) {
            ticket.completed("superseded_before_start", 0, None);
            return;
        }
        self.link_detection.active = Some(ticket);
        let resolve = self.link_detection.resolver();
        let (sender, receiver) = async_channel::bounded(1);
        info!(
            event = "tools.link_detector.detection_started",
            correlation_id = %ticket.id,
            "background link detection started"
        );
        let worker = std::thread::Builder::new()
            .name("link-detector".into())
            .spawn(move || {
                let links = resolve(&request.text);
                let _ = sender.send_blocking(links);
            });
        if let Err(error) = worker {
            self.finish_link_detection(ticket, Err(DetectionFailure::Spawn(error)), cx);
            return;
        }
        cx.spawn(async move |owner, cx| {
            let result = receiver
                .recv()
                .await
                .map_err(|_| DetectionFailure::WorkerClosed);
            let _ = owner.update(cx, |owner, cx| {
                owner.finish_link_detection(ticket, result, cx);
            });
        })
        .detach();
    }

    fn finish_link_detection(
        &mut self,
        ticket: DetectionTicket,
        result: Result<Vec<DetectedLink>, DetectionFailure>,
        cx: &mut Context<Self>,
    ) {
        if self
            .link_detection
            .active
            .is_none_or(|active| active.id != ticket.id)
        {
            return;
        }
        self.link_detection.active = None;
        match result {
            Ok(links) => {
                let current = self.link_detection_is_current(ticket);
                let outcome = if !current {
                    "superseded"
                } else if links.is_empty() {
                    "no_matches"
                } else {
                    "matched"
                };
                ticket.completed(outcome, links.len(), None);
                if current && !links.is_empty() {
                    let application = cx.entity().downgrade();
                    cx.defer(move |cx| {
                        if let Some(application) = application.upgrade() {
                            application.update(cx, |application, cx| {
                                if application.link_detection_is_current(ticket) {
                                    info!(
                                        event = "tools.link_detector.matches_found",
                                        correlation_id = %ticket.id,
                                        count = links.len(),
                                        "link detector recognized clipboard targets"
                                    );
                                    application.request_link_panel(links, cx);
                                }
                            });
                        }
                    });
                }
            }
            Err(failure) => {
                let failure = match failure {
                    DetectionFailure::Spawn(error) => ("worker_start", error.raw_os_error()),
                    DetectionFailure::WorkerClosed => ("worker_closed", None),
                };
                ticket.completed("failed", 0, Some(failure));
            }
        }
        self.start_link_detection(cx);
    }
}

#[cfg(test)]
#[path = "link_detection_tests.rs"]
mod tests;
