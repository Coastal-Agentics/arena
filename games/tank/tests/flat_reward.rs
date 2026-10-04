//! M2 T1 + T2 acceptance: tank's [`Flat`] view and GATE-003 [`Rules::reward`], driven
//! through `Match<TankRules>` (`docs/design/tank-refit.md`, layout in
//! `docs/plans/GATE-003-learning-tanks.md` §6).
//!
//! The impls live in `engine/` (`engine::tank_flat` and `TankRules::reward`, #48). These
//! tests pin the contract as written there, plus the clarifications agreed on
//! 2026-10-03 (recorded in tank-refit.md):
//! - **Scaling:** positions, relative positions, walls and obstacles scale by
//!   `config.arena.size`, and the tick by `config.max_ticks`. Velocities, hp and
//!   cooldown use fixed constants (2.5, 940, 64 and 6), checked here against the level
//!   tables.
//! - **Ties:** tanks are ordered by distance, then lower id; shells by distance, then
//!   list order; obstacles by distance, then index.
//! - **Shaping:** each hit's damage is capped at the target's hp before that hit, so a
//!   duel's shaping is bounded by exactly ±0.5.
//! - **Terminal reward:** ±1 goes to every tank that was active at the start of the
//!   ending tick, including one destroyed on that tick. A draw (wipe-out or tick limit)
//!   gives 0.
//!
//! Confirmed by Shockwave (11:20 AM ET):
//! - **A destroyed agent's encoding is all zeros.**
//! - **`decode_action` clamps** each analog value to [−1, 1] and maps NaN to 0 itself.
//! - **A hit on a tank already at ≤ 0 hp** earlier in the same tick shapes 0.
//! - **Relative positions are in the world frame,** not the hull's.
//! - **Native-vs-wasm parity of `encode_obs`** is deferred to M4/M5, so it isn't covered here.
//!
//! Hash invariance (section 5) compares instrumented runs with plain runs and with the
//! recorded fixtures. The literal bot pins stay where they already live
//! (`src/bots.rs`, `tests/evolve_m1.rs`, `engine-wasm/tests/parity/`), so this file
//! doesn't duplicate them.

use engine::angle::{self, Heading};
use engine::generic::Flat;
use engine::tank_flat::{
    ACTION_LEN, ALLIES, COOLDOWN_SCALE, ENEMIES, MAX_HP_SCALE, OBSTACLES, OBSTACLE_SLOT,
    OBSTACLE_SLOTS, OBS_LEN, PROJECTILES, PROJECTILE_SLOT, PROJECTILE_SLOTS, PROJECTILE_VEL_SCALE,
    SELF, TANK_SLOT, TANK_SLOTS, TANK_VEL_SCALE, TICK, WALLS,
};
use engine::{
    Action, Arena, EndReason, Event, Match, MatchConfig, Observation, Policy, Rect, Replay, Rules,
    TankParams, TankRules, TankSpawn, Vec2, TICK_HZ,
};
use serde_json::Value;
use std::path::PathBuf;
use tank::evolve::{self, Genome};
use tank::loadout::{FIRE_COOLDOWN, MAX_HP, MAX_SPEED};
use tank::matchup::{BLUE_SALT, ORANGE_SALT};
use tank::{rules, Chaser, Loadout, MatchSpec, Wanderer};

// Layout and scales come from the engine (`engine::tank_flat`); the literal GATE-003 §6
// values are pinned once, in `lengths_and_offsets_are_the_gate_003_table` and
// `engine_scales_match_the_level_tables`.

const EPS: f32 = 1e-6;

fn obs(m: &Match, agent: usize) -> [f32; OBS_LEN] {
    let mut out = [f32::NAN; OBS_LEN];
    m.encode_obs(agent, &mut out);
    out
}

fn close(got: f32, want: f32, what: &str) {
    assert!((got - want).abs() <= EPS, "{what}: got {got}, want {want}");
}

fn close_slice(got: &[f32], want: &[f32], what: &str) {
    assert_eq!(got.len(), want.len(), "{what}: length");
    for (i, (&g, &w)) in got.iter().zip(want).enumerate() {
        close(g, w, &format!("{what}[{i}]"));
    }
}

fn clampf(v: f32) -> f32 {
    v.clamp(-1.0, 1.0)
}

/// The expected encoding, built from the rich `Observation` (which the engine already
/// sorts and truncates) with the §6 scaling, as agreed. Obstacles are sorted here, by
/// the squared distance from the tank's centre to the rectangle's nearest point, then
/// by index.
fn reference(o: &Observation, c: &MatchConfig) -> [f32; OBS_LEN] {
    let size = c.arena.size;
    let max_ticks = c.max_ticks as f32;
    let mut v = [0.0f32; OBS_LEN];
    let me = &o.me;
    let s = [
        2.0 * me.pos.x / size.x - 1.0,
        2.0 * me.pos.y / size.y - 1.0,
        me.vel.x / TANK_VEL_SCALE,
        me.vel.y / TANK_VEL_SCALE,
        angle::cos(me.heading),
        angle::sin(me.heading),
        angle::cos(me.turret),
        angle::sin(me.turret),
        me.hp as f32 / me.max_hp as f32,
        me.max_hp as f32 / MAX_HP_SCALE,
        me.cooldown as f32 / COOLDOWN_SCALE,
    ];
    for (k, x) in s.into_iter().enumerate() {
        v[SELF + k] = clampf(x);
    }
    for (base, list) in [(ENEMIES, &o.enemies), (ALLIES, &o.allies)] {
        for (k, t) in list.iter().take(TANK_SLOTS).enumerate() {
            let slot = [
                1.0,
                t.rel.x / size.x,
                t.rel.y / size.y,
                t.vel.x / TANK_VEL_SCALE,
                t.vel.y / TANK_VEL_SCALE,
                angle::cos(t.heading),
                angle::sin(t.heading),
                angle::cos(t.turret),
                angle::sin(t.turret),
                t.hp as f32 / t.max_hp as f32,
                t.max_hp as f32 / MAX_HP_SCALE,
                if t.los { 1.0 } else { 0.0 },
            ];
            for (j, x) in slot.into_iter().enumerate() {
                v[base + TANK_SLOT * k + j] = clampf(x);
            }
        }
    }
    for (k, p) in o.projectiles.iter().take(PROJECTILE_SLOTS).enumerate() {
        let slot = [
            1.0,
            p.rel.x / size.x,
            p.rel.y / size.y,
            p.vel.x / PROJECTILE_VEL_SCALE,
            p.vel.y / PROJECTILE_VEL_SCALE,
            if p.owner_team != me.team { 1.0 } else { 0.0 },
        ];
        for (j, x) in slot.into_iter().enumerate() {
            v[PROJECTILES + PROJECTILE_SLOT * k + j] = clampf(x);
        }
    }
    let w = [
        o.walls.left / size.x,
        o.walls.right / size.x,
        o.walls.bottom / size.y,
        o.walls.top / size.y,
    ];
    for (k, x) in w.into_iter().enumerate() {
        v[WALLS + k] = clampf(x);
    }
    v[TICK] = clampf(o.tick as f32 / max_ticks);
    for (k, r) in nearest_obstacles(me.pos, &o.obstacles)
        .into_iter()
        .enumerate()
    {
        let slot = [
            2.0 * r.min.x / size.x - 1.0,
            2.0 * r.min.y / size.y - 1.0,
            2.0 * r.max.x / size.x - 1.0,
            2.0 * r.max.y / size.y - 1.0,
        ];
        for (j, x) in slot.into_iter().enumerate() {
            v[OBSTACLES + OBSTACLE_SLOT * k + j] = clampf(x);
        }
    }
    v
}

