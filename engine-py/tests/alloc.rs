//! The session step allocates nothing itself: with every agent learning, a whole
//! episode's steps allocate only when the match history doubles.

use engine::TankRules;
use engine_py::{Game, Session};
use racing::RacingRules;
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

/// Allocations over a whole episode of `step` + `encode_obs`, and the ticks played.
fn episode<R: Game>(builds: &[&str], frame_skip: u32) -> (usize, u32)
where
    R::Config: Clone,
{
    let lineup = R::lineup(builds).unwrap();
    let rows: Vec<usize> = (0..builds.len()).collect();
    let mut s = Session::new(lineup, &rows, frame_skip, 7).unwrap();
    let mut act = vec![0.5; rows.len() * R::ACTION_LEN];
    let mut rew = vec![0.0; rows.len()];
    let mut obs = vec![0.0; rows.len() * R::OBS_LEN];
    let before = ALLOCS.load(Ordering::Relaxed);
    let mut step = 0u32;
    loop {
        act[0] = if step.is_multiple_of(3) { -1.0 } else { 1.0 };
        let over = s.step(&act, &mut rew);
        s.encode_obs(&mut obs);
        step += 1;
        if over {
            break;
        }
    }
    (ALLOCS.load(Ordering::Relaxed) - before, s.game().tick())
}

#[test]
fn steps_allocate_only_for_history_doubling() {
    let t = tank::catalog::DEFAULT_BUILD_JSON;
    let r = racing::catalog::DEFAULT_BUILD_JSON;
    for (allocs, ticks) in [
        episode::<TankRules>(&[t, t], 1),
        episode::<TankRules>(&[t, t], 4),
        episode::<RacingRules>(&[r; 4], 1),
        episode::<RacingRules>(&[r; 4], 4),
    ] {
        // The history doubles about log2(ticks × agents) times; everything else
        // (decode, step, reward, encode) reuses its buffers.
        let doublings = (ticks as f64 * 4.0).log2().ceil() as usize + 1;
        eprintln!("{ticks} ticks: {allocs} allocations");
        assert!(
            allocs <= doublings,
            "{allocs} allocations over {ticks} ticks"
        );
    }
}
