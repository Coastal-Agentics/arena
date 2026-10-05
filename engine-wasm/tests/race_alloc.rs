//! `WasmRace.step` allocates nothing itself: a whole race of single-tick steps
//! allocates only when the match history (the replay's action log) doubles.

use engine_wasm::RaceViewer;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Counting;
static ALLOCS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.realloc(p, l, n) }
    }
}

#[global_allocator]
static A: Counting = Counting;

#[test]
fn race_steps_allocate_only_for_history_doubling() {
    let b = |id: &str| {
        format!(
            r#"{{"rules_version":1,"levels":{{"power":3,"top_speed":3,"grip":3}},"behavior":{{"kind":"scripted","id":"{id}"}}}}"#
        )
    };
    for builds in [
        vec![b("follower")],
        vec![b("follower"), b("cutter"), b("blocker"), b("cutter")],
    ] {
        let mut v = RaceViewer::from_builds("7", &format!("[{}]", builds.join(","))).unwrap();
        let before = ALLOCS.load(Ordering::Relaxed);
        while !v.step(1) {}
        let allocs = ALLOCS.load(Ordering::Relaxed) - before;
        let ticks = v.inner().tick();
        let doublings = (ticks as f64 * builds.len() as f64).log2().ceil() as usize + 1;
        eprintln!("{} cars, {ticks} ticks: {allocs} allocations", builds.len());
        assert!(
            allocs <= doublings,
            "{allocs} allocations over {ticks} ticks"
        );
    }
}