fn nearest_obstacles(p: Vec2, obstacles: &[Rect]) -> Vec<Rect> {
    let mut idx: Vec<(f32, usize)> = obstacles
        .iter()
        .enumerate()
        .map(|(i, r)| ((p.clamp(r.min, r.max) - p).length_squared(), i))
        .collect();
    idx.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    idx.into_iter()
        .take(OBSTACLE_SLOTS)
        .map(|(_, i)| obstacles[i])
        .collect()
}

/// Index ranges inside the observation that are 0/1 flags.
fn flag_indices() -> Vec<usize> {
    let mut f = Vec::new();
    for base in [ENEMIES, ALLIES] {
        for k in 0..TANK_SLOTS {
            f.push(base + TANK_SLOT * k);
            f.push(base + TANK_SLOT * k + 11);
        }
    }
    for k in 0..PROJECTILE_SLOTS {
        f.push(PROJECTILES + PROJECTILE_SLOT * k);
        f.push(PROJECTILES + PROJECTILE_SLOT * k + 5);
    }
    f
}

// ---------------------------------------------------------------- match drivers

fn alive(m: &Match) -> Vec<bool> {
    m.tanks().iter().map(|t| t.alive).collect()
}

/// Calls `each(m, active_at_tick_start)` at tick 0 (everyone counts as active) and after
/// every step until the match is over.
fn drive(m: &mut Match, mut step: impl FnMut(&mut Match), mut each: impl FnMut(&Match, &[bool])) {
    let start = alive(m);
    each(m, &start);
    while !m.is_over() {
        let before = alive(m);
        step(m);
        each(m, &before);
    }
}

/// Chaser (team 0) vs Wanderer seeded `seed ^ 0x5eed` on `MatchConfig::duel`, the
/// engine-cli pairing pinned in `src/bots.rs`.
fn chaser_wanderer(seed: u64) -> (Match, Chaser, Wanderer) {
    (
        Match::new(MatchConfig::duel(), seed),
        Chaser,
        Wanderer::new(seed ^ 0x5eed),
    )
}

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../engine-wasm/tests/parity")
}

/// The pinned parity fixtures: (file name, manifest entry, file text).
fn parity_fixtures() -> Vec<(String, Value, String)> {
    let dir = fixture_dir();
    let manifest: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("manifest.json")).unwrap()).unwrap();
    manifest["fixtures"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            let name = f["file"].as_str().unwrap().to_string();
            let text = std::fs::read_to_string(dir.join(&name)).unwrap();
            (name, f.clone(), text)
        })
        .collect()
}

/// The match set for the range and reward sweeps: several Chaser-vs-Wanderer seeds,
/// scripted duels with mixed loadouts, and all 7 parity fixtures (2v2, wipe-out,
/// tick limit, per-tank params).
fn for_each_match(mut f: impl FnMut(&str, &mut Match, &mut dyn FnMut(&mut Match))) {
    for seed in [42u64, 7, 101, u64::MAX, 0, 1, 2, 3, 2916] {
        let (mut m, mut a, mut b) = chaser_wanderer(seed);
        f(&format!("chaser-wanderer seed {seed}"), &mut m, &mut |m| {
            m.step_policies(&mut [&mut a as &mut dyn Policy, &mut b]);
        });
    }
    for q in [
        "seed=42&blue=kiter-5-3-1&orange=charger-4-1-4",
        "seed=7&blue=charger-3-3-3&orange=charger-3-3-3",
        "seed=101&blue=sniper-1-3-5&orange=kiter-2-5-2",
        "seed=9&blue=charger-5-1-3&orange=sniper-3-1-5",
    ] {
        let spec = MatchSpec::from_query(q).unwrap();
        let (mut m, [mut a, mut b]) = spec.start();
        f(q, &mut m, &mut |m| {
            m.step_policies(&mut [a.as_mut(), b.as_mut()]);
        });
    }
    for (name, _, text) in parity_fixtures() {
        let r = Replay::from_json(&text).unwrap();
        let mut m = Match::new(r.config.clone(), r.seed);
        let mut t = 0usize;
        f(&name, &mut m, &mut |m| {
            m.step(&r.actions[t]);
            t += 1;
        });
    }
}

// ================================================================ 1. layout and table

#[test]
fn lengths_and_offsets_are_the_gate_003_table() {
    assert_eq!((TankRules::OBS_LEN, TankRules::ACTION_LEN), (176, 4));
    assert_eq!((OBS_LEN, ACTION_LEN), (176, 4));
    assert_eq!(
        (SELF, ENEMIES, ALLIES, PROJECTILES, WALLS, TICK, OBSTACLES),
        (0, 11, 59, 107, 155, 159, 160)
    );
    assert_eq!(
        (TANK_SLOTS, TANK_SLOT),
        (4, 12),
        "4 enemy and 4 ally slots × 12"
    );
    assert_eq!(
        (PROJECTILE_SLOTS, PROJECTILE_SLOT),
        (8, 6),
        "8 shell slots × 6"
    );
    assert_eq!(
        (OBSTACLE_SLOTS, OBSTACLE_SLOT),
        (4, 4),
        "4 obstacle slots × 4"
    );
    assert_eq!(OBSTACLES + OBSTACLE_SLOTS * OBSTACLE_SLOT, OBS_LEN);
}

#[test]
#[should_panic(expected = "observation buffer length")]
fn encode_obs_panics_on_a_wrong_length_buffer() {
    let m = Match::new(MatchConfig::duel(), 42);
    let mut short = [0.0f32; OBS_LEN - 1];
    m.encode_obs(0, &mut short);
}

#[test]
#[should_panic(expected = "action buffer length")]
fn decode_action_panics_on_a_wrong_length_input() {
    TankRules::decode_action(&[0.0; ACTION_LEN + 1]);
}

