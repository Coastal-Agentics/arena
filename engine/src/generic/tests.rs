//! The generic core under a second, test-only rules impl ("race"), so `Match<R>`,
//! `Replay<R>`, `ReplayPlayer<R>` and `Policy<R>` are exercised without any tank code.

use super::*;
use serde::Deserialize;

/// Agents race along a line to `goal`; each step adds the (clamped) move plus a coin
/// flip from the match RNG. A finished agent is inactive. The match ends when every
/// agent has finished (`EndReason::Finished`, the first one wins) or at `max_ticks`
/// (draw). It writes replay format 5 (the default `WRITES_FORMAT`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Race;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct RaceConfig {
    agents: usize,
    goal: i32,
    max_ticks: u32,
    /// "Added in format 4": rejected by `check_format` in older replays (only reachable
    /// through [`Legacy`], which reads formats before 5).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    boost: Option<i32>,
}

#[derive(Clone, Debug, PartialEq)]
struct RaceState {
    pos: Vec<i32>,
    done: Vec<bool>,
    first: Option<usize>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
struct Move {
    step: i32,
}

#[derive(Debug, PartialEq)]
struct RaceObs {
    agent: usize,
    pos: i32,
    tick: u32,
}

#[derive(Clone, Debug, PartialEq)]
struct Finished(usize);

impl Rules for Race {
    type Config = RaceConfig;
    type State = RaceState;
    type Action = Move;
    type Observation = RaceObs;
    type Event = Finished;

    const GAME: &'static str = "race";
    const RULES_VERSION: u32 = 3;

    fn init(config: &RaceConfig, rng: &mut MatchRng) -> RaceState {
        RaceState {
            pos: (0..config.agents)
                .map(|_| (rng.next_u32() % 5) as i32)
                .collect(),
            done: vec![false; config.agents],
            first: None,
        }
    }
    fn agents(state: &RaceState) -> usize {
        state.pos.len()
    }
    fn is_active(state: &RaceState, agent: usize) -> bool {
        !state.done[agent]
    }
    fn sanitize(action: Move) -> Move {
        Move {
            step: action.step.clamp(-1, 3),
        }
    }
    fn step(
        config: &RaceConfig,
        state: &mut RaceState,
        actions: &[Move],
        rng: &mut MatchRng,
        events: &mut Vec<Finished>,
    ) {
        assert!(events.is_empty() && actions.len() == state.pos.len());
        for (i, a) in actions.iter().enumerate() {
            if state.done[i] {
                continue;
            }
            state.pos[i] += a.step + (rng.next_u32() % 2) as i32 + config.boost.unwrap_or(0);
            if state.pos[i] >= config.goal {
                state.done[i] = true;
                state.first.get_or_insert(i);
                events.push(Finished(i));
            }
        }
    }
    fn observe(_: &RaceConfig, state: &RaceState, agent: usize, tick: u32) -> RaceObs {
        RaceObs {
            agent,
            pos: state.pos[agent],
            tick,
        }
    }
    fn outcome(config: &RaceConfig, state: &RaceState, tick: u32) -> Option<Outcome> {
        if state.done.iter().all(|&d| d) {
            Some(Outcome {
                winner: state.first.map(|w| w as u8),
                ticks: tick,
                reason: EndReason::Finished,
            })
        } else if tick >= config.max_ticks {
            Some(Outcome {
                winner: None,
                ticks: tick,
                reason: EndReason::TickLimit,
            })
        } else {
            None
        }
    }
    fn hash_state(state: &RaceState, h: &mut StateHasher) {
        for (p, d) in state.pos.iter().zip(&state.done) {
            h.write_u64(*p as u32 as u64);
            h.write_u64(*d as u64);
        }
    }
    fn check_format(config: &RaceConfig, format: u32) -> Result<(), &'static str> {
        if format < 4 && config.boost.is_some() {
            return Err("boost");
        }
        Ok(())
    }
}

