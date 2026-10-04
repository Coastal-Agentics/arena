//! The `saltmarsh_arena._core` module (`python` feature). Thin: the Python package
//! (`python/saltmarsh_arena`) builds the numpy views and the PettingZoo and
//! Gymnasium envs on top.
//!
//! **Buffers.** [`Arena`] owns four `bytearray`s, made once at construction:
//! observations (`rows × OBS_LEN` f32), actions (`rows × ACTION_LEN` f32), rewards
//! (`rows` f32) and active flags (`rows` u8). Python views them with `np.frombuffer`
//! (no copy), writes actions into the action view, calls `step()`, and reads the
//! others. `step()` takes no arguments and allocates nothing: it copies the action
//! bytes into a reused `f32` buffer (so alignment never matters), steps the
//! [`Session`], and copies the results back, all native-endian like numpy's
//! `float32`. A numpy view holds a buffer export on its `bytearray`, so Python can't
//! resize it under us; the length is still checked on every call.

use crate::{Game, Session, GAMES};
use engine::generic::{setup_hash, EndReason, Replay};
use engine::TankRules;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyByteArray;
use racing::RacingRules;

/// The parts of a [`Session`] the module needs, without its game type.
trait AnySession {
    fn step(&mut self, actions: &[f32], rewards: &mut [f32]) -> bool;
    fn encode_obs(&self, out: &mut [f32]);
    fn active(&self, row: usize) -> bool;
    fn reset(&mut self, seed: u64);
    fn tick(&self) -> u32;
    fn seed(&self) -> u64;
    fn outcome(&self) -> Option<engine::generic::Outcome>;
    fn state_hash(&self) -> u64;
    fn setup_hash(&self) -> u64;
    fn replay_json(&self) -> String;
}

impl<R: Game> AnySession for Session<R>
where
    R::Config: Clone,
{
    fn step(&mut self, actions: &[f32], rewards: &mut [f32]) -> bool {
        Session::step(self, actions, rewards)
    }
    fn encode_obs(&self, out: &mut [f32]) {
        Session::encode_obs(self, out)
    }
    fn active(&self, row: usize) -> bool {
        Session::active(self, row)
    }
    fn reset(&mut self, seed: u64) {
        Session::reset(self, seed)
    }
    fn tick(&self) -> u32 {
        self.game().tick()
    }
    fn seed(&self) -> u64 {
        self.game().seed()
    }
    fn outcome(&self) -> Option<engine::generic::Outcome> {
        self.game().outcome()
    }
    fn state_hash(&self) -> u64 {
        self.game().state_hash()
    }
    fn setup_hash(&self) -> u64 {
        setup_hash(self.game().seed(), self.game().config())
    }
    fn replay_json(&self) -> String {
        self.game().replay().to_json()
    }
}

/// The game, its agent names, and a boxed session for it.
struct Made {
    session: Box<dyn AnySession>,
    names: Vec<String>,
    obs_len: usize,
    action_len: usize,
}

fn make<R: Game>(
    builds: &[String],
    learning: &[usize],
    frame_skip: u32,
    seed: u64,
) -> PyResult<Made>
where
    R::Config: Clone,
{
    let refs: Vec<&str> = builds.iter().map(String::as_str).collect();
    let lineup = R::lineup(&refs).map_err(PyValueError::new_err)?;
    let names = (0..lineup.behaviors.len()).map(R::agent_name).collect();
    let session =
        Session::new(lineup, learning, frame_skip, seed).map_err(PyValueError::new_err)?;
    Ok(Made {
        session: Box::new(session),
        names,
        obs_len: R::OBS_LEN,
        action_len: R::ACTION_LEN,
    })
}

fn made(
    game: &str,
    builds: &[String],
    learning: &[usize],
    frame_skip: u32,
    seed: u64,
) -> PyResult<Made> {
    match game {
        "tank" => make::<TankRules>(builds, learning, frame_skip, seed),
        "racing" => make::<RacingRules>(builds, learning, frame_skip, seed),
        _ => Err(unknown(game)),
    }
}