#[test]
fn engine_scales_match_the_level_tables() {
    // The GATE-003 §6 values.
    assert_eq!(
        (
            TANK_VEL_SCALE,
            MAX_HP_SCALE,
            COOLDOWN_SCALE,
            PROJECTILE_VEL_SCALE
        ),
        (2.5, 940.0, 64.0, 6.0)
    );
    let tick_hz = TICK_HZ as f64;
    let top_speed = MAX_SPEED.iter().copied().fold(f32::MIN, f32::max) as f64;
    assert_eq!(
        top_speed / tick_hz,
        TANK_VEL_SCALE as f64,
        "velocity ÷ 2.5 = MAX_SPEED[4] / 60"
    );
    assert_eq!(MAX_SPEED[4] as f64 / tick_hz, TANK_VEL_SCALE as f64);
    assert_eq!(
        *MAX_HP.iter().max().unwrap() as f32,
        MAX_HP_SCALE,
        "max_hp ÷ 940 = MAX_HP[4]"
    );
    assert_eq!(MAX_HP[4] as f32, MAX_HP_SCALE);
    assert_eq!(
        *FIRE_COOLDOWN.iter().max().unwrap() as f32,
        COOLDOWN_SCALE,
        "cooldown ÷ 64 = FIRE_COOLDOWN[0], the longest reload"
    );
    assert_eq!(FIRE_COOLDOWN[0] as f32, COOLDOWN_SCALE);
    // Every valid loadout (and the engine default) stays inside the scales, so the
    // clamp never bites in a normal match.
    let mut all: Vec<TankParams> = Loadout::ALL.iter().map(|l| l.params()).collect();
    all.push(TankParams::default());
    for p in all {
        assert!(
            p.max_speed as f64 / tick_hz <= TANK_VEL_SCALE as f64,
            "{p:?}"
        );
        assert!(p.max_hp as f32 <= MAX_HP_SCALE, "{p:?}");
        assert!(p.fire_cooldown as f32 <= COOLDOWN_SCALE, "{p:?}");
        assert_eq!(
            p.projectile_speed as f64 / tick_hz,
            PROJECTILE_VEL_SCALE as f64,
            "shell velocity ÷ 6 = 360 u/s ÷ 60"
        );
    }
}

fn spawn(team: u8, x: f32, y: f32, heading: Heading) -> TankSpawn {
    TankSpawn {
        team,
        pos: Some(Vec2::new(x, y)),
        heading: Some(heading),
        params: None,
    }
}

fn exact_params() -> TankParams {
    TankParams {
        max_hp: 650,
        projectile_spread: 0,
        ..TankParams::default()
    }
}

#[test]
fn self_block_walls_and_tick_on_a_constructed_state() {
    let h = angle::from_degrees(30);
    let config = MatchConfig {
        arena: Arena::new(800.0, 600.0),
        tanks: vec![spawn(0, 200.0, 150.0, h), spawn(1, 600.0, 450.0, 0)],
        params: exact_params(),
        max_ticks: 7200,
    };
    let mut m = Match::new(config, 1);
    let o = obs(&m, 0);
    let want_self = [
        -0.5,
        -0.5,
        0.0,
        0.0,
        angle::cos(h),
        angle::sin(h),
        angle::cos(h),
        angle::sin(h),
        1.0,
        650.0 / MAX_HP_SCALE,
        0.0,
    ];
    close_slice(&o[SELF..SELF + 11], &want_self, "self @0");
    close_slice(&o[WALLS..WALLS + 4], &[0.25, 0.75, 0.25, 0.75], "walls @0");
    close(o[TICK], 0.0, "tick @0");
    close_slice(
        &o[OBSTACLES..OBS_LEN],
        &[0.0; OBSTACLE_SLOTS * OBSTACLE_SLOT],
        "no obstacles: all slots empty",
    );
    close_slice(
        &o[ALLIES..PROJECTILES],
        &[0.0; TANK_SLOTS * TANK_SLOT],
        "no allies: empty slots are zeros",
    );
    close_slice(
        &o[PROJECTILES..WALLS],
        &[0.0; PROJECTILE_SLOTS * PROJECTILE_SLOT],
        "no shells yet",
    );

    // Drive forward at full throttle, turn the turret, fire.
    let go = Action {
        throttle: 1.0,
        turn: 0.0,
        turret_turn: 1.0,
        fire: true,
    };
    m.step(&[go, Action::default()]);
    let t = m.tanks()[0].clone();
    assert_eq!(t.turret, h.wrapping_add(546), "turret turned one full step");
    assert_eq!(t.cooldown, 45);
    let o = obs(&m, 0);
    let want_self = [
        2.0 * t.pos.x / 800.0 - 1.0,
        2.0 * t.pos.y / 600.0 - 1.0,
        t.vel.x / TANK_VEL_SCALE,
        t.vel.y / TANK_VEL_SCALE,
        angle::cos(h),
        angle::sin(h),
        angle::cos(t.turret),
        angle::sin(t.turret),
        1.0,
        650.0 / MAX_HP_SCALE,
        45.0 / COOLDOWN_SCALE,
    ];
    close_slice(&o[SELF..SELF + 11], &want_self, "self @1");
    // 120 u/s = 2 u/tick, so |vel| / 2.5 = 0.8.
    // (The table's cos/sin aren't exactly unit length, hence the looser tolerance.)
    let speed = Vec2::new(o[2], o[3]).length();
    assert!((speed - 0.8).abs() <= 1e-5, "|vel| scaled: {speed}");
    close_slice(
        &o[WALLS..WALLS + 4],
        &[
            t.pos.x / 800.0,
            (800.0 - t.pos.x) / 800.0,
            t.pos.y / 600.0,
            (600.0 - t.pos.y) / 600.0,
        ],
        "walls @1",
    );
    close(o[TICK], 1.0 / m.config().max_ticks as f32, "tick @1");
    // Its own shell: present, own team (is-enemy 0), velocity ÷ 6 is the unit direction.
    let p = m.projectiles()[0].clone();
    close_slice(
        &o[PROJECTILES..PROJECTILES + 6],
        &[
            1.0,
            (p.pos.x - t.pos.x) / 800.0,
            (p.pos.y - t.pos.y) / 600.0,
            angle::cos(t.turret),
            angle::sin(t.turret),
            0.0,
        ],
        "own shell",
    );
    // The enemy sees the same shell as an enemy shell.
    assert_eq!(
        obs(&m, 1)[PROJECTILES + 5],
        1.0,
        "is-enemy from the other side"
    );
}

