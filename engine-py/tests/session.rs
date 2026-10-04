//! The session core against the native runners: same replays, byte for byte.

use engine::generic::{Match, Policy, Replay};
use engine::TankRules;
use engine_py::{reference_action, reference_episode, Game, Session};
use racing::RacingRules;

fn build(levels: [(&str, u8); 3], behavior: &str) -> String {
    let l: Vec<String> = levels.iter().map(|(k, v)| format!("\"{k}\":{v}")).collect();
    format!(
        r#"{{"rules_version":1,"levels":{{{}}},"behavior":{{"kind":"scripted","id":"{behavior}"}}}}"#,
        l.join(",")
    )
}

fn tank(a: u8, s: u8, d: u8, b: &str) -> String {
    build([("attack", a), ("speed", s), ("defense", d)], b)
}

fn car(p: u8, t: u8, g: u8, b: &str) -> String {
    build([("power", p), ("top_speed", t), ("grip", g)], b)
}

/// Drive a session with [`reference_action`] rows until it ends or `max_steps`.
fn run_session<R: Game>(s: &mut Session<R>, max_steps: u32) -> f64
where
    R::Config: Clone,
{
    let rows = s.learning().len();
    let mut act = vec![0.0; rows * R::ACTION_LEN];
    let mut rew = vec![0.0; rows];
    let mut obs = vec![0.0; rows * R::OBS_LEN];
    let mut total = 0.0f64;
    let mut step = 0;
    while step < max_steps {
        for (i, a) in act.iter_mut().enumerate() {
            *a = reference_action(step, i / R::ACTION_LEN, i % R::ACTION_LEN);
        }
        let over = s.step(&act, &mut rew);
        s.encode_obs(&mut obs);
        assert!(
            obs.iter().all(|v| (-1.0..=1.0).contains(v)),
            "obs in [-1, 1]"
        );
        total += rew.iter().map(|&r| r as f64).sum::<f64>();
        step += 1;
        if over {
            break;
        }
    }
    total
}

fn same<R: Game>(builds: &[String], learning: &[usize], frame_skip: u32, seed: u64, max_steps: u32)
where
    R::Config: Clone,
{
    let refs: Vec<&str> = builds.iter().map(String::as_str).collect();
    let lineup = R::lineup(&refs).unwrap();
    let mut s = Session::new(lineup.clone(), learning, frame_skip, seed).unwrap();
    run_session(&mut s, max_steps);
    let native = reference_episode(&lineup, learning, frame_skip, seed, max_steps);
    let (a, b) = (s.game().replay().to_json(), native.replay().to_json());
    assert_eq!(
        a,
        b,
        "{} learning {learning:?} skip {frame_skip} seed {seed}",
        R::GAME
    );
    let verified = Replay::<R>::from_json(&a).unwrap().verify().unwrap();
    assert_eq!(verified.state_hash(), s.game().state_hash());
    if max_steps == u32::MAX {
        assert!(s.game().is_over());
    }
}

#[test]
fn all_scripted_tank_session_is_the_matchspec_match() {
    for seed in [0, 1, 42, 7_777, u64::MAX - 5] {
        for (b, o) in [
            ("kiter-5-3-1", "charger-4-1-4"),
            ("sniper-3-3-3", "kiter-2-5-2"),
        ] {
            let spec =
                tank::MatchSpec::from_query(&format!("seed={seed}&blue={b}&orange={o}")).unwrap();
            let builds = [b, o].map(|t| {
                let p: Vec<&str> = t.split('-').collect();
                let n = |i: usize| p[i].parse().unwrap();
                tank(n(1), n(2), n(3), p[0])
            });
            let lineup = TankRules::lineup(&[&builds[0], &builds[1]]).unwrap();
            let mut s = Session::new(lineup, &[], 4, seed).unwrap();
            while !s.step(&[], &mut []) {}
            let (mut m, [mut x, mut y]) = spec.start();
            let o = m.run(&mut [x.as_mut(), y.as_mut()]);
            assert_eq!(s.game().outcome(), Some(o));
            assert_eq!(s.game().replay().to_json(), m.replay().to_json());
        }
    }
}

#[test]
fn all_scripted_racing_session_is_the_balance_race() {
    for seed in [0, 3, 99, u64::MAX] {
        let builds = [
            car(3, 3, 3, "follower"),
            car(5, 2, 2, "cutter"),
            car(2, 2, 5, "blocker"),
        ];
        let refs: Vec<&str> = builds.iter().map(String::as_str).collect();
        let lineup = RacingRules::lineup(&refs).unwrap();
        let config = lineup.config.clone();
        let mut s = Session::new(lineup, &[], 4, seed).unwrap();
        while !s.step(&[], &mut []) {}
        // `racing::balance::run`'s loop, with its replay.
        let drivers = ["follower", "cutter", "blocker"]
            .map(|k| racing::drivers::Behavior::from_key(k).unwrap());
        let mut m = Match::<RacingRules>::new(config.clone(), seed);
        let mut ps: Vec<Box<dyn Policy<RacingRules>>> = drivers
            .iter()
            .enumerate()
            .map(|(i, b)| b.driver(&config, i, seed))
            .collect();
        let mut refs: Vec<&mut dyn Policy<RacingRules>> = ps
            .iter_mut()
            .map(|p| p.as_mut() as &mut dyn Policy<RacingRules>)
            .collect();
        let o = m.run(&mut refs);
        assert_eq!(s.game().outcome(), Some(o));
        assert_eq!(s.game().replay().to_json(), m.replay().to_json());
        let r = racing::balance::run(&config, &drivers, seed);
        assert_eq!(r.ticks, o.ticks);
    }
}