fn unknown(game: &str) -> PyErr {
    let ids: Vec<&str> = GAMES.iter().map(|g| g.id).collect();
    PyValueError::new_err(format!("unknown game {game:?}; games: {ids:?}"))
}

fn reason(r: EndReason) -> &'static str {
    match r {
        EndReason::LastStanding => "last_standing",
        EndReason::AllDestroyed => "all_destroyed",
        EndReason::TickLimit => "tick_limit",
        EndReason::Finished => "finished",
    }
}

fn hex(h: u64) -> String {
    format!("{h:016x}")
}

/// Copy native-endian f32s out of `bytes`.
fn read_f32s(bytes: &[u8], out: &mut [f32]) {
    for (v, b) in out.iter_mut().zip(bytes.as_chunks::<4>().0) {
        *v = f32::from_ne_bytes(*b);
    }
}

/// Copy f32s into `bytes`, native-endian.
fn write_f32s(values: &[f32], bytes: &mut [u8]) {
    for (b, v) in bytes.as_chunks_mut::<4>().0.iter_mut().zip(values) {
        *b = v.to_ne_bytes();
    }
}

/// One match of a game for a training tool: see the module docs and
/// `saltmarsh_arena/_core.pyi`.
#[pyclass(module = "saltmarsh_arena._core", unsendable)]
struct Arena {
    game: &'static str,
    made: Made,
    learning: Vec<usize>,
    frame_skip: u32,
    obs: Py<PyByteArray>,
    actions: Py<PyByteArray>,
    rewards: Py<PyByteArray>,
    active: Py<PyByteArray>,
    obs_f32: Vec<f32>,
    act_f32: Vec<f32>,
    rew_f32: Vec<f32>,
}

impl Arena {
    /// Write the observations and active flags into the buffers.
    fn publish(&mut self, py: Python<'_>) -> PyResult<()> {
        self.made.session.encode_obs(&mut self.obs_f32);
        let obs = self.obs.bind(py);
        let active = self.active.bind(py);
        let rows = self.learning.len();
        if obs.len() != self.obs_f32.len() * 4 || active.len() != rows {
            return Err(PyRuntimeError::new_err("an Arena buffer was resized"));
        }
        // SAFETY: we hold the GIL and run no Python code while these slices live.
        unsafe {
            write_f32s(&self.obs_f32, obs.as_bytes_mut());
            for (k, f) in active.as_bytes_mut().iter_mut().enumerate() {
                *f = self.made.session.active(k) as u8;
            }
        }
        Ok(())
    }
}

#[pymethods]
impl Arena {
    #[new]
    #[pyo3(signature = (game, builds, learning, frame_skip = 4, seed = 0))]
    fn new(
        py: Python<'_>,
        game: &str,
        builds: Vec<String>,
        learning: Vec<usize>,
        frame_skip: u32,
        seed: u64,
    ) -> PyResult<Self> {
        let made = made(game, &builds, &learning, frame_skip, seed)?;
        let id = GAMES.iter().find(|g| g.id == game).expect("known game").id;
        let rows = learning.len();
        let zeros = |n: usize| PyByteArray::new(py, &vec![0u8; n]).unbind();
        let mut a = Self {
            game: id,
            obs: zeros(rows * made.obs_len * 4),
            actions: zeros(rows * made.action_len * 4),
            rewards: zeros(rows * 4),
            active: zeros(rows),
            obs_f32: vec![0.0; rows * made.obs_len],
            act_f32: vec![0.0; rows * made.action_len],
            rew_f32: vec![0.0; rows],
            made,
            learning,
            frame_skip,
        };
        a.publish(py)?;
        Ok(a)
    }

    /// Start a new match at `seed`; refreshes the observation and active buffers and
    /// zeroes the rewards.
    fn reset(&mut self, py: Python<'_>, seed: u64) -> PyResult<()> {
        self.made.session.reset(seed);
        self.rew_f32.fill(0.0);
        let rewards = self.rewards.bind(py);
        // SAFETY: as in `publish`.
        unsafe { rewards.as_bytes_mut().fill(0) };
        self.publish(py)
    }