#[test]
fn tank_slots_sort_ties_overflow_padding_and_los() {
    let shared = exact_params();
    let tough = TankParams {
        max_hp: 940,
        ..shared.clone()
    };
    let hd = |i: u16| i.wrapping_mul(5000);
    let mut tanks = vec![
        spawn(0, 400.0, 300.0, hd(0)), // 0: observer
        spawn(1, 500.0, 300.0, hd(1)), // 1: enemy, d 100
        spawn(1, 350.0, 300.0, hd(2)), // 2: enemy, d 50, max_hp 940
        spawn(1, 400.0, 150.0, hd(3)), // 3: enemy, d 150, behind obstacle 0 (no LOS)
        spawn(1, 600.0, 300.0, hd(4)), // 4: enemy, d 200
        spawn(1, 150.0, 300.0, hd(5)), // 5: enemy, d 250: the 5th, dropped
        spawn(0, 470.0, 370.0, hd(6)), // 6: ally, d² 9800
        spawn(0, 330.0, 370.0, hd(7)), // 7: ally, d² 9800 (tie, so id 6 first)
    ];
    tanks[2].params = Some(tough);
    let arena = Arena::new(800.0, 600.0)
        .with_obstacle(Rect::new(Vec2::new(390.0, 200.0), Vec2::new(410.0, 230.0))) // 0: d 70
        .with_obstacle(Rect::new(Vec2::new(700.0, 500.0), Vec2::new(750.0, 550.0))) // 1: d 360.6
        .with_obstacle(Rect::new(Vec2::new(100.0, 280.0), Vec2::new(120.0, 320.0))) // 2: d 280
        .with_obstacle(Rect::new(Vec2::new(680.0, 280.0), Vec2::new(700.0, 320.0))) // 3: d 280 (tie)
        .with_obstacle(Rect::new(Vec2::new(380.0, 500.0), Vec2::new(420.0, 520.0))) // 4: d 200
        .with_obstacle(Rect::new(Vec2::new(20.0, 20.0), Vec2::new(40.0, 40.0))); // 5: d 444
    let config = MatchConfig {
        arena,
        tanks,
        params: shared,
        max_ticks: 7200,
    };
    let m = Match::new(config, 3);
    // Sanity: the rich observation agrees with the intended setup.
    let rich = m.observe(0);
    let ids = |l: &[engine::TankObs]| l.iter().map(|t| t.id).collect::<Vec<_>>();
    assert_eq!(ids(&rich.enemies), [2, 1, 3, 4]);
    assert_eq!(ids(&rich.allies), [6, 7]);

    let o = obs(&m, 0);
    let slot = |rel: (f32, f32), h: Heading, max_hp: f32, los: f32| {
        [
            1.0,
            rel.0 / 800.0,
            rel.1 / 600.0,
            0.0,
            0.0,
            angle::cos(h),
            angle::sin(h),
            angle::cos(h),
            angle::sin(h),
            1.0,
            max_hp / MAX_HP_SCALE,
            los,
        ]
    };
    let e = |k: usize| &o[ENEMIES + TANK_SLOT * k..ENEMIES + TANK_SLOT * (k + 1)];
    let a = |k: usize| &o[ALLIES + TANK_SLOT * k..ALLIES + TANK_SLOT * (k + 1)];
    close_slice(
        e(0),
        &slot((-50.0, 0.0), hd(2), 940.0, 1.0),
        "enemy slot 0 = id 2",
    );
    close_slice(
        e(1),
        &slot((100.0, 0.0), hd(1), 650.0, 1.0),
        "enemy slot 1 = id 1",
    );
    close_slice(
        e(2),
        &slot((0.0, -150.0), hd(3), 650.0, 0.0),
        "enemy slot 2 = id 3, no LOS",
    );
    close_slice(
        e(3),
        &slot((200.0, 0.0), hd(4), 650.0, 1.0),
        "enemy slot 3 = id 4",
    );
    close_slice(
        a(0),
        &slot((70.0, 70.0), hd(6), 650.0, 1.0),
        "ally slot 0 = id 6 (tie, lower id)",
    );
    close_slice(
        a(1),
        &slot((-70.0, 70.0), hd(7), 650.0, 1.0),
        "ally slot 1 = id 7",
    );
    close_slice(a(2), &[0.0; TANK_SLOT], "ally slot 2 empty");
    close_slice(a(3), &[0.0; TANK_SLOT], "ally slot 3 empty");
    // The 5th enemy (id 5, rel x −250) is nowhere in the vector.
    for k in 0..TANK_SLOTS {
        assert!(
            (e(k)[1] - (-250.0 / 800.0)).abs() > 1e-3,
            "overflow enemy leaked into slot {k}"
        );
    }

    // Obstacles: nearest 4 by distance to the rectangle, ties by index (2 before 3).
    let ob = |min: (f32, f32), max: (f32, f32)| {
        [
            2.0 * min.0 / 800.0 - 1.0,
            2.0 * min.1 / 600.0 - 1.0,
            2.0 * max.0 / 800.0 - 1.0,
            2.0 * max.1 / 600.0 - 1.0,
        ]
    };
    let want: Vec<f32> = [
        ob((390.0, 200.0), (410.0, 230.0)),
        ob((380.0, 500.0), (420.0, 520.0)),
        ob((100.0, 280.0), (120.0, 320.0)),
        ob((680.0, 280.0), (700.0, 320.0)),
    ]
    .concat();
    close_slice(&o[OBSTACLES..OBS_LEN], &want, "obstacles: 0, 4, 2, 3");
}

#[test]
fn shell_slots_order_ties_overflow_and_owner_flag() {
    let rapid = TankParams {
        fire_cooldown: 1,
        ..exact_params()
    };
    let config = MatchConfig {
        arena: Arena::new(800.0, 600.0),
        tanks: vec![
            spawn(0, 400.0, 300.0, 0), // 0: observer, never fires
            spawn(1, 400.0, 400.0, 0), // 1: enemy above, fires +x
            spawn(1, 400.0, 200.0, 0), // 2: enemy below, fires +x (same distance as 1's shell)
            spawn(0, 200.0, 300.0, 0), // 3: ally, fires +x through the observer
        ],
        params: rapid,
        max_ticks: 7200,
    };
    let mut m = Match::new(config, 5);
    let fire = Action {
        fire: true,
        ..Action::default()
    };
    m.step(&[Action::default(), fire, fire, fire]);
    assert_eq!(m.projectiles().len(), 3);
    let o = obs(&m, 0);
    let me = m.tanks()[0].pos;
    let shell = |i: usize, enemy: f32| {
        let p = &m.projectiles()[i];
        [
            1.0,
            (p.pos.x - me.x) / 800.0,
            (p.pos.y - me.y) / 600.0,
            p.vel.x / PROJECTILE_VEL_SCALE,
            p.vel.y / PROJECTILE_VEL_SCALE,
            enemy,
        ]
    };
    let s =
        |k: usize| &o[PROJECTILES + PROJECTILE_SLOT * k..PROJECTILES + PROJECTILE_SLOT * (k + 1)];
    // Shells 0 and 1 are exactly equidistant: list order (tank 1's shell first) breaks the tie.
    let p = m.projectiles();
    assert_eq!(
        (p[0].pos - me).length_squared(),
        (p[1].pos - me).length_squared()
    );
    assert_eq!((p[0].owner, p[1].owner, p[2].owner), (1, 2, 3));
    close_slice(s(0), &shell(0, 1.0), "shell slot 0 = tank 1's");
    close_slice(
        s(1),
        &shell(1, 1.0),
        "shell slot 1 = tank 2's (tie, list order)",
    );
    close_slice(
        s(2),
        &shell(2, 0.0),
        "shell slot 2 = ally shell, is-enemy 0",
    );
    assert!(s(0)[2] > 0.0 && s(1)[2] < 0.0, "above, then below");
    close(s(0)[3], 1.0, "6 u/tick ÷ 6");
    for k in 3..PROJECTILE_SLOTS {
        close_slice(
            s(k),
            &[0.0; PROJECTILE_SLOT],
            &format!("shell slot {k} empty"),
        );
    }

    // Overflow: 3 more volleys make 12 shells; only the 8 nearest are encoded.
    for _ in 0..3 {
        m.step(&[Action::default(), fire, fire, fire]);
    }
    assert!(m.projectiles().len() > 8);
    let o = obs(&m, 0);
    let want = reference(&m.observe(0), m.config());
    close_slice(
        &o[PROJECTILES..WALLS],
        &want[PROJECTILES..WALLS],
        "8 nearest shells",
    );
    assert!(
        (0..PROJECTILE_SLOTS).all(|k| o[PROJECTILES + PROJECTILE_SLOT * k] == 1.0),
        "all 8 slots present"
    );
}