fn config() -> RaceConfig {
    RaceConfig {
        agents: 3,
        goal: 60,
        max_ticks: 500,
        boost: None,
    }
}

/// Agent `i` asks for step `i` (agent 2 asks for 7, which is clamped to 3).
fn run(config: RaceConfig, seed: u64) -> Match<Race> {
    let mut m = Match::<Race>::new(config, seed);
    let mut p0 = |_: &RaceObs| Move { step: 0 };
    let mut p1 = |_: &RaceObs| Move { step: 1 };
    let mut p2 = |_: &RaceObs| Move { step: 7 };
    m.run(&mut [&mut p0, &mut p1, &mut p2]);
    m
}

#[test]
fn same_seed_same_match_different_seed_differs() {
    let (a, b) = (run(config(), 5), run(config(), 5));
    assert_eq!(a.state_hash(), b.state_hash());
    assert_eq!(a.history(), b.history());
    assert_eq!(a.outcome(), b.outcome());
    assert_ne!(run(config(), 5).state(), run(config(), 6).state());
}

#[test]
fn loop_sanitizes_records_and_ends() {
    let m = run(config(), 1);
    let o = m.outcome().expect("over");
    assert_eq!((o.winner, o.reason), (Some(2), EndReason::Finished));
    assert_eq!(o.ticks, m.tick());
    assert_eq!(m.history().len(), m.tick() as usize);
    // Sanitized before recording: 7 is recorded as 3.
    assert_eq!(
        m.history()[0],
        [Move { step: 0 }, Move { step: 1 }, Move { step: 3 }]
    );
    assert_eq!(m.events(), &[Finished(0)], "agent 0 finishes last");
    // Further steps are no-ops.
    let mut m2 = m.clone();
    assert_eq!(m2.step(&[Move { step: 1 }]), Some(o));
    assert_eq!(m2.state_hash(), m.state_hash());
}

#[test]
fn missing_and_extra_actions() {
    let mut a = Match::<Race>::new(config(), 9);
    let mut b = Match::<Race>::new(config(), 9);
    a.step(&[Move { step: 2 }]);
    b.step(&[
        Move { step: 2 },
        Move::default(),
        Move::default(),
        Move { step: 3 },
    ]);
    assert_eq!(a.history(), b.history());
    assert_eq!(a.history()[0].len(), 3);
    assert_eq!(a.state_hash(), b.state_hash());
}

#[test]
fn inactive_agents_are_not_asked() {
    let mut m = Match::<Race>::new(config(), 3);
    let (mut asked0, mut asked1) = (0u32, 0u32);
    let mut p0 = |o: &RaceObs| {
        asked0 += 1;
        assert_eq!(o.agent, 0);
        Move { step: 3 }
    };
    let mut p1 = |o: &RaceObs| {
        asked1 += 1;
        assert_eq!((o.agent, o.tick), (1, asked1 - 1));
        Move { step: 1 }
    };
    // Agent 2 has no policy: it gets the default action and never finishes.
    let mut finished = [None, None];
    for _ in 0..100 {
        m.step_policies(&mut [&mut p0, &mut p1]);
        for (i, f) in finished.iter_mut().enumerate() {
            if f.is_none() && m.state().done[i] {
                *f = Some(m.tick());
            }
        }
    }
    assert!(m.outcome().is_none(), "agent 2 never finishes");
    assert!(m.history().iter().all(|t| t[2] == Move::default()));
    // Once finished, an agent isn't asked again and records the default action.
    for (i, asked) in [(0, asked0), (1, asked1)] {
        let f = finished[i].expect("finishes");
        assert!(f < 100);
        assert_eq!(asked, f, "agent {i}");
        assert!(m
            .history()
            .iter()
            .skip(f as usize)
            .all(|t| t[i] == Move::default()));
    }
}

