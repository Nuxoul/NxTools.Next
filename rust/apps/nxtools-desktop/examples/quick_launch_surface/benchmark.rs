use super::*;
use gpui_kit::{ScrollStrategy, Task, UniformListScrollHandle, uniform_list};
use std::{cell::Cell, rc::Rc, time::Duration};

pub(super) const ROWS: usize = 1000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Scenario {
    Idle,
    Background,
    Input,
    Scroll,
    Multiwindow,
}
impl Scenario {
    pub(super) fn parse(value: &str) -> Result<Self, &'static str> {
        match value {
            "idle" => Ok(Self::Idle),
            "background" => Ok(Self::Background),
            "input" => Ok(Self::Input),
            "scroll" => Ok(Self::Scroll),
            "multiwindow" => Ok(Self::Multiwindow),
            _ => Err("unsupported benchmark scenario"),
        }
    }
    fn phase(self) -> Phase {
        match self {
            Self::Input => Phase::Input,
            Self::Scroll => Phase::Scroll,
            _ => Phase::Idle,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Phase {
    Idle,
    Input,
    Scroll,
}
impl Phase {
    fn tag(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Input => "input",
            Self::Scroll => "scroll",
        }
    }
}
pub(super) struct BenchmarkState {
    phase: Phase,
    generation: Rc<Cell<u64>>,
    pub(super) updates: u64,
    scroll: UniformListScrollHandle,
    pub(super) max_rendered_row: Cell<usize>,
    task: Option<Task<()>>,
    lifetime: Option<Task<()>>,
}
impl BenchmarkState {
    pub(super) fn new() -> Self {
        Self {
            phase: Phase::Idle,
            generation: Rc::new(Cell::new(0)),
            updates: 0,
            scroll: UniformListScrollHandle::new(),
            max_rendered_row: Cell::new(0),
            task: None,
            lifetime: None,
        }
    }
}
impl Drop for BenchmarkState {
    fn drop(&mut self) {
        self.generation.set(self.generation.get() + 1);
    }
}

impl MaterialSurface {
    pub(super) fn benchmark_list(&self, cx: &Context<Self>) -> gpui_kit::AnyElement {
        let benchmark = self.benchmark.as_ref().expect("explicit benchmark view");
        uniform_list(
            "native-benchmark-list",
            ROWS,
            cx.processor(|this, range: std::ops::Range<usize>, _, cx| {
                if let Some(state) = this.benchmark.as_ref() {
                    state
                        .max_rendered_row
                        .set(state.max_rendered_row.get().max(range.start));
                }
                range
                    .map(|index| {
                        ListItem::new(SharedString::from(format!("bench-row-{index}")))
                            .selected(this.selected == index)
                            .accessibility_label(SharedString::from(format!("示例记录 {index:04}")))
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .items_center()
                                    .gap_3()
                                    .py_2()
                                    .child(Icon::new(IconName::FileText).size(px(20.0)))
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_1()
                                            .child(format!("示例记录 {index:04}"))
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child("用于原生虚拟列表资源对照"),
                                            ),
                                    ),
                            )
                            .into_any_element()
                    })
                    .collect::<Vec<_>>()
            }),
        )
        .flex_1()
        .min_h_0()
        .track_scroll(&benchmark.scroll)
        .into_any_element()
    }

    pub(super) fn stop_benchmark_phase(&mut self) {
        let Some(state) = self.benchmark.as_mut() else {
            return;
        };
        state.task.take();
        println!(
            "ui.material_benchmark.phase_finished correlation={} phase={} generation={} updates={} visible_row={} max_rendered_row={}",
            self.correlation,
            state.phase.tag(),
            state.generation.get(),
            state.updates,
            state.scroll.logical_scroll_top_index(),
            state.max_rendered_row.get()
        );
        state.generation.set(state.generation.get() + 1);
    }

    pub(super) fn set_benchmark_phase(
        &mut self,
        phase: Phase,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.benchmark.as_mut() else {
            return;
        };
        state.task.take();
        let generation = state.generation.get() + 1;
        state.generation.set(generation);
        state.phase = phase;
        state.updates = 0;
        state.max_rendered_row.set(0);
        state.scroll.scroll_to_item_strict(0, ScrollStrategy::Top);
        let token = state.generation.clone();
        let input = self.input.clone();
        let handle = window.window_handle();
        let reset_input = input.clone();
        let reset_token = token.clone();
        cx.defer(move |cx| {
            if reset_token.get() == generation {
                let _ = handle.update(cx, |_, window, cx| {
                    reset_input.update(cx, |input, cx| input.set_value("", window, cx))
                });
            }
        });
        println!(
            "ui.material_benchmark.phase_started correlation={} phase={} generation={} updates=0 visible_row=0",
            self.correlation,
            phase.tag(),
            generation
        );
        cx.notify();
        if phase == Phase::Idle {
            return;
        }
        let interval = if phase == Phase::Input {
            Duration::from_millis(50)
        } else {
            Duration::from_millis(16)
        };
        let task = cx.spawn(async move |view, cx| {
            let mut updates = 0u64;
            loop {
                cx.background_executor().timer(interval).await;
                if token.get() != generation {
                    break;
                }
                updates += 1;
                if phase == Phase::Input {
                    let value = format!("{:016}", updates % 10_000_000_000_000_000);
                    let input = input.clone();
                    if handle
                        .update(cx, |_, window, cx| {
                            if token.get() == generation {
                                input.update(cx, |input, cx| input.set_value(value, window, cx));
                            }
                        })
                        .is_err()
                    {
                        break;
                    }
                }
                if view
                    .update(cx, |view, cx| {
                        let Some(state) = view.benchmark.as_mut() else {
                            return;
                        };
                        if state.generation.get() != generation {
                            return;
                        }
                        state.updates = updates;
                        if phase == Phase::Scroll {
                            let period = ((ROWS - 20) * 2) as u64;
                            let position = (updates * 2) % period;
                            let row = if position < (ROWS - 20) as u64 {
                                position
                            } else {
                                period - position
                            };
                            state
                                .scroll
                                .scroll_to_item_strict(row as usize, ScrollStrategy::Top);
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        self.benchmark
            .as_mut()
            .expect("retained benchmark state")
            .task = Some(task);
    }

    pub(super) fn arm_benchmark(
        &mut self,
        scenario: Scenario,
        seconds: u64,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        let handle = window.window_handle();
        let correlation = self.correlation;
        let task = cx.spawn(async move |view, cx| {
            // Native frame/material preparation and counter warmup occur before the measured phase.
            cx.background_executor().timer(Duration::from_secs(8)).await;
            if handle
                .update(cx, |_, window, cx| {
                    view.update(cx, |view, cx| {
                        view.set_benchmark_phase(scenario.phase(), window, cx)
                    })
                })
                .is_err()
            {
                return;
            }
            cx.background_executor()
                .timer(Duration::from_secs(seconds))
                .await;
            if view
                .update(cx, |view, _| view.stop_benchmark_phase())
                .is_err()
            {
                return;
            }
            println!("ui.material_benchmark.completed correlation={correlation}");
            cx.background_executor().timer(Duration::from_secs(1)).await;
            cx.update(|cx| {
                for window in cx.windows() {
                    let _ = window.update(cx, |_, window, _| window.remove_window());
                }
            });
        });
        self.benchmark
            .as_mut()
            .expect("explicit benchmark lifetime")
            .lifetime = Some(task);
    }
}