#[test]
fn scaling_follows_the_match_config_not_fixed_numbers() {
    // A 1000 × 500 arena and a 600-tick limit.
    let config = MatchConfig {
        arena: Arena::new(1000.0, 500.0)
            .with_obstacle(Rect::new(Vec2::new(100.0, 100.0), Vec2::new(200.0, 150.0))),
        tanks: vec![spawn(0, 250.0, 125.0, 0), spawn(1, 750.0, 375.0, 0)],
        params: exact_params(),
        max_ticks: 600,
    };
    let mut m = Match::new(config, 2);
    let o = obs(&m, 0);
    close_slice(&o[SELF..SELF + 2], &[-0.5, -0.5], "pos by arena size");
    close_slice(
        &o[ENEMIES + 1..ENEMIES + 3],
        &[0.5, 0.5],
        "rel by arena size",
    );
    close_slice(
        &o[WALLS..WALLS + 4],
        &[0.25, 0.75, 0.25, 0.75],
        "walls by arena size",
    );
    close_slice(
        &o[OBSTACLES..OBSTACLES + 4],
        &[-0.8, -0.6, -0.6, -0.4],
        "obstacle like pos",
    );
    for _ in 0..60 {
        m.step(&[Action::default(), Action::default()]);
    }
    close(obs(&m, 0)[TICK], 0.1, "tick ÷ max_ticks (60 / 600)");
}

#[test]
fn values_are_clamped_when_custom_params_exceed_the_scales() {
    let wild = TankParams {
        max_hp: 2000,
        max_speed: 300.0,
        fire_cooldown: 200,
        ..exact_params()
    };
    let config = MatchConfig {
        arena: Arena::new(800.0, 600.0),
        tanks: vec![spawn(0, 200.0, 300.0, 0), spawn(1, 600.0, 300.0, 0)],
        params: wild,
        max_ticks: 7200,
    };
    let mut m = Match::new(config, 4);
    let go = Action {
        throttle: 1.0,
        fire: true,
        ..Action::default()
    };
    m.step(&[go, Action::default()]);
    assert!(m.tanks()[0].vel.x > 2.5, "really faster than the scale");
    let o = obs(&m, 0);
    assert_eq!(o[SELF + 2], 1.0, "vel x 5 ÷ 2.5 clamps to 1");
    assert_eq!(o[SELF + 9], 1.0, "max_hp 2000 ÷ 940 clamps to 1");
    assert_eq!(o[SELF + 10], 1.0, "cooldown 200 ÷ 64 clamps to 1");
    assert_eq!(o[ENEMIES + 10], 1.0, "enemy max_hp clamps to 1");
}

#[test]
fn duel_pillars_leave_two_empty_obstacle_slots() {
    let m = Match::new(MatchConfig::duel(), 42);
    for agent in 0..2 {
        let o = obs(&m, agent);
        assert!(o[OBSTACLES..OBSTACLES + 2 * OBSTACLE_SLOT]
            .iter()
            .any(|&v| v != 0.0));
        close_slice(
            &o[OBSTACLES + 2 * OBSTACLE_SLOT..OBS_LEN],
            &[0.0; 2 * OBSTACLE_SLOT],
            "slots 2 and 3 empty",
        );
        close_slice(
            &o[ENEMIES + TANK_SLOT..ALLIES],
            &[0.0; 3 * TANK_SLOT],
            "enemy slots 1–3 empty",
        );
        close_slice(
            &o[ALLIES..PROJECTILES],
            &[0.0; TANK_SLOTS * TANK_SLOT],
            "no allies in a duel",
        );
        assert_eq!(o[ENEMIES], 1.0);
    }
}

#[test]
fn encode_obs_writes_every_index_and_ignores_old_buffer_contents() {
    let (mut m, mut a, mut b) = chaser_wanderer(42);
    for _ in 0..120 {
        m.step_policies(&mut [&mut a as &mut dyn Policy, &mut b]);
    }
    for agent in 0..2 {
        let mut x = [f32::NAN; OBS_LEN];
        let mut y = [7.0f32; OBS_LEN];
        let mut z = [0.0f32; OBS_LEN];
        m.encode_obs(agent, &mut x);
        m.encode_obs(agent, &mut y);
        m.encode_obs(agent, &mut z);
        assert!(x.iter().all(|v| v.is_finite()), "every index written");
        assert_eq!(x.map(f32::to_bits), y.map(f32::to_bits));
        assert_eq!(x.map(f32::to_bits), z.map(f32::to_bits));
        // Same as calling the trait function directly.
        let mut w = [0.0f32; OBS_LEN];
        TankRules::encode_obs(m.config(), m.state(), agent, m.tick(), &mut w);
        assert_eq!(x.map(f32::to_bits), w.map(f32::to_bits));
    }
}

// ================================================================ 2. ranges

#[test]
fn every_value_is_finite_in_range_and_matches_the_table_over_many_matches() {
    let flags = flag_indices();
    let mut checked = 0usize;
    for_each_match(|label, m, step| {
        drive(m, step, |m, _| {
            for agent in 0..m.tanks().len() {
                let o = obs(m, agent);
                let at = |i: usize| format!("{label}, tick {}, agent {agent}, index {i}", m.tick());
                for (i, &v) in o.iter().enumerate() {
                    assert!(v.is_finite() && (-1.0..=1.0).contains(&v), "{}: {v}", at(i));
                }
                for &i in &flags {
                    assert!(o[i] == 0.0 || o[i] == 1.0, "{}: flag {}", at(i), o[i]);
                }
                for (i, &v) in o.iter().enumerate().take(TICK + 1).skip(WALLS) {
                    assert!(v >= 0.0, "{}: walls and tick are ≥ 0", at(i));
                }
                // A destroyed agent's encoding is all zeros (confirmed).
                if !m.tanks()[agent].alive {
                    close_slice(
                        &o,
                        &[0.0; OBS_LEN],
                        &format!("{label}: destroyed agent {agent}"),
                    );
                } else {
                    assert!(o[SELF + 8] > 0.0 && o[SELF + 9] > 0.0, "{}", at(SELF + 8));
                    let want = reference(&m.observe(agent), m.config());
                    close_slice(
                        &o,
                        &want,
                        &format!("{label}, tick {}, agent {agent}", m.tick()),
                    );
                }
                checked += 1;
            }
        });
    });
    assert!(checked > 10_000, "{checked} observations checked");
}

// ================================================================ 3. decode

fn decode(v: [f32; ACTION_LEN]) -> Action {
    TankRules::decode_action(&v)
}

#[test]
fn decode_maps_in_range_values_to_controls() {
    for t in [-1.0f32, -0.5, 0.0, 0.25, 1.0] {
        for u in [-1.0f32, 0.0, 0.75] {
            for r in [-0.3f32, 0.0, 1.0] {
                let a = decode([t, u, r, -1.0]);
                assert_eq!(
                    (a.throttle, a.turn, a.turret_turn, a.fire),
                    (t, u, r, false)
                );
            }
        }
    }
    // fire = value > 0.
    assert!(!decode([0.0, 0.0, 0.0, -1.0]).fire);
    assert!(!decode([0.0, 0.0, 0.0, 0.0]).fire);
    assert!(!decode([0.0, 0.0, 0.0, -0.0]).fire);
    assert!(decode([0.0, 0.0, 0.0, 1e-6]).fire);
    assert!(decode([0.0, 0.0, 0.0, 1.0]).fire);
}

