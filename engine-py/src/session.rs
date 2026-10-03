//! [`Session`]: one learning-tool match over the [`Flat`] view, and the native
//! reference loop it is checked against.

use crate::{Game, Lineup};
#[cfg(doc)]
use engine::generic::{Flat, Rules};
use engine::generic::{Match, Policy};
use std::cell::Cell;

/// A match driven by a training tool. Some agents are *learning* (their actions come
/// from the caller, one row of [`Flat::ACTION_LEN`] floats each, in
/// [`learning`](Session::learning) order); the rest play their build's scripted
/// behavior in Rust.
///
/// One [`step`](Session::step) is `frame_skip` ticks with the same learning actions,
/// stopping early when the match ends. Each tick is exactly
/// [`Match::step_policies`]: inactive agents get `Action::default()` and their driver
/// is not asked, and every action is taken from the tick-start state. So a session's
/// replay is the replay the same actions give natively ([`reference_episode`]).
///
/// The step allocates nothing itself: actions are read from a slice into a reused
/// buffer, and observations and rewards are written into the caller's slices. (The
/// match history grows by doubling, and a scripted tank's rich `Observation` is built
/// per tick, as natively.)
pub struct Session<R: Game>
where
    R::Config: Clone,
{
    lineup: Lineup<R>,
    learning: Vec<usize>,
    /// Per agent: its row, if learning.
    row: Vec<Option<usize>>,
    frame_skip: u32,
    m: Match<R>,
    /// Per agent: its scripted driver (learning agents have none).
    scripts: Vec<Option<Box<dyn Policy<R>>>>,
    /// This step's decoded learning actions, by row.
    decoded: Vec<R::Action>,
    /// One tick's actions, by agent (reused).
    actions: Vec<R::Action>,
}

impl<R: Game> Session<R>
where
    R::Config: Clone,
{
    /// A session at `seed`. `learning` lists the learning agents (row order); every
    /// other agent needs a scripted build. `frame_skip` is at least 1.
    pub fn new(
        lineup: Lineup<R>,
        learning: &[usize],
        frame_skip: u32,
        seed: u64,
    ) -> Result<Self, String> {
        let n = lineup.behaviors.len();
        let mut row = vec![None; n];
        for (k, &a) in learning.iter().enumerate() {
            match row.get_mut(a) {
                None => return Err(format!("learning agent {a} out of range (0..{n})")),
                Some(Some(_)) => return Err(format!("learning agent {a} listed twice")),
                Some(r) => *r = Some(k),
            }
        }
        if let Some(a) = (0..n).find(|&a| row[a].is_none() && lineup.behaviors[a].is_none()) {
            return Err(format!(
                "agent {a} ({}) has a champion build, so it must be a learning agent \
                 (a champion needs its genome; the loader resolves it)",
                R::agent_name(a)
            ));
        }
        if frame_skip == 0 {
            return Err("frame_skip must be at least 1".into());
        }
        let m = Match::new(lineup.config.clone(), seed);
        let mut s = Self {
            learning: learning.to_vec(),
            row,
            frame_skip,
            m,
            scripts: Vec::with_capacity(n),
            decoded: vec![R::Action::default(); learning.len()],
            actions: Vec::with_capacity(n),
            lineup,
        };
        s.reset(seed);
        Ok(s)
    }

    /// Start a new match at `seed` (fresh scripted drivers, seeded as natively).
    pub fn reset(&mut self, seed: u64) {
        self.m = Match::new(self.lineup.config.clone(), seed);
        let config = self.m.config();
        self.scripts.clear();
        for (a, b) in self.lineup.behaviors.iter().enumerate() {
            self.scripts.push(match (self.row[a], b) {
                (None, Some(id)) => Some(R::scripted(config, a, id, seed)),
                _ => None,
            });
        }
    }

    /// One step: decode one action row per learning agent from `actions`, then run up
    /// to `frame_skip` ticks. `rewards[k]` gets row `k`'s reward summed over those
    /// ticks. Returns whether the match is over. A no-op (zero rewards) once over.
    ///
    /// # Panics
    /// If `actions` is not `rows × ACTION_LEN` long or `rewards` not `rows` long.
    pub fn step(&mut self, actions: &[f32], rewards: &mut [f32]) -> bool {
        let rows = self.learning.len();
        assert_eq!(actions.len(), rows * R::ACTION_LEN, "action buffer length");
        assert_eq!(rewards.len(), rows, "reward buffer length");
        rewards.fill(0.0);
        let len = R::ACTION_LEN;
        for (k, d) in self.decoded.iter_mut().enumerate() {
            *d = R::decode_action(&actions[k * len..(k + 1) * len]);
        }
        for _ in 0..self.frame_skip {
            if self.m.is_over() {
                break;
            }
            let state = self.m.state();
            self.actions.clear();
            for a in 0..self.scripts.len() {
                let act = if !R::is_active(state, a) {
                    R::Action::default()
                } else if let Some(k) = self.row[a] {
                    self.decoded[k]
                } else {
                    let p = self.scripts[a].as_mut().expect("scripted agent");
                    p.act(&self.m.observe(a))
                };
                self.actions.push(act);
            }
            self.m.step(&self.actions);
            for (r, &a) in rewards.iter_mut().zip(&self.learning) {
                *r += self.m.reward(a);
            }
        }
        self.m.is_over()
    }

    /// Write every learning agent's observation into `out` (`rows × OBS_LEN`, row
    /// order), from the current state.
    ///
    /// # Panics
    /// If `out` is not `rows × OBS_LEN` long.
    pub fn encode_obs(&self, out: &mut [f32]) {
        assert_eq!(
            out.len(),
            self.learning.len() * R::OBS_LEN,
            "observation buffer length"
        );
        let len = R::OBS_LEN;
        for (k, &a) in self.learning.iter().enumerate() {
            self.m.encode_obs(a, &mut out[k * len..(k + 1) * len]);
        }
    }

    /// Whether row `k`'s agent is still in play ([`Rules::is_active`]: a tank alive,
    /// a car not finished).
    pub fn active(&self, k: usize) -> bool {
        R::is_active(self.m.state(), self.learning[k])
    }

    /// The learning agents, by row.
    pub fn learning(&self) -> &[usize] {
        &self.learning
    }
    /// Agents in the match (learning and scripted).
    pub fn agents(&self) -> usize {
        self.lineup.behaviors.len()
    }
    /// Ticks per [`step`](Session::step).
    pub fn frame_skip(&self) -> u32 {
        self.frame_skip
    }
    /// The match (tick, outcome, hashes, replay).
    pub fn game(&self) -> &Match<R> {
        &self.m
    }
}

