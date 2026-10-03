//! Test-only policies for the engine's own unit tests. Compiled only under
//! `#[cfg(test)]`, so none of this is public API.
//!
//! The placeholder bots `Chaser` and `Wanderer` live in `games/tank` (ADR-014 Phase A),
//! and the hashes that `docs/engine/` quotes for them are pinned there. These two
//! policies are deliberately different, so the engine keeps determinism pins of its own
//! that don't depend on any game crate. Across seeds 0–199 they end matches both ways
//! (`last_standing` and `all_destroyed`), win on both teams, drive in reverse, and are
//! sensitive to the obstacles and the shot spread about as often as the bots.

use crate::angle::turn_toward;
use crate::policy::{Action, Observation};
use crate::sim::{Match, MatchConfig};
use rand_chacha::rand_core::{RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// Closes to 250 units from the nearest enemy, backs off inside 150, turns the hull
/// toward it at 3/4 rate (tolerance 0.3), and fires when the turret is aligned
/// (tolerance 0.05). Stateless.
pub(crate) fn hunter() -> impl FnMut(&Observation) -> Action {
    |obs: &Observation| {
        let Some(e) = obs.enemies.first() else {
            return Action::default();
        };
        let aim = turn_toward(obs.me.turret, e.rel, 0.05);
        let steer = turn_toward(obs.me.heading, e.rel, 0.3);
        let throttle = if e.dist_sq > 250.0 * 250.0 {
            1.0
        } else if e.dist_sq < 150.0 * 150.0 {
            -0.5
        } else {
            0.0
        };
        Action {
            throttle,
            turn: steer as f32 * 0.75,
            turret_turn: aim as f32,
            fire: aim == 0,
        }
    }
}

/// Seeded random drive: every 15 to 44 ticks it picks a throttle (1, 0.6 or -0.4) and a
/// hull turn (-1, -0.5, 0, 0.5 or 1) from its own `ChaCha8Rng`. The turret tracks the
/// nearest enemy and fires when aligned (tolerance 0.1).
pub(crate) fn drifter(seed: u64) -> impl FnMut(&Observation) -> Action {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let (mut hold, mut throttle, mut turn) = (0u32, 0.0f32, 0.0f32);
    move |obs: &Observation| {
        if hold == 0 {
            throttle = [1.0, 0.6, -0.4][(rng.next_u32() % 3) as usize];
            turn = [-1.0, -0.5, 0.0, 0.5, 1.0][(rng.next_u32() % 5) as usize];
            hold = 15 + rng.next_u32() % 30;
        }
        hold -= 1;
        let (aim, fire) = match obs.enemies.first() {
            Some(e) => {
                let a = turn_toward(obs.me.turret, e.rel, 0.1);
                (a as f32, a == 0)
            }
            None => (0.0, false),
        };
        Action {
            throttle,
            turn,
            turret_turn: aim,
            fire,
        }
    }
}

/// Hunter (tank 0) vs a drifter seeded `seed ^ 0xd1f7` (tank 1) on `config`, to the end.
pub(crate) fn play_config(config: MatchConfig, seed: u64) -> Match {
    let mut m = Match::new(config, seed);
    let (mut a, mut b) = (hunter(), drifter(seed ^ 0xd1f7));
    m.run(&mut [&mut a, &mut b]);
    m
}

/// [`play_config`] on the default duel.
pub(crate) fn play(seed: u64) -> Match {
    play_config(MatchConfig::duel(), seed)
}

/// The test binary's allocator: the system one, counting allocations (and reallocations)
/// per thread, so tests running in parallel don't see each other's.
mod counting {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;

    thread_local! {
        static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
    }

    struct Counting;

    fn count() {
        // `try_with`: the thread-local may already be gone while a thread shuts down.
        let _ = ALLOCATIONS.try_with(|n| n.set(n.get() + 1));
    }

    // SAFETY: every call is forwarded unchanged to `System`.
    unsafe impl GlobalAlloc for Counting {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            count();
            System.alloc(layout)
        }
        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            count();
            System.alloc_zeroed(layout)
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            System.dealloc(ptr, layout)
        }
        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            count();
            System.realloc(ptr, layout, new_size)
        }
    }

    #[global_allocator]
    static COUNTING: Counting = Counting;

    pub(crate) fn allocations() -> u64 {
        ALLOCATIONS.with(Cell::get)
    }
}

/// Heap allocations (and reallocations) made on this thread while `f` runs.
pub(crate) fn allocations_in<T>(f: impl FnOnce() -> T) -> (T, u64) {
    let before = counting::allocations();
    let out = f();
    (out, counting::allocations() - before)
}
