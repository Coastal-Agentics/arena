//! The generic sim core (ADR-014, step B1): the fixed-tick loop, the seeded RNG, the
//! state hash, replays and the setup hash, written once for any game.
//!
//! A game plugs in by implementing [`Rules`]: its config, state, action, observation and
//! event types, and the functions the loop calls each tick. [`Match<R>`](Match) owns
//! everything that makes a match reproducible (seed, RNG, tick counter, action history,
//! outcome) and lends the RNG to [`Rules::init`] and [`Rules::step`] only.
//!
//! Tank Arena's rules are [`crate::TankRules`], still inside `engine` (ADR-014 defers
//! moving them to `games/tank`). The crate-root names are that instantiation:
//! [`crate::Match`] is `generic::Match<TankRules>`, [`crate::Replay`] and
//! [`crate::ReplayPlayer`] likewise, and [`Policy`] defaults its rules parameter to
//! `TankRules`, so `impl Policy for MyBot` and `&mut dyn Policy` mean the tank policy.

mod match_loop;
mod replay;
#[cfg(test)]
mod tests;

pub use match_loop::{EndReason, Match, Outcome};
pub use replay::{
    setup_hash, Replay, ReplayError, ReplayPlayer, OLDEST_READABLE_FORMAT, REPLAY_FORMAT,
};

use crate::sim::TankRules;
use rand_chacha::rand_core::{RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::de::DeserializeOwned;
use serde::Serialize;

/// A game's rules, plugged into the generic [`Match`]. All functions are associated
/// (no `self`): the rules are a type, and all match data lives in `Config` and `State`.
///
/// Contract the loop relies on (and that a rules crate must keep for determinism):
/// * [`init`](Rules::init) and [`step`](Rules::step) are the only places randomness
///   comes from, and only through the lent [`MatchRng`], in a documented order;
/// * no wall clock, no OS entropy, no iteration over unordered containers;
/// * [`hash_state`](Rules::hash_state) feeds every piece of dynamic state that affects
///   later ticks.
///
/// Per tick, [`Match::step`] takes one action per agent (missing entries are
/// `Action::default()`), passes each through [`sanitize`](Rules::sanitize), records the
/// result in the history, calls [`step`](Rules::step), increments the tick and asks
/// [`outcome`](Rules::outcome) whether the match is over. [`Match::step_policies`] calls
/// a policy only for agents that are [`is_active`](Rules::is_active).
pub trait Rules {
    /// Everything (with the seed) needed to reproduce a match. Stored in replays and
    /// hashed by [`setup_hash`] in its `serde_json` form.
    type Config: Clone + PartialEq + Serialize + DeserializeOwned;
    /// The dynamic state of a running match.
    type State: Clone;
    /// One agent's input for one tick, as recorded in the history and in replays.
    type Action: Copy + Default + PartialEq + Serialize + DeserializeOwned;
    /// What a policy sees for one agent on one tick.
    type Observation;
    /// Something that happened during the last step, for viewers and rule layers.
    type Event;

    /// The state at tick 0. May draw from `rng` (e.g. random spawns), in a documented
    /// order.
    fn init(config: &Self::Config, rng: &mut MatchRng) -> Self::State;
    /// Number of agents: the length of every tick's action list.
    fn agents(state: &Self::State) -> usize;
    /// Whether `agent` takes part this tick. For an inactive agent the policy isn't
    /// called and the default action is recorded.
    fn is_active(state: &Self::State, agent: usize) -> bool;
    /// The action as it is applied and recorded (e.g. clamped to its valid range).
    fn sanitize(action: Self::Action) -> Self::Action;
    /// Advance `state` by one fixed tick. `actions` holds exactly [`Rules::agents`]
    /// sanitized entries; `events` is empty on entry.
    fn step(
        config: &Self::Config,
        state: &mut Self::State,
        actions: &[Self::Action],
        rng: &mut MatchRng,
        events: &mut Vec<Self::Event>,
    );
    /// The observation for `agent` from the current state; `tick` is the number of
    /// ticks simulated so far.
    fn observe(
        config: &Self::Config,
        state: &Self::State,
        agent: usize,
        tick: u32,
    ) -> Self::Observation;
    /// `Some` once the match is over. Called after [`Match::new`] (tick 0) and after
    /// every step; once it returns `Some`, the match never steps again.
    fn outcome(config: &Self::Config, state: &Self::State, tick: u32) -> Option<Outcome>;
    /// Feed the dynamic state into the state hash. The engine has already fed the tick.
    fn hash_state(state: &Self::State, h: &mut StateHasher);
    /// Reject a config that uses something the replay's `format` doesn't have, naming
    /// the field (reported as [`ReplayError::FieldNotInFormat`]). Called by
    /// [`Replay::from_json`] after the format range and `setup_hash` checks.
    fn check_format(config: &Self::Config, format: u32) -> Result<(), &'static str>;
}

/// The match's seeded random number generator: `ChaCha8Rng::seed_from_u64(seed)`.
///
/// Owned by [`Match`] and lent to [`Rules::init`] and [`Rules::step`] only; it can't be
/// created outside the engine, so nothing else draws from it.
#[derive(Clone, Debug)]
pub struct MatchRng(ChaCha8Rng);

impl MatchRng {
    pub(crate) fn new(seed: u64) -> Self {
        Self(ChaCha8Rng::seed_from_u64(seed))
    }

    /// The next 32 random bits (`RngCore::next_u32` of the ChaCha8 stream).
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        self.0.next_u32()
    }

    /// The next 64 random bits (`RngCore::next_u64` of the ChaCha8 stream).
    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        self.0.next_u64()
    }
}

