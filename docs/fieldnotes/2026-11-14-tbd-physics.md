# 2026-11-14 — TBD physics switches

*By Starscream, Engine Lead.*

**What:** Four estimated effects are now switches on the TBD game step. TBD is a physics sim with estimated parameters. TBD is the working name. Defaults are on. Each one can be turned off in the config JSON.

- **Voltage sag.** Open-circuit voltage runs from 4.2 V to 3.5 V per cell on 6S. Resistance defaults to 7.5 mΩ per cell (band 5–10 mΩ), marked ESTIMATE and unsourced. Thrust scales with `(V / 25.2 V)²`. Hover draw is about 19 A.
- **Ram drag.** Per duct, `ṁ V_air` opposite the in-plane airspeed. About 2–3 N total at 4 m/s, and a small pitch moment. `stateJson` adds `aero_force_n` and `aero_torque_nm` for `xfrc_applied` on body `tbd`, on top of `wind_force_n`.
- **Ground effect.** Cheeseman and Bennett, open-rotor formula used as an estimate for a duct. About +7% at a 37 mm duct height, about 1 at 1 m. Height comes from `range_down`.
- **Fold rate.** The command slews so 90° takes about 0.19 s (about 465°/s). ES08MD II no-load figure. The XML hold stiffness is unchanged.

Motor lag stays 40 ms. 5.5 N per duct stays an optimistic placeholder. 2.3 g/W stays a placeholder with range 1.8–2.5.

**Why:** So the page and the native runner can share one set of estimated corrections without baking them into the MuJoCo file where a position actuator has no rate limit.

**Verified:** Unit tests for sag, ground effect, ram drag and the fold time. Same-seed replay still matches. Native stair hop, seed 1, wind on, MuJoCo 3.17 via mujoco-rs: still completes and still lands. Score 530 (was 669), 15.36 s (was 10.99 s). No controller gains were changed.

**Next:** Retarget this branch's PR onto main after the TBD sim PR merges. The `web/tbd/` page should apply `aero_force_n` and `aero_torque_nm`.