    /// One step with the actions in `action_buffer`: up to `frame_skip` ticks.
    /// Refreshes the observation, reward and active buffers. Returns whether the match
    /// is over.
    fn step(&mut self, py: Python<'_>) -> PyResult<bool> {
        let actions = self.actions.bind(py);
        let rewards = self.rewards.bind(py);
        if actions.len() != self.act_f32.len() * 4 || rewards.len() != self.rew_f32.len() * 4 {
            return Err(PyRuntimeError::new_err("an Arena buffer was resized"));
        }
        // SAFETY: as in `publish`.
        unsafe { read_f32s(actions.as_bytes(), &mut self.act_f32) };
        let over = self.made.session.step(&self.act_f32, &mut self.rew_f32);
        unsafe { write_f32s(&self.rew_f32, rewards.as_bytes_mut()) };
        self.publish(py)?;
        Ok(over)
    }

    /// The game id.
    #[getter]
    fn game(&self) -> &'static str {
        self.game
    }
    /// Every agent's name, by agent index (learning and scripted).
    #[getter]
    fn agent_names(&self) -> Vec<String> {
        self.made.names.clone()
    }
    /// The learning agents' indices, in row order.
    #[getter]
    fn learning(&self) -> Vec<usize> {
        self.learning.clone()
    }
    /// Floats per observation row.
    #[getter]
    fn obs_len(&self) -> usize {
        self.made.obs_len
    }
    /// Floats per action row.
    #[getter]
    fn action_len(&self) -> usize {
        self.made.action_len
    }
    /// Ticks per step.
    #[getter]
    fn frame_skip(&self) -> u32 {
        self.frame_skip
    }
    /// `rows × obs_len` f32, written by `reset` and `step`.
    #[getter]
    fn obs_buffer(&self, py: Python<'_>) -> Py<PyByteArray> {
        self.obs.clone_ref(py)
    }
    /// `rows × action_len` f32, read by `step`.
    #[getter]
    fn action_buffer(&self, py: Python<'_>) -> Py<PyByteArray> {
        self.actions.clone_ref(py)
    }
    /// `rows` f32: each row's reward summed over the last step's ticks.
    #[getter]
    fn reward_buffer(&self, py: Python<'_>) -> Py<PyByteArray> {
        self.rewards.clone_ref(py)
    }
    /// `rows` u8: 1 while the row's agent is in play (tank alive, car not finished).
    #[getter]
    fn active_buffer(&self, py: Python<'_>) -> Py<PyByteArray> {
        self.active.clone_ref(py)
    }
    /// Ticks simulated so far.
    #[getter]
    fn tick(&self) -> u32 {
        self.made.session.tick()
    }
    /// The match seed.
    #[getter]
    fn seed(&self) -> u64 {
        self.made.session.seed()
    }
    /// Whether the match has ended.
    #[getter]
    fn is_over(&self) -> bool {
        self.made.session.outcome().is_some()
    }
    /// `None`, or `(winner_team, ticks, reason)` once the match has ended
    /// (`winner_team` is `None` for a draw).
    fn outcome(&self) -> Option<(Option<u8>, u32, &'static str)> {
        self.made
            .session
            .outcome()
            .map(|o| (o.winner, o.ticks, reason(o.reason)))
    }
    /// The state hash now, as the replay's 16-digit hex `final_hash`.
    fn state_hash(&self) -> String {
        hex(self.made.session.state_hash())
    }
    /// The replay's `setup_hash` (seed and config), 16-digit hex.
    fn setup_hash(&self) -> String {
        hex(self.made.session.setup_hash())
    }
    /// The match so far as replay JSON (`docs/engine/replay-format.md`): the same
    /// bytes a native run of the same actions writes.
    fn replay_json(&self) -> String {
        self.made.session.replay_json()
    }
}

