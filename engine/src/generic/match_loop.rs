//! The generic match: the fixed-tick loop, RNG ownership, action history and state hash.

use super::{MatchRng, Policy, Replay, Rules, StateHasher};
use serde::{Deserialize, Serialize};

/// Why a match ended. Team-based; generic across games (ADR-014).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndReason {
    /// Exactly one team has living tanks, and some tank of another team exists.
    LastStanding,
    /// No tank is alive (draw): the last tanks died on the same tick, or the match
    /// had no tanks at all.
    AllDestroyed,
    /// `max_ticks` reached (draw).
    TickLimit,
}

/// Final result of a match.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Outcome {
    /// Winning team, or `None` for a draw.
    pub winner: Option<u8>,
    /// Tick count when the match ended (equals [`Match::tick`] at the end).
    pub ticks: u32,
    /// Why it ended.
    pub reason: EndReason,
}

/// A running match under rules `R`. Create with [`Match::new`], drive with
/// [`Match::step`] or [`Match::run`], inspect with the accessors. Works identically
/// headless and in wasm.
///
/// For Tank Arena use the crate-root alias [`crate::Match`] (`Match<TankRules>`), which
/// also has the tank accessors (`tanks`, `projectiles`, `tank_params`).
#[derive(Clone, Debug)]
pub struct Match<R: Rules> {
    config: R::Config,
    seed: u64,
    rng: MatchRng,
    tick: u32,
    state: R::State,
    events: Vec<R::Event>,
    outcome: Option<Outcome>,
    history: Vec<Vec<R::Action>>,
}

impl<R: Rules> Match<R> {
    /// Start a match: the RNG is `ChaCha8Rng::seed_from_u64(seed)`, the state comes
    /// from [`Rules::init`] (the only draws before the first tick), and the outcome is
    /// checked once, so a match can be over at tick 0.
    pub fn new(config: R::Config, seed: u64) -> Self {
        let mut rng = MatchRng::new(seed);
        let state = R::init(&config, &mut rng);
        let outcome = R::outcome(&config, &state, 0);
        Self {
            config,
            seed,
            rng,
            tick: 0,
            state,
            events: Vec::new(),
            outcome,
            history: Vec::new(),
        }
    }

    /// The config this match was created with.
    pub fn config(&self) -> &R::Config {
        &self.config
    }
    /// The seed this match was created with.
    pub fn seed(&self) -> u64 {
        self.seed
    }
    /// Ticks simulated so far (0 before the first step).
    pub fn tick(&self) -> u32 {
        self.tick
    }
    /// The rules' dynamic state.
    pub fn state(&self) -> &R::State {
        &self.state
    }
    /// Events produced by the most recent step.
    pub fn events(&self) -> &[R::Event] {
        &self.events
    }
    /// The result, once the match has ended.
    pub fn outcome(&self) -> Option<Outcome> {
        self.outcome
    }
    /// True once the match has ended.
    pub fn is_over(&self) -> bool {
        self.outcome.is_some()
    }
    /// Actions applied so far (after [`Rules::sanitize`]), one `Vec` (indexed by agent)
    /// per tick.
    pub fn history(&self) -> &[Vec<R::Action>] {
        &self.history
    }

    /// The observation for `agent` from the current state ([`Rules::observe`]).
    pub fn observe(&self, agent: usize) -> R::Observation {
        R::observe(&self.config, &self.state, agent, self.tick)
    }

    /// Advance one fixed 1/60 s tick. `actions[i]` drives agent `i`; missing entries
    /// mean `Action::default()`, extra entries are ignored. Returns the outcome once the
    /// match has ended (further calls are no-ops).
    ///
    /// Order: clear the events; sanitize one action per agent ([`Rules::sanitize`]);
    /// [`Rules::step`]; record the sanitized actions; increment the tick; check
    /// [`Rules::outcome`]. For Tank Arena's order inside the step, see
    /// [`crate::TankRules`].
    pub fn step(&mut self, actions: &[R::Action]) -> Option<Outcome> {
        if self.outcome.is_some() {
            return self.outcome;
        }
        self.events.clear();
        let recorded: Vec<R::Action> = (0..R::agents(&self.state))
            .map(|i| R::sanitize(actions.get(i).copied().unwrap_or_default()))
            .collect();
        R::step(
            &self.config,
            &mut self.state,
            &recorded,
            &mut self.rng,
            &mut self.events,
        );
        self.history.push(recorded);
        self.tick += 1;
        self.outcome = R::outcome(&self.config, &self.state, self.tick);
        self.outcome
    }

    /// Query each active agent's policy (`policies[i]` drives agent `i`) and step once.
    ///
    /// Inactive agents ([`Rules::is_active`]), and agents without a policy, get
    /// `Action::default()` (the policy is not called). All observations are taken from
    /// the same tick-start state.
    pub fn step_policies(&mut self, policies: &mut [&mut dyn Policy<R>]) -> Option<Outcome> {
        let actions: Vec<R::Action> = (0..R::agents(&self.state))
            .map(|i| match policies.get_mut(i) {
                Some(p) if R::is_active(&self.state, i) => p.act(&self.observe(i)),
                _ => R::Action::default(),
            })
            .collect();
        self.step(&actions)
    }

    /// Run to completion with the given policies.
    ///
    /// Terminates as long as the rules end every match (Tank Arena: the tick limit).
    pub fn run(&mut self, policies: &mut [&mut dyn Policy<R>]) -> Outcome {
        loop {
            if let Some(o) = self.step_policies(policies) {
                return o;
            }
        }
    }

    /// FNV-1a hash of the full simulation state (bit patterns, so any drift shows): the
    /// tick, then whatever [`Rules::hash_state`] feeds. It does not cover the config,
    /// the seed, the RNG state or the action history.
    pub fn state_hash(&self) -> u64 {
        let mut h = StateHasher::new();
        h.write_u64(self.tick as u64);
        R::hash_state(&self.state, &mut h);
        h.finish()
    }

    /// Package this match (so far) as a replay.
    pub fn replay(&self) -> Replay<R> {
        Replay::from_match(self)
    }
}