/// FNV-1a (64-bit) over little-endian `u64` words: the state hash builder passed to
/// [`Rules::hash_state`].
///
/// ```
/// use engine::generic::StateHasher;
/// let mut h = StateHasher::new();
/// h.write_u64(1);
/// assert_ne!(h.finish(), StateHasher::new().finish());
/// ```
#[derive(Clone, Debug)]
pub struct StateHasher(u64);

impl StateHasher {
    /// A hasher at the FNV-1a offset basis.
    pub fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    /// Feed `v` as its 8 little-endian bytes. Floats go in as `to_bits()`, signed
    /// integers as their bit pattern, so any drift shows.
    #[inline]
    pub fn write_u64(&mut self, v: u64) {
        for b in v.to_le_bytes() {
            self.0 ^= b as u64;
            self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
        }
    }

    /// The hash so far.
    pub fn finish(&self) -> u64 {
        self.0
    }
}

impl Default for StateHasher {
    fn default() -> Self {
        Self::new()
    }
}

/// A controller for one agent. Must be deterministic given its own state and the
/// observations (use a seeded RNG inside the policy if it needs randomness).
///
/// The rules parameter defaults to [`TankRules`], so `Policy` alone is the tank policy
/// (`impl Policy for MyBot`, `&mut dyn Policy`, `Box<dyn Policy>`).
///
/// Any `FnMut(&Observation) -> Action` closure is a policy:
///
/// ```
/// use engine::{Action, Match, MatchConfig, Observation};
///
/// let mut sit = |_: &Observation| Action::default();
/// let mut spin = |_: &Observation| Action { turn: 1.0, ..Default::default() };
/// let mut m = Match::new(MatchConfig::duel(), 1);
/// let o = m.run(&mut [&mut sit, &mut spin]);
/// assert_eq!(o.winner, None); // nobody shoots: draw at the tick limit
/// ```
pub trait Policy<R: Rules = TankRules> {
    /// Choose this tick's action from the observation.
    fn act(&mut self, obs: &R::Observation) -> R::Action;
}

impl<R: Rules, F: FnMut(&R::Observation) -> R::Action> Policy<R> for F {
    fn act(&mut self, obs: &R::Observation) -> R::Action {
        self(obs)
    }
}
