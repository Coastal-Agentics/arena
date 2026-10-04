//! The opt-in, fixed-size `f32` view of a game, for bindings (Python, wasm).

use super::{Match, Rules};

/// A fixed-size `f32` encoding of one agent's observation and action, for training
/// tools and bindings. Opt-in: the match loop never calls it, and a game without it
/// still runs, replays and verifies as before.
///
/// Every agent of a game has the same lengths. [`encode_obs`](Flat::encode_obs) writes
/// into a caller-owned slice (so a binding can reuse one buffer for every call), and
/// [`decode_action`](Flat::decode_action) reads one; neither allocates. The encoding
/// describes one agent's view and controls only: what an agent is (its policy, params
/// and loadout) and what the map is live in the game's `Config`, not in this trait.
pub trait Flat: Rules {
    /// Length of one agent's encoded observation.
    const OBS_LEN: usize;
    /// Length of one agent's encoded action.
    const ACTION_LEN: usize;

    /// Write `agent`'s observation into `out` (exactly [`OBS_LEN`](Flat::OBS_LEN)
    /// values). `tick` is the number of ticks simulated so far. Must be deterministic:
    /// the same state gives the same values natively and in wasm.
    fn encode_obs(
        config: &Self::Config,
        state: &Self::State,
        agent: usize,
        tick: u32,
        out: &mut [f32],
    );

    /// Read one action from `input` (exactly [`ACTION_LEN`](Flat::ACTION_LEN) values).
    /// The loop still applies [`Rules::sanitize`] to the result before recording it.
    fn decode_action(input: &[f32]) -> Self::Action;
}

impl<R: Flat> Match<R> {
    /// [`Flat::encode_obs`] for `agent` from the current state.
    ///
    /// # Panics
    /// If `out.len()` is not [`Flat::OBS_LEN`].
    pub fn encode_obs(&self, agent: usize, out: &mut [f32]) {
        assert_eq!(out.len(), R::OBS_LEN, "observation buffer length");
        R::encode_obs(self.config(), self.state(), agent, self.tick(), out);
    }
}