#[test]
fn decode_round_trips_actions() {
    for &(t, u, r, f) in &[
        (0.0, 0.0, 0.0, false),
        (1.0, -1.0, 0.5, true),
        (-0.25, 0.125, -1.0, false),
        (0.8, 1.0, 0.0, true),
    ] {
        let a = Action {
            throttle: t,
            turn: u,
            turret_turn: r,
            fire: f,
        };
        let v = [t, u, r, if f { 1.0 } else { -1.0 }];
        assert_eq!(decode(v), a);
    }
}

#[test]
fn out_of_range_input_is_clamped_before_it_is_applied_and_recorded() {
    // decode_action clamps itself and maps NaN to 0 (confirmed); the loop's sanitize is
    // then a no-op on its output.
    let cases = [
        ([2.0, -3.0, 1.5, 5.0], (1.0, -1.0, 1.0, true)),
        ([-9.0, 9.0, -1.01, -5.0], (-1.0, 1.0, -1.0, false)),
        (
            [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::NAN],
            (0.0, 1.0, -1.0, false),
        ),
    ];
    for (input, (t, u, r, f)) in cases {
        let want = Action {
            throttle: t,
            turn: u,
            turret_turn: r,
            fire: f,
        };
        assert_eq!(decode(input), want, "decode clamps {input:?}");
        assert_eq!(TankRules::sanitize(decode(input)), want, "{input:?}");
        let mut m = Match::new(MatchConfig::duel(), 42);
        m.step(&[decode(input), Action::default()]);
        assert_eq!(m.history().get(0).unwrap()[0], want, "recorded {input:?}");
    }
}

// ================================================================ 4. reward

/// The expected shaping for `agent` on the tick just stepped, from its events: for each
/// hit, the damage capped at the target's hp before that hit, ×0.5 ÷ the target's
/// max hp. Returns (dealt, taken), with taken ≤ 0.
fn shaping(m: &Match, agent: usize) -> (f32, f32) {
    let ev = m.events();
    let (mut dealt, mut taken) = (0.0f32, 0.0f32);
    for (k, e) in ev.iter().enumerate() {
        let Event::Hit {
            target,
            owner,
            damage,
        } = *e
        else {
            continue;
        };
        if owner != agent && target != agent {
            continue;
        }
        // hp before this hit = final hp + this and every later hit on the same target.
        let later: i32 = ev[k..]
            .iter()
            .map(|e| match *e {
                Event::Hit {
                    target: t,
                    damage: d,
                    ..
                } if t == target => d,
                _ => 0,
            })
            .sum();
        let before = m.tanks()[target].hp + later;
        // A later hit in the same tick on a tank already at ≤ 0 hp shapes 0 (confirmed).
        let eff = damage.min(before.max(0));
        let v = 0.5 * eff as f32 / m.tank_params(target).max_hp as f32;
        if owner == agent {
            dealt += v;
        }
        if target == agent {
            taken -= v;
        }
    }
    (dealt, taken)
}

/// The expected terminal reward for `agent` if the match ended on the tick just stepped.
fn terminal(m: &Match, agent: usize, active_at_start: bool) -> f32 {
    let Some(o) = m.outcome() else { return 0.0 };
    if o.ticks != m.tick() || !active_at_start {
        return 0.0;
    }
    match (o.reason, o.winner) {
        (EndReason::TickLimit, _) | (_, None) => 0.0,
        (_, Some(w)) if m.tanks()[agent].team == w => 1.0,
        _ => -1.0,
    }
}

#[test]
fn reward_is_shaping_plus_terminal_over_many_matches() {
    let mut overkills = 0usize;
    let mut kinds = std::collections::BTreeSet::new();
    for_each_match(|label, m, step| {
        let n = m.tanks().len();
        let mut cum_dealt = vec![0.0f32; n];
        let mut cum_taken = vec![0.0f32; n];
        let mut total = vec![0.0f32; n];
        drive(m, step, |m, before| {
            let mut tick_shaping = 0.0f32;
            for agent in 0..n {
                let r = m.reward(agent);
                let direct = <TankRules as Rules>::reward(m.config(), m.state(), m.events(), agent);
                assert_eq!(
                    r.to_bits(),
                    direct.to_bits(),
                    "Match::reward = Rules::reward"
                );
                let (d, t) = shaping(m, agent);
                let want = d + t + terminal(m, agent, before[agent]);
                assert!(
                    (r - want).abs() <= 1e-5,
                    "{label}, tick {}, agent {agent}: reward {r}, want {want}",
                    m.tick()
                );
                if m.tick() == 0 {
                    assert_eq!(r, 0.0, "nothing has happened at tick 0");
                }
                cum_dealt[agent] += d;
                cum_taken[agent] += t;
                total[agent] += r;
                tick_shaping += d + t;
            }
            assert!(
                tick_shaping.abs() <= 1e-5,
                "{label}: shaping is zero-sum per tick"
            );
            for e in m.events() {
                if let Event::Hit { target, damage, .. } = *e {
                    if m.tanks()[target].hp < 0 && damage > 0 {
                        overkills += 1;
                    }
                }
            }
        });
        let o = m.outcome().unwrap();
        kinds.insert(format!("{:?}", o.reason));
        let enemies = |a: usize| {
            m.tanks()
                .iter()
                .filter(|t| t.team != m.tanks()[a].team)
                .count() as f32
        };
        for a in 0..n {
            assert!(
                cum_taken[a] >= -0.5 - 1e-5,
                "{label}: taken {} < −0.5",
                cum_taken[a]
            );
            assert!(
                cum_dealt[a] <= 0.5 * enemies(a) + 1e-5,
                "{label}: dealt {} > 0.5 per enemy",
                cum_dealt[a]
            );
        }
        if n == 2 {
            // Duel: shaping in [−0.5, 0.5], the whole match is zero-sum, and the total
            // has the sign of the winner.
            assert!(
                (total[0] + total[1]).abs() <= 1e-5,
                "{label}: duel is zero-sum"
            );
            for a in 0..2 {
                let shaped = cum_dealt[a] + cum_taken[a];
                assert!(shaped.abs() <= 0.5 + 1e-5, "{label}: shaping {shaped}");
                match o.winner {
                    Some(w) if m.tanks()[a].team == w => assert!(total[a] >= 0.5 - 1e-5),
                    Some(_) => assert!(total[a] <= -0.5 + 1e-5),
                    None => assert!(total[a].abs() <= 0.5 + 1e-5),
                }
            }
        }
    });
    assert!(
        overkills > 0,
        "the sweep includes overkill hits, so the cap is exercised"
    );
    for k in ["LastStanding", "AllDestroyed", "TickLimit"] {
        assert!(
            kinds.contains(k),
            "the sweep covers a {k} ending: {kinds:?}"
        );
    }
}

