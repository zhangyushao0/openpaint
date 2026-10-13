//! The harness's global allocator (S51): the system's, counting the bytes live and the most live
//! at once, so `long-session` tells the Rust heap (the app's and its libraries': fjall, wgpu,
//! egui) apart from what the GPU driver keeps in the process. The harness only: no product crate
//! counts, and the app on the tablet allocates through the system as before.
#![expect(
    unsafe_code,
    reason = "a global allocator implements the unsafe `GlobalAlloc`; each call goes to the \
              system's allocator unchanged, only counted"
)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

/// The system's allocator, counted.
struct Counting;

#[global_allocator]
static HEAP: Counting = Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller's contract for `alloc`, passed on.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            grew(layout.size());
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller's contract for `alloc_zeroed`, passed on.
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            grew(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: the caller's contract for `dealloc`, passed on.
        unsafe { System.dealloc(pointer, layout) };
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: the caller's contract for `realloc`, passed on.
        let moved = unsafe { System.realloc(pointer, layout, new_size) };
        if !moved.is_null() {
            match new_size.checked_sub(layout.size()) {
                Some(more) => grew(more),
                None => {
                    LIVE.fetch_sub(layout.size() - new_size, Ordering::Relaxed);
                }
            }
        }
        moved
    }
}

fn grew(bytes: usize) {
    let live = LIVE.fetch_add(bytes, Ordering::Relaxed) + bytes;
    PEAK.fetch_max(live, Ordering::Relaxed);
}

/// Bytes the Rust heap holds now, and the most it held at once since the process started.
pub fn bytes() -> [u64; 2] {
    let read = |count: &AtomicUsize| count.load(Ordering::Relaxed) as u64;
    [read(&LIVE), read(&PEAK)]
}
