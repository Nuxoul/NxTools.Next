use gpui_kit::{AnyWindowHandle, App, Bounds, DisplayId, Pixels, point, px, size};
use nxtools_protocol::CursorMonitorSnapshot;

#[derive(Clone, Copy)]
pub(crate) struct MonitorPlacement {
    pub work_area: Bounds<Pixels>,
    pub display_id: Option<DisplayId>,
    pub scale: f32,
}

impl MonitorPlacement {
    pub(crate) fn centered(&self, desired: gpui_kit::Size<Pixels>) -> Bounds<Pixels> {
        let fitted = size(
            desired.width.min(self.work_area.size.width),
            desired.height.min(self.work_area.size.height),
        );
        Bounds::new(
            self.work_area.origin
                + point(
                    (self.work_area.size.width - fitted.width) / 2.0,
                    (self.work_area.size.height - fitted.height) / 2.0,
                ),
            fitted,
        )
    }
    pub(crate) fn from_snapshot(snapshot: CursorMonitorSnapshot, cx: &App) -> Self {
        let scale = snapshot.dpi.max(96) as f32 / 96.0;
        Self {
            work_area: Bounds::new(
                point(
                    px(snapshot.work_left as f32 / scale),
                    px(snapshot.work_top as f32 / scale),
                ),
                size(
                    px(snapshot.work_width as f32 / scale),
                    px(snapshot.work_height as f32 / scale),
                ),
            ),
            display_id: cx
                .displays()
                .into_iter()
                .find(|display| u64::from(display.id()) == snapshot.display_id)
                .map(|display| display.id()),
            scale,
        }
    }
}

pub(crate) fn cursor_monitor_placement(
    snapshot: Option<CursorMonitorSnapshot>,
    display_index: u32,
    dpi: u32,
    cx: &App,
) -> MonitorPlacement {
    if let Some(snapshot) = snapshot {
        return MonitorPlacement::from_snapshot(snapshot, cx);
    }
    let displays = cx.displays();
    let display = displays
        .get(display_index as usize)
        .cloned()
        .or_else(|| cx.primary_display());
    MonitorPlacement {
        work_area: display
            .as_ref()
            .map(|display| display.visible_bounds())
            .unwrap_or_else(|| Bounds::new(point(px(0.0), px(0.0)), size(px(1920.0), px(1080.0)))),
        display_id: display.map(|display| display.id()),
        scale: dpi.max(96) as f32 / 96.0,
    }
}

pub(crate) fn owner_monitor_placement(
    snapshot: Option<CursorMonitorSnapshot>,
    owner: Option<AnyWindowHandle>,
    cx: &mut App,
) -> MonitorPlacement {
    let display = owner.and_then(|owner| {
        owner
            .update(cx, |_, window, cx| {
                window
                    .display(cx)
                    .map(|display| (display, window.scale_factor()))
            })
            .ok()
            .flatten()
    });
    if let Some(snapshot) = snapshot
        && display
            .as_ref()
            .is_none_or(|(display, _)| u64::from(display.id()) == snapshot.display_id)
    {
        return MonitorPlacement::from_snapshot(snapshot, cx);
    }
    if let Some((display, scale)) = display {
        return MonitorPlacement {
            work_area: display.visible_bounds(),
            display_id: Some(display.id()),
            scale,
        };
    }
    cursor_monitor_placement(snapshot, 0, snapshot.map_or(96, |monitor| monitor.dpi), cx)
}

pub(crate) fn active_monitor_placement(cx: &mut App) -> MonitorPlacement {
    let owner = cx.windows().into_iter().find(|handle| {
        handle
            .update(cx, |_, window, _| window.is_window_active())
            .unwrap_or(false)
    });
    owner_monitor_placement(None, owner, cx)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[gpui_kit::test]
    fn multimonitor_auxiliary_center_stays_in_negative_work_area(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        for dpi in [96, 120, 144] {
            let snapshot = CursorMonitorSnapshot {
                display_id: 0xCAFE,
                work_left: -1920,
                work_top: -1080,
                work_width: 1920,
                work_height: 1040,
                dpi,
            };
            let placement = cx.update(|cx| MonitorPlacement::from_snapshot(snapshot, cx));
            for desired in [size(px(560.0), px(260.0)), size(px(2400.0), px(1600.0))] {
                let bounds = placement
                    .centered(desired)
                    .to_device_pixels(placement.scale);
                assert!(i32::from(bounds.origin.x) >= -1920);
                assert!(i32::from(bounds.origin.y) >= -1080);
                assert!(i32::from(bounds.origin.x) + i32::from(bounds.size.width) <= 0);
                assert!(i32::from(bounds.origin.y) + i32::from(bounds.size.height) <= -40);
            }
        }
    }
}