/// Tank 1 sits between shooters 0 and 2 (both team 0); both fire on tick 1, spread 0.
/// max_hp 30, damage 20.
fn crossfire() -> Match {
    let fragile = TankParams {
        max_hp: 30,
        projectile_damage: 20,
        fire_cooldown: 1000,
        ..exact_params()
    };
    let config = MatchConfig {
        arena: Arena::new(800.0, 600.0),
        tanks: vec![
            spawn(0, 200.0, 300.0, 0),
            spawn(1, 400.0, 300.0, 0),
            spawn(0, 600.0, 300.0, angle::HALF_TURN),
        ],
        params: fragile,
        max_ticks: 7200,
    };
    Match::new(config, 6)
}

#[test]
fn overkill_is_capped_and_terminal_goes_to_tanks_active_at_the_ending_tick() {
    let mut m = crossfire();
    let fire = Action {
        fire: true,
        ..Action::default()
    };
    m.step(&[fire, Action::default(), fire]);
    let mut sums = [0.0f32; 3];
    let mut hit_ticks = Vec::new();
    while !m.is_over() {
        m.step(&[Action::default(); 3]);
        let hits = m
            .events()
            .iter()
            .filter(|e| matches!(e, Event::Hit { .. }))
            .count();
        if hits > 0 {
            hit_ticks.push((m.tick(), hits));
        }
        for (a, s) in sums.iter_mut().enumerate() {
            *s += m.reward(a);
        }
    }
    let o = m.outcome().unwrap();
    assert_eq!((o.winner, o.reason), (Some(0), EndReason::LastStanding));
    assert_eq!(m.tanks()[1].hp, -10, "20 + 20 damage on 30 hp");
    // Taken is capped at the 30 hp the target had: exactly −0.5, plus the −1 terminal.
    assert!((sums[1] - (-1.5)).abs() <= EPS, "target: {}", sums[1]);
    // Shooter 0's shell comes first in the list (lower id): 20 of 30 → 1/3; shooter 2's
    // second hit is capped at the 10 hp left → 1/6. Both are active winners: +1 each.
    assert_eq!(
        hit_ticks.len(),
        1,
        "both shells land on the same tick: {hit_ticks:?}"
    );
    assert_eq!(hit_ticks[0].1, 2);
    assert!(
        (sums[0] - (1.0 + 1.0 / 3.0)).abs() <= EPS,
        "shooter 0: {}",
        sums[0]
    );
    assert!(
        (sums[2] - (1.0 + 1.0 / 6.0)).abs() <= EPS,
        "shooter 2: {}",
        sums[2]
    );
    assert!(
        (sums[0] + sums[2] - (2.0 + 0.5)).abs() <= EPS,
        "dealt is capped too: {sums:?}"
    );
}

#[test]
fn a_tank_destroyed_before_the_ending_tick_gets_no_terminal_reward() {
    // 2v1 against tank 0: tank 0 (team 0) kills tank 1 (team 1) first, then tank 2 (team 1)
    // kills tank 0. Team 1 wins; tank 1 died earlier, so its final-tick reward is 0.
    let p = TankParams {
        max_hp: 20,
        projectile_damage: 20,
        fire_cooldown: 1000,
        ..exact_params()
    };
    let config = MatchConfig {
        arena: Arena::new(800.0, 600.0),
        tanks: vec![
            spawn(0, 300.0, 300.0, 0),                   // 0: fires +x at tank 1
            spawn(1, 400.0, 300.0, 0),                   // 1: d 100, dies first
            spawn(1, 300.0, 100.0, angle::QUARTER_TURN), // 2: fires +y at tank 0 from 200 away
        ],
        params: p,
        max_ticks: 7200,
    };
    let mut m = Match::new(config, 8);
    let fire = Action {
        fire: true,
        ..Action::default()
    };
    m.step(&[fire, Action::default(), fire]);
    let mut destroyed_at = [0u32; 3];
    let mut last = [0.0f32; 3];
    while !m.is_over() {
        m.step(&[Action::default(); 3]);
        for e in m.events() {
            if let Event::Destroyed { tank } = *e {
                destroyed_at[tank] = m.tick();
            }
        }
        last = [m.reward(0), m.reward(1), m.reward(2)];
    }
    let o = m.outcome().unwrap();
    assert_eq!((o.winner, o.reason), (Some(1), EndReason::LastStanding));
    assert!(
        destroyed_at[1] > 0 && destroyed_at[1] < destroyed_at[0],
        "{destroyed_at:?}"
    );
    assert_eq!(destroyed_at[0], o.ticks);
    // Final tick: tank 0 is destroyed on it (active at its start): −0.5 shaping, −1.
    assert!((last[0] - (-1.5)).abs() <= EPS, "{last:?}");
    // Tank 2 dealt the killing hit (+0.5) and won (+1).
    assert!((last[2] - 1.5).abs() <= EPS, "{last:?}");
    // Tank 1 was already out.
    assert_eq!(last[1], 0.0, "{last:?}");
}

#[test]
fn draws_give_no_terminal_reward() {
    for (name, entry, text) in parity_fixtures() {
        let reason = entry["outcome"]["reason"].as_str().unwrap().to_string();
        if reason == "last_standing" {
            continue;
        }
        let r = Replay::from_json(&text).unwrap();
        let mut m = Match::new(r.config.clone(), r.seed);
        for a in &r.actions {
            m.step(a);
        }
        assert!(m.outcome().unwrap().winner.is_none(), "{name}");
        for agent in 0..m.tanks().len() {
            let (d, t) = shaping(&m, agent);
            assert!(
                (m.reward(agent) - (d + t)).abs() <= 1e-5,
                "{name} ({reason}): agent {agent}"
            );
        }
    }
}

#[test]
fn after_the_end_step_is_a_no_op_and_reward_and_obs_repeat() {
    let (mut m, mut a, mut b) = chaser_wanderer(42);
    m.run(&mut [&mut a as &mut dyn Policy, &mut b]);
    let rewards = [m.reward(0), m.reward(1)];
    assert!(
        rewards[0].abs() >= 0.5,
        "the final tick carries the terminal reward"
    );
    let obs0 = [obs(&m, 0), obs(&m, 1)];
    let (tick, hash, outcome) = (m.tick(), m.state_hash(), m.outcome());
    let push = Action {
        throttle: 1.0,
        turn: 1.0,
        turret_turn: 1.0,
        fire: true,
    };
    for _ in 0..3 {
        assert_eq!(m.step(&[push, push]), outcome);
        assert_eq!((m.tick(), m.state_hash()), (tick, hash));
        // A repeat read returns the final tick's value: read it once, not every call.
        assert_eq!(
            [m.reward(0), m.reward(1)].map(f32::to_bits),
            rewards.map(f32::to_bits)
        );
        assert_eq!(obs(&m, 0).map(f32::to_bits), obs0[0].map(f32::to_bits));
        assert_eq!(obs(&m, 1).map(f32::to_bits), obs0[1].map(f32::to_bits));
    }
}

// ================================================================ 5. hashes unchanged