/// The action row `k` sends on step `step` in the determinism tests:
/// `((7·step + 5·k + 3·j) mod 17 − 8) / 8` for each component `j`. Every value is a
/// multiple of 1/8 in [−1, 1], so Python computes the identical `f32`.
pub fn reference_action(step: u32, k: usize, j: usize) -> f32 {
    ((step as usize * 7 + k * 5 + j * 3) % 17) as f32 / 8.0 - 1.0
}

/// Longest action row [`reference_episode`] supports.
const MAX_ACTION_LEN: usize = 16;

/// The native side of the determinism tests: the match a session plays when each
/// learning row sends [`reference_action`] on every step, for up to `max_steps` steps,
/// played with [`Match::step_policies`] and closures instead of [`Session`].
pub fn reference_episode<R: Game>(
    lineup: &Lineup<R>,
    learning: &[usize],
    frame_skip: u32,
    seed: u64,
    max_steps: u32,
) -> Match<R>
where
    R::Config: Clone,
{
    assert!(R::ACTION_LEN <= MAX_ACTION_LEN, "action row too long");
    let mut m = Match::<R>::new(lineup.config.clone(), seed);
    let tick = Cell::new(0u32);
    let mut drivers: Vec<Box<dyn Policy<R> + '_>> = (0..lineup.behaviors.len())
        .map(|a| -> Box<dyn Policy<R> + '_> {
            match learning.iter().position(|&l| l == a) {
                Some(k) => {
                    let tick = &tick;
                    Box::new(move |_: &R::Observation| {
                        let step = tick.get() / frame_skip;
                        let mut row = [0.0; MAX_ACTION_LEN];
                        for (j, v) in row[..R::ACTION_LEN].iter_mut().enumerate() {
                            *v = reference_action(step, k, j);
                        }
                        R::decode_action(&row[..R::ACTION_LEN])
                    })
                }
                None => R::scripted(
                    m.config(),
                    a,
                    lineup.behaviors[a].expect("scripted agent"),
                    seed,
                ),
            }
        })
        .collect();
    let mut refs: Vec<&mut dyn Policy<R>> = drivers
        .iter_mut()
        .map(|d| d.as_mut() as &mut dyn Policy<R>)
        .collect();
    while !m.is_over() && m.tick() < max_steps.saturating_mul(frame_skip) {
        tick.set(m.tick());
        m.step_policies(&mut refs);
    }
    drop(refs);
    drop(drivers);
    m
}
