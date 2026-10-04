//! Native step throughput of the session core, for comparison with
//! `bench/bench.py` (the same loop through Python):
//! `cargo run --release -p engine-py --example bench [steps]`.
//!
//! Every learning row sends `reference_action` (17 rows cycling), episodes reset
//! with the next seed when they end, and the number is env steps per second.

use engine::TankRules;
use engine_py::{reference_action, Game, Session};
use racing::RacingRules;
use std::time::Instant;

fn bench<R: Game>(builds: &[&str], learning: &[usize], frame_skip: u32, steps: u32) -> (f64, f64)
where
    R::Config: Clone,
{
    let lineup = R::lineup(builds).unwrap();
    let mut s = Session::new(lineup, learning, frame_skip, 0).unwrap();
    let rows = learning.len();
    let pats: Vec<Vec<f32>> = (0..17)
        .map(|st| {
            (0..rows * R::ACTION_LEN)
                .map(|i| reference_action(st, i / R::ACTION_LEN, i % R::ACTION_LEN))
                .collect()
        })
        .collect();
    let mut rew = vec![0.0; rows];
    let mut obs = vec![0.0; rows * R::OBS_LEN];
    let (mut seed, mut ticks, mut sink) = (0u64, 0u64, 0.0f32);
    let t = Instant::now();
    for step in 0..steps {
        let before = s.game().tick();
        let over = s.step(&pats[(step % 17) as usize], &mut rew);
        s.encode_obs(&mut obs);
        ticks += (s.game().tick() - before) as u64;
        sink += obs[0] + rew.first().copied().unwrap_or(0.0);
        if over {
            seed += 1;
            s.reset(seed);
        }
    }
    let secs = t.elapsed().as_secs_f64();
    std::hint::black_box(sink);
    (steps as f64 / secs, ticks as f64 / secs)
}

fn main() {
    let steps: u32 = std::env::args()
        .nth(1)
        .map_or(200_000, |s| s.parse().unwrap());
    let t = tank::catalog::DEFAULT_BUILD_JSON;
    let r = racing::catalog::DEFAULT_BUILD_JSON;
    println!("case,frame_skip,steps_per_sec,ticks_per_sec");
    for fs in [1, 4] {
        let (a, b) = bench::<TankRules>(&[t, t], &[0, 1], fs, steps);
        println!("tank 2 learning,{fs},{a:.0},{b:.0}");
        let (a, b) = bench::<TankRules>(&[t, t], &[0], fs, steps);
        println!("tank 1 learning + charger,{fs},{a:.0},{b:.0}");
        let (a, b) = bench::<RacingRules>(&[r; 4], &[0, 1, 2, 3], fs, steps);
        println!("racing 4 learning,{fs},{a:.0},{b:.0}");
        let (a, b) = bench::<RacingRules>(&[r; 4], &[0], fs, steps);
        println!("racing 1 learning + 3 followers,{fs},{a:.0},{b:.0}");
    }
}