/// `[(id, rules_version, obs_len, action_len, min_agents, max_agents), ...]`.
#[pyfunction]
fn games() -> Vec<(&'static str, u64, usize, usize, usize, usize)> {
    GAMES
        .iter()
        .map(|g| {
            (
                g.id,
                g.rules_version,
                g.obs_len,
                g.action_len,
                g.min_agents,
                g.max_agents,
            )
        })
        .collect()
}

fn info(game: &str) -> PyResult<&'static crate::games::GameInfo> {
    GAMES
        .iter()
        .find(|g| g.id == game)
        .ok_or_else(|| unknown(game))
}

/// The game's build catalog as JSON.
#[pyfunction]
fn catalog_json(game: &str) -> PyResult<&'static str> {
    Ok(info(game)?.catalog_json)
}

/// The game's default build as JSON.
#[pyfunction]
fn default_build_json(game: &str) -> PyResult<&'static str> {
    Ok(info(game)?.default_build_json)
}

/// `validateBuild` as JSON: `{"ok": true, ...}` or `{"ok": false, "errors": [...]}`.
#[pyfunction]
fn validate_build_json(game: &str, build: &str) -> String {
    match GAMES.iter().find(|g| g.id == game) {
        Some(g) => (g.validate_json)(build),
        None => game_catalog::unknown_game_json(),
    }
}

/// Re-simulate a replay natively (`Replay::verify`) and return its `final_hash`.
/// Raises `ValueError` if it does not load or verify.
#[pyfunction]
fn verify_replay(game: &str, json: &str) -> PyResult<String> {
    fn verify<R: engine::generic::Rules>(json: &str) -> PyResult<String>
    where
        R::Config: Clone,
    {
        let bad = |e: engine::generic::ReplayError| PyValueError::new_err(e.to_string());
        let m = Replay::<R>::from_json(json)
            .map_err(bad)?
            .verify()
            .map_err(bad)?;
        Ok(hex(m.state_hash()))
    }
    match game {
        "tank" => verify::<TankRules>(json),
        "racing" => verify::<RacingRules>(json),
        _ => Err(unknown(game)),
    }
}

/// The determinism tests' native episode (`engine_py::reference_episode`) as replay
/// JSON. `max_steps=None` plays to the end.
#[pyfunction]
#[pyo3(signature = (game, builds, learning, frame_skip, seed, max_steps = None))]
fn reference_replay(
    game: &str,
    builds: Vec<String>,
    learning: Vec<usize>,
    frame_skip: u32,
    seed: u64,
    max_steps: Option<u32>,
) -> PyResult<String> {
    fn run<R: Game>(b: &[String], l: &[usize], f: u32, s: u64, n: u32) -> PyResult<String>
    where
        R::Config: Clone,
    {
        let refs: Vec<&str> = b.iter().map(String::as_str).collect();
        let lineup = R::lineup(&refs).map_err(PyValueError::new_err)?;
        // Same checks as a session.
        Session::new(lineup.clone(), l, f, s).map_err(PyValueError::new_err)?;
        Ok(crate::reference_episode(&lineup, l, f, s, n)
            .replay()
            .to_json())
    }
    let n = max_steps.unwrap_or(u32::MAX);
    match game {
        "tank" => run::<TankRules>(&builds, &learning, frame_skip, seed, n),
        "racing" => run::<RacingRules>(&builds, &learning, frame_skip, seed, n),
        _ => Err(unknown(game)),
    }
}

/// The engine version replays record.
#[pyfunction]
fn engine_version() -> &'static str {
    engine::version()
}

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Arena>()?;
    m.add_function(wrap_pyfunction!(games, m)?)?;
    m.add_function(wrap_pyfunction!(catalog_json, m)?)?;
    m.add_function(wrap_pyfunction!(default_build_json, m)?)?;
    m.add_function(wrap_pyfunction!(validate_build_json, m)?)?;
    m.add_function(wrap_pyfunction!(verify_replay, m)?)?;
    m.add_function(wrap_pyfunction!(reference_replay, m)?)?;
    m.add_function(wrap_pyfunction!(engine_version, m)?)?;
    Ok(())
}
