//! Current-thread allocator measurements for synthetic indexer fixtures.
#![allow(unsafe_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Allocations {
    pub calls: usize,
    pub requested_bytes: usize,
    pub largest_request: usize,
    pub retained_bytes: usize,
    pub peak_bytes: usize,
}

thread_local! {
    static OBSERVATION: Cell<Option<Allocations>> = const { Cell::new(None) };
}

struct ObservedSystem;

#[global_allocator]
static ALLOCATOR: ObservedSystem = ObservedSystem;

fn record(allocated: usize, freed: usize, allocation: bool) {
    let _ = OBSERVATION.try_with(|cell| {
        if let Some(mut stats) = cell.get() {
            stats.calls += usize::from(allocation);
            stats.requested_bytes = stats.requested_bytes.saturating_add(allocated);
            stats.largest_request = stats.largest_request.max(allocated);
            stats.retained_bytes = stats
                .retained_bytes
                .saturating_add(allocated)
                .saturating_sub(freed);
            stats.peak_bytes = stats.peak_bytes.max(stats.retained_bytes);
            cell.set(Some(stats));
        }
    });
}

// SAFETY: Every pointer/layout is forwarded unchanged to System. Observation uses only
// a const-initialized thread-local Cell and never allocates or dereferences the pointer.
unsafe impl GlobalAlloc for ObservedSystem {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            record(layout.size(), 0, true);
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            record(layout.size(), 0, true);
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
        record(0, layout.size(), false);
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let resized = unsafe { System.realloc(pointer, layout, new_size) };
        if !resized.is_null() {
            record(new_size, layout.size(), true);
        }
        resized
    }
}

pub(crate) fn measure<T>(operation: impl FnOnce() -> T) -> (T, Allocations) {
    struct StopObservation;
    impl Drop for StopObservation {
        fn drop(&mut self) {
            OBSERVATION.with(|cell| cell.set(None));
        }
    }
    OBSERVATION.with(|cell| {
        assert!(cell.get().is_none(), "allocator measurements cannot nest");
        cell.set(Some(Allocations::default()));
    });
    let guard = StopObservation;
    let value = operation();
    let stats = OBSERVATION.with(|cell| cell.get().expect("active observation"));
    drop(guard);
    (value, stats)
}
