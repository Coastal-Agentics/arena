//! Tolerance checks against `assets/tbd/reference/tbd_control.py` and
//! `tbd_control.js`. The fixture is regenerated with
//! `python3 games/tbd/tests/fixtures/gen_parity.py`.

use serde::Deserialize;
use tbd::{mix, quat_to_rpy, wrap, Battery, FlightController, Plant, Setpoint};

const TOL: f64 = 1e-9;

#[derive(Deserialize)]
struct Doc {
    controller: Vec<Case>,
    mix: Vec<MixCase>,
    wrap: Vec<WrapCase>,
    quat: Vec<QuatCase>,
    battery: BatDoc,
}

#[derive(Deserialize)]
struct Case {
    name: String,
    dt: f64,
    a_ff: [f64; 3],
    steps: Vec<StepIn>,
}

#[derive(Deserialize)]
struct StepIn {
    pos: [f64; 3],
    quat: [f64; 4],
    vel: [f64; 3],
    omega: [f64; 3],
    p_des: [f64; 3],
    v_des: [f64; 3],
    yaw_des: f64,
    python: Out,
    js: Option<Out>,
}

#[derive(Deserialize)]
struct Out {
    u: [f64; 4],
    f: [f64; 4],
    rpy: [f64; 3],
}

#[derive(Deserialize)]
struct MixCase {
    #[serde(rename = "F")]
    collective: f64,
    tx: f64,
    ty: f64,
    tz: f64,
    python: [f64; 4],
    js: [f64; 4],
}

#[derive(Deserialize)]
struct WrapCase {
    a: f64,
    python: f64,
    js: f64,
}

#[derive(Deserialize)]
struct QuatCase {
    q: [f64; 4],
    python: [f64; 3],
    js: [f64; 3],
}

#[derive(Deserialize)]
struct BatDoc {
    dt: f64,
    hover_minutes: f64,
    js_hover_minutes: f64,
    steps: Vec<BatStep>,
}

#[derive(Deserialize)]
struct BatStep {
    thrusts: [f64; 4],
    driving: f64,
    python_power: f64,
    python_pct: f64,
    js_power: f64,
    js_pct: f64,
}

fn near(a: f64, b: f64, what: &str) {
    let d = (a - b).abs();
    assert!(d <= TOL, "{what}: {a} vs {b} (delta {d})");
}

fn near4(a: [f64; 4], b: [f64; 4], what: &str) {
    for i in 0..4 {
        near(a[i], b[i], &format!("{what}[{i}]"));
    }
}

fn near3(a: [f64; 3], b: [f64; 3], what: &str) {
    for i in 0..3 {
        near(a[i], b[i], &format!("{what}[{i}]"));
    }
}

#[test]
fn controller_matches_python_and_js() {
    let doc: Doc = serde_json::from_str(include_str!("fixtures/controller_parity.json")).unwrap();
    for case in doc.controller {
        let mut fc = FlightController::new();
        for (i, step) in case.steps.iter().enumerate() {
            let out = fc.step(
                case.dt,
                &Plant {
                    pos: step.pos,
                    quat: step.quat,
                    vel: step.vel,
                    omega: step.omega,
                },
                &Setpoint {
                    pos: step.p_des,
                    vel: step.v_des,
                    yaw: step.yaw_des,
                    a_ff: case.a_ff,
                },
            );
            let tag = format!("{} step {i}", case.name);
            near4(out.u, step.python.u, &format!("{tag} python u"));
            near4(out.f, step.python.f, &format!("{tag} python f"));
            near3(out.rpy, step.python.rpy, &format!("{tag} python rpy"));
            if let Some(js) = &step.js {
                near4(out.u, js.u, &format!("{tag} js u"));
                near4(out.f, js.f, &format!("{tag} js f"));
                near3(out.rpy, js.rpy, &format!("{tag} js rpy"));
            } else {
                assert_ne!(
                    case.a_ff, [0.0; 3],
                    "{} should be the feedforward case",
                    case.name
                );
            }
        }
    }
    for m in doc.mix {
        let f = mix(m.collective, m.tx, m.ty, m.tz);
        near4(f, m.python, "mix python");
        near4(f, m.js, "mix js");
    }
    for w in doc.wrap {
        near(wrap(w.a), w.python, "wrap python");
        near(wrap(w.a), w.js, "wrap js");
    }
    for q in doc.quat {
        near3(quat_to_rpy(q.q), q.python, "quat python");
        near3(quat_to_rpy(q.q), q.js, "quat js");
    }
    let mut bat = Battery::new();
    near(
        bat.hover_minutes(),
        doc.battery.hover_minutes,
        "hover minutes python",
    );
    near(
        bat.hover_minutes(),
        doc.battery.js_hover_minutes,
        "hover minutes js",
    );
    assert!(
        (bat.hover_minutes() - 2.12).abs() < 0.02,
        "PARAMS.md ~2.1 min"
    );
    for (i, s) in doc.battery.steps.iter().enumerate() {
        near(
            bat.power(&s.thrusts, s.driving),
            s.python_power,
            &format!("power py {i}"),
        );
        near(
            bat.power(&s.thrusts, s.driving),
            s.js_power,
            &format!("power js {i}"),
        );
        let pct = bat.step(doc.battery.dt, &s.thrusts, s.driving);
        near(pct, s.python_pct, &format!("pct py {i}"));
        near(pct, s.js_pct, &format!("pct js {i}"));
    }
}

#[test]
fn hover_thrust_is_weight_over_four_ducts() {
    let mut fc = FlightController::new();
    let out = fc.step(
        0.002,
        &Plant {
            pos: [0.0, 0.0, 1.0],
            quat: [1.0, 0.0, 0.0, 0.0],
            vel: [0.0; 3],
            omega: [0.0; 3],
        },
        &Setpoint::hold([0.0, 0.0, 1.0]),
    );
    let u = 0.970 * 9.81 / 4.0 / 5.5;
    for i in 0..4 {
        near(out.u[i], u, "hover u");
        near(out.f[i], 0.970 * 9.81 / 4.0, "hover f");
    }
    assert_eq!(fc.integrator(), 0.0);
}