#[test]
fn learning_tank_sessions_replay_natively() {
    let builds = [tank(5, 3, 1, "kiter"), tank(4, 1, 4, "charger")];
    for learning in [&[0, 1][..], &[1, 0], &[0], &[1]] {
        for frame_skip in [1, 4, 7] {
            for seed in [0, 42] {
                same::<TankRules>(&builds, learning, frame_skip, seed, u32::MAX);
            }
        }
    }
    same::<TankRules>(&builds, &[0, 1], 1, 5, 1_000);
}

#[test]
fn learning_racing_sessions_replay_natively() {
    let builds = [
        car(3, 3, 3, "follower"),
        car(5, 2, 2, "cutter"),
        car(2, 2, 5, "blocker"),
        car(1, 4, 4, "follower"),
    ];
    for learning in [&[0, 1, 2, 3][..], &[2], &[3, 0]] {
        for frame_skip in [1, 4] {
            for seed in [0, 9] {
                same::<RacingRules>(&builds, learning, frame_skip, seed, u32::MAX);
            }
        }
    }
    same::<RacingRules>(&builds[..1], &[0], 4, 1, u32::MAX);
    same::<RacingRules>(&builds, &[1], 1, 5, 1_000);
}

#[test]
fn frame_skip_stops_early_and_sums_rewards() {
    let builds = [car(3, 3, 3, "follower")];
    let lineup = RacingRules::lineup(&[&builds[0]]).unwrap();
    let mut one = Session::new(lineup.clone(), &[0], 1, 3).unwrap();
    let mut four = Session::new(lineup, &[0], 4, 3).unwrap();
    let (mut r1, mut r4) = ([0.0f32], [0.0f32]);
    let (mut sum1, mut sum4) = (0.0f32, 0.0f32);
    for _ in 0..30 {
        let mut step4 = 0.0f32;
        for _ in 0..4 {
            one.step(&[1.0, 0.0], &mut r1);
            step4 += r1[0];
        }
        sum1 += step4;
        four.step(&[1.0, 0.0], &mut r4);
        // The same ticks, and each step's reward is the sum of its ticks' rewards.
        assert_eq!(one.game().state_hash(), four.game().state_hash());
        assert_eq!(step4, r4[0]);
        sum4 += r4[0];
    }
    assert_eq!(four.game().tick(), 120);
    assert_eq!(sum1, sum4);
    assert!(sum4 > 0.0, "driving forward makes progress");
    // The last step stops at the end, and a finished match's steps are no-ops with
    // zero reward.
    let mut seven = Session::new(
        TankRules::lineup(&[&tank(5, 3, 1, "kiter"), &tank(4, 1, 4, "charger")]).unwrap(),
        &[],
        7,
        1,
    )
    .unwrap();
    while !seven.step(&[], &mut []) {}
    let end = seven.game().outcome().unwrap().ticks;
    assert_eq!(seven.game().tick(), end);
    assert!(seven.step(&[], &mut []));
    assert_eq!(seven.game().tick(), end);
    while !four.step(&[0.0, 0.0], &mut r4) {}
    let t = four.game().tick();
    assert!(four.step(&[1.0, 0.0], &mut r4));
    assert_eq!((four.game().tick(), r4[0]), (t, 0.0));
}

#[test]
fn bad_sessions_are_rejected() {
    let builds = [tank(3, 3, 3, "kiter"), tank(3, 3, 3, "kiter")];
    let lineup = TankRules::lineup(&[&builds[0], &builds[1]]).unwrap();
    let err = |l: &[usize], f| Session::new(lineup.clone(), l, f, 0).err().unwrap();
    assert_eq!(err(&[2], 4), "learning agent 2 out of range (0..2)");
    assert_eq!(err(&[1, 1], 4), "learning agent 1 listed twice");
    assert_eq!(err(&[], 0), "frame_skip must be at least 1");
    let champ = r#"{"rules_version":1,"levels":{"attack":3,"speed":3,"defense":3},"behavior":{"kind":"champion","ref":"gen-1"}}"#;
    let lineup = TankRules::lineup(&[&builds[0], champ]).unwrap();
    let e = Session::new(lineup.clone(), &[0], 4, 0).err().unwrap();
    assert!(
        e.starts_with("agent 1 (orange_1) has a champion build"),
        "{e}"
    );
    assert!(Session::new(lineup, &[1], 4, 0).is_ok());
}

#[test]
fn reset_restarts_the_same_match() {
    let builds = [tank(5, 3, 1, "kiter"), tank(4, 1, 4, "charger")];
    let lineup = TankRules::lineup(&[&builds[0], &builds[1]]).unwrap();
    let mut s = Session::new(lineup, &[0], 4, 11).unwrap();
    run_session(&mut s, u32::MAX);
    let first = s.game().replay().to_json();
    s.reset(12);
    run_session(&mut s, u32::MAX);
    assert_ne!(s.game().replay().to_json(), first);
    s.reset(11);
    run_session(&mut s, u32::MAX);
    assert_eq!(s.game().replay().to_json(), first);
}