#[test]
fn state_hash_is_tick_then_rules_state() {
    let m = run(config(), 2);
    let mut h = StateHasher::new();
    h.write_u64(m.tick() as u64);
    Race::hash_state(m.state(), &mut h);
    assert_eq!(m.state_hash(), h.finish());
}

#[test]
fn replays_roundtrip_verify_and_reject() {
    let m = run(config(), 11);
    let json = m.replay().to_json();
    assert!(
        json.starts_with(r#"{"format":5,"game":"race","rules_version":3,"engine_version":"#)
            && json.contains(r#""seed":"11""#),
        "{json}"
    );
    let r = Replay::<Race>::from_json(&json).expect("parses");
    assert_eq!(
        (r.game.as_deref(), r.rules_version),
        (Some("race"), Some(3))
    );
    assert_eq!(r.to_json(), json);
    let back = r.verify().expect("reproduces");
    assert_eq!(back.state(), m.state());
    // Edited config: setup hash mismatch.
    let edited = json.replace(r#""goal":60"#, r#""goal":61"#);
    assert!(matches!(
        Replay::<Race>::from_json(&edited).unwrap().verify(),
        Err(ReplayError::SetupMismatch { .. })
    ));
    // Tampered actions: outcome or hash mismatch.
    let mut t = r.clone();
    for a in t.actions.iter_mut() {
        a[2].step = -1;
    }
    assert!(t.verify().is_err());
}

#[test]
fn format_5_checks_game_and_rules_version() {
    let json = run(config(), 11).replay().to_json();
    let load = |from: &str, to: &str| {
        assert!(json.contains(from), "{from}");
        Replay::<Race>::from_json(&json.replace(from, to))
    };
    let other_game = load(r#""game":"race""#, r#""game":"racing""#).unwrap_err();
    assert_eq!(
        other_game,
        ReplayError::GameMismatch {
            expected: "race",
            got: Some("racing".into())
        }
    );
    assert_eq!(
        other_game.to_string(),
        r#"replay is for game "racing", these rules are "race""#
    );
    assert_eq!(
        load(r#""game":"race","#, ""),
        Err(ReplayError::GameMismatch {
            expected: "race",
            got: None
        })
    );
    let old_rules = load(r#""rules_version":3"#, r#""rules_version":2"#).unwrap_err();
    assert_eq!(
        old_rules,
        ReplayError::RulesVersionMismatch {
            game: "race",
            expected: 3,
            got: Some(2)
        }
    );
    assert_eq!(
        old_rules.to_string(),
        "replay has race rules_version 2, these rules are version 3"
    );
    assert_eq!(
        load(r#""rules_version":3,"#, ""),
        Err(ReplayError::RulesVersionMismatch {
            game: "race",
            expected: 3,
            got: None
        })
    );
    // A format 5 writer reads no older file: those predate `game` (Tank Arena files).
    let v4 = json.replace(
        r#""format":5,"game":"race","rules_version":3"#,
        r#""format":4"#,
    );
    assert_eq!(
        Replay::<Race>::from_json(&v4),
        Err(ReplayError::GameMismatch {
            expected: "race",
            got: None
        })
    );
    // An older format can't carry the format 5 fields.
    let e = load(r#""format":5"#, r#""format":4"#).unwrap_err();
    assert_eq!(
        e,
        ReplayError::FieldNotInFormat {
            format: 4,
            field: "game"
        }
    );
    assert_eq!(
        e.to_string(),
        "replay format 4 has no game (needs format 5)"
    );
    assert_eq!(
        load(r#""format":5"#, r#""format":6"#),
        Err(ReplayError::Format(6))
    );
}

#[test]
fn another_games_replay_is_named_not_misparsed() {
    // The envelope is read before the config, so the error names the game.
    let tank = crate::Match::new(crate::MatchConfig::duel(), 7)
        .replay()
        .to_json();
    assert_eq!(
        Replay::<Race>::from_json(&tank),
        Err(ReplayError::GameMismatch {
            expected: "race",
            got: None
        })
    );
    let race = run(config(), 7).replay().to_json();
    assert_eq!(
        crate::Replay::from_json(&race),
        Err(ReplayError::GameMismatch {
            expected: "tank",
            got: Some("race".into())
        })
    );
}

/// [`Race`] as a game whose replays predate format 5 (`WRITES_FORMAT = 4`), the path
/// Tank Arena takes, without tank code. Every rule delegates to [`Race`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Legacy;

impl Rules for Legacy {
    type Config = RaceConfig;
    type State = RaceState;
    type Action = Move;
    type Observation = RaceObs;
    type Event = Finished;

    const GAME: &'static str = "legacy";
    const RULES_VERSION: u32 = 1;
    const WRITES_FORMAT: u32 = 4;

    fn init(config: &RaceConfig, rng: &mut MatchRng) -> RaceState {
        Race::init(config, rng)
    }
    fn agents(state: &RaceState) -> usize {
        Race::agents(state)
    }
    fn is_active(state: &RaceState, agent: usize) -> bool {
        Race::is_active(state, agent)
    }
    fn sanitize(action: Move) -> Move {
        Race::sanitize(action)
    }
    fn step(
        config: &RaceConfig,
        state: &mut RaceState,
        actions: &[Move],
        rng: &mut MatchRng,
        events: &mut Vec<Finished>,
    ) {
        Race::step(config, state, actions, rng, events)
    }
    fn observe(config: &RaceConfig, state: &RaceState, agent: usize, tick: u32) -> RaceObs {
        Race::observe(config, state, agent, tick)
    }
    fn outcome(config: &RaceConfig, state: &RaceState, tick: u32) -> Option<Outcome> {
        Race::outcome(config, state, tick)
    }
    fn hash_state(state: &RaceState, h: &mut StateHasher) {
        Race::hash_state(state, h)
    }
    fn check_format(config: &RaceConfig, format: u32) -> Result<(), &'static str> {
        Race::check_format(config, format)
    }
}

#[test]
fn a_pre_format_5_game_keeps_its_format_and_reads_both() {
    let mut legacy = Match::<Legacy>::new(config(), 11);
    let mut p = |o: &RaceObs| Move {
        step: (o.agent % 3) as i32,
    };
    let mut q = p;
    let mut s = p;
    legacy.run(&mut [&mut p, &mut q, &mut s]);
    let json = legacy.replay().to_json();
    // Format 4, no game fields: the bytes a pre-format-5 writer always wrote.
    assert!(
        json.starts_with(r#"{"format":4,"engine_version":"#),
        "{json}"
    );
    assert!(!json.contains("game") && !json.contains("rules_version"));
    let r = Replay::<Legacy>::from_json(&json).expect("format 4 loads");
    assert_eq!((r.game.clone(), r.rules_version), (None, None));
    assert_eq!(r.verify().unwrap().state_hash(), legacy.state_hash());
    // The same game in format 5 also loads, when it names this game and version.
    let v5 = json.replace(
        r#""format":4"#,
        r#""format":5,"game":"legacy","rules_version":1"#,
    );
    let r5 = Replay::<Legacy>::from_json(&v5).expect("format 5 naming this game loads");
    assert_eq!(r5.verify().unwrap().state_hash(), legacy.state_hash());
    assert!(matches!(
        Replay::<Legacy>::from_json(&v5.replace("legacy", "race")),
        Err(ReplayError::GameMismatch { .. })
    ));
    // A config field the older format doesn't have, via Rules::check_format.
    let mut cfg = config();
    cfg.boost = Some(1);
    let boosted = Match::<Legacy>::new(cfg, 11)
        .replay()
        .to_json()
        .replace(r#""format":4"#, r#""format":3"#);
    let e = Replay::<Legacy>::from_json(&boosted).unwrap_err();
    assert_eq!(
        e,
        ReplayError::FieldNotInFormat {
            format: 3,
            field: "boost"
        }
    );
    assert_eq!(
        e.to_string(),
        "replay format 3 has no boost (needs format 4)"
    );
    // Format 2: no setup hash, still verifies.
    let mut v2 = r.clone();
    v2.format = 2;
    v2.setup_hash = None;
    let v2 = Replay::<Legacy>::from_json(&v2.to_json()).expect("format 2 loads");
    assert_eq!(v2.verify().unwrap().state_hash(), legacy.state_hash());
}

#[test]
fn replay_player_steps_to_end() {
    let m = run(config(), 4);
    let mut p = ReplayPlayer::new(m.replay());
    let mut n = 0;
    while p.step() {
        n += 1;
    }
    assert!(p.is_finished());
    assert_eq!(n, m.tick());
    assert_eq!(p.state().state_hash(), m.state_hash());
}

#[test]
fn policy_trait_objects_for_other_rules() {
    struct Sprinter;
    impl Policy<Race> for Sprinter {
        fn act(&mut self, _: &RaceObs) -> Move {
            Move { step: 3 }
        }
    }
    let mut boxed: Vec<Box<dyn Policy<Race>>> = vec![Box::new(Sprinter), Box::new(Sprinter)];
    let mut cfg = config();
    cfg.agents = 2;
    let mut m = Match::<Race>::new(cfg, 8);
    let (a, b) = boxed.split_at_mut(1);
    let o = m.run(&mut [&mut *a[0], &mut *b[0]]);
    assert_eq!(o.reason, EndReason::Finished);
}

/// The flat view for the test game: [pos, tick, done] per agent; one action value.
impl Flat for Race {
    const OBS_LEN: usize = 3;
    const ACTION_LEN: usize = 1;
    fn encode_obs(_: &RaceConfig, state: &RaceState, agent: usize, tick: u32, out: &mut [f32]) {
        out[0] = state.pos[agent] as f32;
        out[1] = tick as f32;
        out[2] = state.done[agent] as u8 as f32;
    }
    fn decode_action(input: &[f32]) -> Move {
        Move {
            step: input[0] as i32,
        }
    }
}

#[test]
fn flat_view_encodes_into_the_callers_buffer() {
    let mut m = Match::<Race>::new(config(), 2);
    let mut buf = [f32::NAN; Race::OBS_LEN];
    m.encode_obs(1, &mut buf);
    assert_eq!(buf, [m.state().pos[1] as f32, 0.0, 0.0]);
    // Decoded actions go through the same sanitize as any other action.
    let acts: Vec<Move> = [[7.0], [1.0], [-5.0]]
        .iter()
        .map(|a| Race::decode_action(a))
        .collect();
    m.step(&acts);
    assert_eq!(
        m.history()[0],
        [Move { step: 3 }, Move { step: 1 }, Move { step: -1 }]
    );
    m.encode_obs(1, &mut buf);
    assert_eq!(buf[1], 1.0, "tick after one step");
}

#[test]
#[should_panic(expected = "observation buffer length")]
fn flat_view_checks_the_buffer_length() {
    let m = Match::<Race>::new(config(), 2);
    m.encode_obs(0, &mut [0.0; 2]);
}

#[test]
fn reward_defaults_to_zero_and_is_not_recorded() {
    let m = run(config(), 1);
    for agent in 0..3 {
        assert_eq!(m.reward(agent), 0.0);
    }
    assert!(!m.replay().to_json().contains("reward"));
}

#[test]
fn history_is_one_flat_buffer() {
    let m = run(config(), 1);
    let h = m.history();
    assert_eq!((h.len(), h.agents()), (m.tick() as usize, 3));
    assert_eq!(h.as_flat().len(), h.len() * 3);
    assert_eq!(h.iter().len(), h.len());
    assert_eq!(h.get(h.len()), None);
    for (t, tick) in h.iter().enumerate() {
        assert_eq!(tick, &h.as_flat()[t * 3..t * 3 + 3]);
        assert_eq!(tick, &m.replay().actions[t][..]);
    }
}
