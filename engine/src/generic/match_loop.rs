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
    /// [`Rules::agents`], read once at [`Match::new`]: the stride of `history`.
    agents: usize,
    /// Every recorded action, tick after tick (`agents` per tick), in one buffer that
    /// grows by doubling instead of one `Vec` per tick.
    history: Vec<R::Action>,
    /// Reused by [`Match::step_policies`] for the tick's actions.
    scratch: Vec<R::Action>,
}

/// The recorded actions of a match, one slice of [`Rules::agents`] actions per tick:
/// a view of [`Match`]'s flat history buffer ([`Match::history`]).
#[derive(Debug, PartialEq)]
pub struct History<'a, A> {
    actions: &'a [A],
    agents: usize,
    ticks: usize,
}

impl<A> Clone for History<'_, A> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<A> Copy for History<'_, A> {}

impl<'a, A> History<'a, A> {
    /// Number of recorded ticks (equals [`Match::tick`]).
    pub fn len(&self) -> usize {
        self.ticks
    }
    /// True before the first step.
    pub fn is_empty(&self) -> bool {
        self.ticks == 0
    }
    /// Actions per tick ([`Rules::agents`]).
    pub fn agents(&self) -> usize {
        self.agents
    }
    /// Tick `t`'s actions, indexed by agent, or `None` past the end.
    pub fn get(&self, t: usize) -> Option<&'a [A]> {
        (t < self.ticks).then(|| &self.actions[t * self.agents..(t + 1) * self.agents])
    }
    /// Every tick's actions, in order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &'a [A]> + DoubleEndedIterator + 'a {
        let (actions, agents) = (self.actions, self.agents);
        (0..self.ticks).map(move |t| &actions[t * agents..(t + 1) * agents])
    }
    /// The whole buffer: tick 0's actions, then tick 1's, and so on.
    pub fn as_flat(&self) -> &'a [A] {
        self.actions
    }
}

impl<A> std::ops::Index<usize> for History<'_, A> {
    type Output = [A];
    fn index(&self, t: usize) -> &[A] {
        self.get(t).expect("tick out of range")
    }
}

impl<R: Rules> Match<R> {
    /// Start a match: the RNG is `ChaCha8Rng::seed_from_u64(seed)`, the state comes
    /// from [`Rules::init`] (the only draws before the first tick), and the outcome is
    /// checked once, so a match can be over at tick 0.
    pub fn new(config: R::Config, seed: u64) -> Self {
        let mut rng = MatchRng::new(seed);
        let state = R::init(&config, &mut rng);
        let outcome = R::outcome(&config, &state, 0);
        let agents = R::agents(&state);
        Self {
            config,
            seed,
            rng,
            tick: 0,
            state,
            events: Vec::new(),
            outcome,
            agents,
            history: Vec::new(),
            scratch: Vec::new(),
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
    /// Actions applied so far (after [`Rules::sanitize`]): one slice (indexed by agent)
    /// per tick.
    pub fn history(&self) -> History<'_, R::Action> {
        History {
            actions: &self.history,
            agents: self.agents,
            ticks: self.tick as usize,
        }
    }

    /// [`Rules::reward`] for `agent` on the tick just stepped (0.0 unless the game
    /// defines it). Never recorded.
    pub fn reward(&self, agent: usize) -> f32 {
        R::reward(&self.config, &self.state, &self.events, agent)
    }

    /// The observation for `agent` from the current state ([`Rules::observe`]).
    pub fn observe(&self, agent: usize) -> R::Observation {
        R::observe(&self.config, &self.state, agent, self.tick)
    }

    /// Advance one fixed 1/60 s tick. `actions[i]` drives agent `i`; missing entries
    /// mean `Action::default()`, extra entries are ignored. Returns the outcome once the
    /// match has ended (further calls are no-ops).
    ///
    /// Order: clear the events; sanitize one action per agent ([`Rules::sanitize`]) and
    /// append them to the history; [`Rules::step`] on those recorded actions; increment
    /// the tick; check [`Rules::outcome`]. For Tank Arena's order inside the step, see
    /// [`crate::TankRules`].
    pub fn step(&mut self, actions: &[R::Action]) -> Option<Outcome> {
        if self.outcome.is_some() {
            return self.outcome;
        }
        self.events.clear();
        let n = R::agents(&self.state);
        assert_eq!(n, self.agents, "Rules::agents changed during the match");
        // Record straight into the flat history; the rules step reads that slice.
        let start = self.history.len();
        self.history
            .extend((0..n).map(|i| R::sanitize(actions.get(i).copied().unwrap_or_default())));
        R::step(
            &self.config,
            &mut self.state,
            &self.history[start..],
            &mut self.rng,
            &mut self.events,
        );
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
        let mut actions = std::mem::take(&mut self.scratch);
        actions.clear();
        actions.extend(
            (0..R::agents(&self.state)).map(|i| match policies.get_mut(i) {
                Some(p) if R::is_active(&self.state, i) => p.act(&self.observe(i)),
                _ => R::Action::default(),
            }),
        );
        let outcome = self.step(&actions);
        self.scratch = actions;
        outcome
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