/// Every per-tick call a binding makes: encode_obs and reward for every agent.
fn poke(m: &Match, buf: &mut [f32; OBS_LEN]) -> f32 {
    let mut acc = 0.0f32;
    for agent in 0..m.tanks().len() {
        m.encode_obs(agent, buf);
        acc += buf.iter().sum::<f32>() + m.reward(agent);
        acc += <TankRules as Rules>::reward(m.config(), m.state(), m.events(), agent);
    }
    acc
}

fn chaser_wanderer_instrumented(seed: u64) -> Match {
    let (mut m, mut a, mut b) = chaser_wanderer(seed);
    let mut buf = [0.0; OBS_LEN];
    let mut sink = 0.0f32;
    drive(
        &mut m,
        |m| {
            m.step_policies(&mut [&mut a as &mut dyn Policy, &mut b]);
        },
        |m, _| sink += poke(m, &mut buf),
    );
    std::hint::black_box(sink);
    m
}

fn chaser_wanderer_plain(seed: u64) -> Match {
    let (mut m, mut a, mut b) = chaser_wanderer(seed);
    m.run(&mut [&mut a as &mut dyn Policy, &mut b]);
    m
}

#[test]
fn bot_pins_and_smoke_digest_are_unchanged_with_obs_and_reward_each_tick() {
    // The literal pins (42, 7, 101, u64::MAX and digest 28ae434ec1996a74) are asserted
    // on the plain runs in src/bots.rs; here the instrumented runs must equal them
    // byte for byte (replay JSON: format, config, actions, outcome, final_hash and
    // setup_hash).
    for seed in [42u64, 7, 101, u64::MAX] {
        let (x, y) = (
            chaser_wanderer_instrumented(seed),
            chaser_wanderer_plain(seed),
        );
        assert_eq!(x.replay().to_json(), y.replay().to_json(), "seed {seed}");
        assert_eq!(x.state_hash(), y.state_hash(), "seed {seed}");
    }
    // The smoke digest, computed exactly as src/bots.rs does, both ways.
    let digest = |play: fn(u64) -> Match| {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for seed in 0..200 {
            let m = play(seed);
            let o = m.outcome().expect("match ends");
            for b in [o.winner.unwrap_or(255)]
                .into_iter()
                .chain(o.ticks.to_le_bytes())
                .chain(m.state_hash().to_le_bytes())
            {
                h = (h ^ b as u64).wrapping_mul(0x0100_0000_01b3);
            }
        }
        h
    };
    assert_eq!(
        format!("{:016x}", digest(chaser_wanderer_instrumented)),
        format!("{:016x}", digest(chaser_wanderer_plain))
    );
}

#[test]
fn parity_fixtures_are_unchanged_with_obs_and_reward_each_tick() {
    let fixtures = parity_fixtures();
    assert_eq!(fixtures.len(), 7, "the 7 pinned parity fixtures");
    for (name, entry, text) in fixtures {
        let r = Replay::from_json(&text).unwrap();
        assert_eq!(r.format, 4, "{name}");
        let mut m = Match::new(r.config.clone(), r.seed);
        let mut buf = [0.0; OBS_LEN];
        let mut sink = 0.0f32;
        let mut t = 0usize;
        drive(
            &mut m,
            |m| {
                m.step(&r.actions[t]);
                t += 1;
            },
            |m, _| sink += poke(m, &mut buf),
        );
        std::hint::black_box(sink);
        let got = format!("{:016x}", m.state_hash());
        assert_eq!(got, r.final_hash, "{name}");
        assert_eq!(
            got,
            entry["final_hash"].as_str().unwrap(),
            "{name}: manifest"
        );
        assert_eq!(m.outcome(), r.outcome, "{name}");
        // Replay format 4, byte-identical to the pinned file.
        assert_eq!(
            m.replay().to_json(),
            text.trim_end(),
            "{name}: replay bytes"
        );
    }
}

fn champion() -> (Genome, Value) {
    let f: Value = serde_json::from_str(include_str!("fixtures/m1-champion.json")).unwrap();
    (Genome::from_json(&f["genome"]).unwrap(), f)
}

/// `evolve::duel`, with every per-tick binding call added.
fn duel_instrumented(blue: &Genome, orange: &Genome, seed: u64) -> evolve::Duel {
    let setup = rules::duel(blue.loadout, orange.loadout);
    let mut m = Match::new(setup.config, seed);
    let mut a = blue.build(seed ^ BLUE_SALT);
    let mut b = orange.build(seed ^ ORANGE_SALT);
    let mut buf = [0.0; OBS_LEN];
    let mut sink = 0.0f32;
    drive(
        &mut m,
        |m| {
            m.step_policies(&mut [a.as_mut(), b.as_mut()]);
        },
        |m, _| sink += poke(m, &mut buf),
    );
    std::hint::black_box(sink);
    let o = m.outcome().unwrap();
    let t = m.tanks();
    evolve::Duel {
        winner: o.winner,
        ticks: o.ticks,
        hash: m.state_hash(),
        hp: [t[0].hp, t[1].hp],
    }
}

#[test]
fn m1_champion_duels_are_unchanged_with_obs_and_reward_each_tick() {
    // The pinned held-out digest (d15709d4b3bd6953) is checked on the plain runs in
    // tests/evolve_m1.rs; here a sample of the same held-out duels must match exactly.
    let (g, _) = champion();
    let field = evolve::scripted_field();
    for opp in &field {
        for s in 0..4 {
            let seed = evolve::HELDOUT_SEED_BASE + s;
            assert_eq!(
                duel_instrumented(&g, opp, seed),
                evolve::duel(&g, opp, seed)
            );
            assert_eq!(
                duel_instrumented(opp, &g, seed),
                evolve::duel(opp, &g, seed)
            );
        }
    }
}

#[test]
#[ignore = "all 6,000 held-out duels instrumented; use --release"]
fn m1_champion_held_out_digest_is_unchanged_with_obs_and_reward_each_tick() {
    let (g, f) = champion();
    let field = evolve::scripted_field();
    let jobs: Vec<(usize, u64, usize)> = (0..3)
        .flat_map(|o| {
            (0..evolve::HELDOUT_SEEDS).flat_map(move |s| (0..2).map(move |side| (o, s, side)))
        })
        .collect();
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let duels = evolve::par_map(&jobs, threads, |&(o, s, side)| {
        let seed = evolve::HELDOUT_SEED_BASE + s;
        if side == 0 {
            duel_instrumented(&g, &field[o], seed)
        } else {
            duel_instrumented(&field[o], &g, seed)
        }
    });
    // The digest exactly as evolve::held_out computes it.
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    let mut fnv = |bytes: &[u8]| {
        for &b in bytes {
            h ^= b as u64;
            h = h.wrapping_mul(0x100_0000_01b3);
        }
    };
    for (&(o, s, side), d) in jobs.iter().zip(&duels) {
        fnv(&[o as u8, side as u8, d.winner.unwrap_or(2)]);
        fnv(&(evolve::HELDOUT_SEED_BASE + s).to_le_bytes());
        fnv(&d.ticks.to_le_bytes());
        fnv(&d.hash.to_le_bytes());
    }
    assert_eq!(
        Value::String(format!("{h:016x}")),
        f["held_out"]["digest"],
        "pinned M1 champion digest"
    );
}
