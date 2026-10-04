//! M2 T1: `encode_obs` writes only into the caller's slice and allocates nothing
//! (the `Flat` contract in `engine::generic::flat`: "neither allocates").
//!
//! In its own test binary because it installs a counting global allocator.

use engine::generic::Flat;
use engine::tank_flat::OBS_LEN;
use engine::{Action, Match, MatchConfig, Policy, TankRules};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use tank::{Chaser, MatchSpec, Wanderer};

struct Counting;

thread_local! {
    static ALLOCS: Cell<usize> = const { Cell::new(0) };
}

fn bump() {
    let _ = ALLOCS.try_with(|c| c.set(c.get() + 1));
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        bump();
        System.alloc(l)
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        bump();
        System.alloc_zeroed(l)
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        bump();
        System.realloc(p, l, n)
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        System.dealloc(p, l)
    }
}

#[global_allocator]
static A: Counting = Counting;

fn allocs_during(f: impl FnOnce()) -> usize {
    let before = ALLOCS.with(Cell::get);
    f();
    ALLOCS.with(Cell::get) - before
}

#[test]
fn counter_sees_allocations() {
    assert!(allocs_during(|| drop(std::hint::black_box(vec![1u8; 64]))) > 0);
}

#[test]
fn encode_obs_and_decode_action_do_not_allocate() {
    let mut buf = [0.0f32; OBS_LEN];
    // A duel mid-match, with shells in flight.
    let (mut m, mut a, mut b) = (
        Match::new(MatchConfig::duel(), 42),
        Chaser,
        Wanderer::new(42 ^ 0x5eed),
    );
    // A scripted duel with mixed loadouts.
    let spec = MatchSpec::from_query("seed=7&blue=kiter-5-3-1&orange=charger-4-1-4").unwrap();
    let (mut d, [mut p, mut q]) = spec.start();
    for tick in 0..300 {
        m.step_policies(&mut [&mut a as &mut dyn Policy, &mut b]);
        d.step_policies(&mut [p.as_mut(), q.as_mut()]);
        for agent in 0..2 {
            let n = allocs_during(|| {
                m.encode_obs(agent, &mut buf);
                d.encode_obs(agent, &mut buf);
                std::hint::black_box(&buf);
            });
            assert_eq!(n, 0, "encode_obs allocated at tick {tick}, agent {agent}");
        }
        let n = allocs_during(|| {
            std::hint::black_box(TankRules::decode_action(&[0.5, -0.5, 1.0, 1.0]));
        });
        assert_eq!(n, 0, "decode_action allocated");
        if m.is_over() || d.is_over() {
            break;
        }
    }
    let _ = Action::default();
}
