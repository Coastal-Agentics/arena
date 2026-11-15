"""TBD MuJoCo validation: drive, turn, fold, hover/take-off, scripted stair hop (+ MP4 and stills).
Run:  MUJOCO_GL=egl .venv/bin/python validate.py      (outputs in out/)"""
import json, math, sys, numpy as np, mujoco as mj
from pathlib import Path
from tbd_control import FlightController, Battery, P, quat_to_rpy
HERE = Path(__file__).parent; OUT = HERE / 'out'; OUT.mkdir(exist_ok=True)
m = mj.MjModel.from_xml_path(str(HERE / 'tbd.xml')); d = mj.MjData(m)
DT = m.opt.timestep
A = {m.actuator(i).name: i for i in range(m.nu)}
THR = [A[f'thrust_{c}'] for c in ('FL', 'FR', 'RL', 'RR')]
WL = [A['drive_FL'], A['drive_RL']]; WR = [A['drive_FR'], A['drive_RR']]
FOLD = A['fold']; FOLD_Q = m.jnt_qposadr[m.joint('fold_FL').id]
STOW, DEPLOY = 1.5708, 0.0
mass = float(m.body_subtreemass[m.body('tbd').id])
report = {'mass_kg': mass}
def st(): return d.qpos[0:3].copy(), d.qpos[3:7].copy(), d.qvel[0:3].copy(), d.qvel[3:6].copy()
def reset(key): mj.mj_resetDataKeyframe(m, d, m.key(key).id); mj.mj_forward(m, d)
def drive(l, r):
    for i in WL: d.ctrl[i] = l
    for i in WR: d.ctrl[i] = r

# ---------------- 1. drive + turn ----------------
reset('ground'); drive(-26, -26)          # reverse, away from the stairs
for _ in range(int(3/DT)): mj.mj_step(m, d)
report['drive_speed_mps'] = float(np.linalg.norm(d.qvel[0:2])); report['drive_dist_3s_m'] = float(abs(d.qpos[0]))
reset('ground'); drive(-26, 26)
for _ in range(int(3/DT)): mj.mj_step(m, d)
report['spin_in_place_yaw_rate_dps'] = float(math.degrees(d.qvel[5]))
reset('ground'); drive(10, 26)
for _ in range(int(4/DT)): mj.mj_step(m, d)
report['arc_turn_yaw_deg_4s'] = float(math.degrees(quat_to_rpy(d.qpos[3:7])[2]))
drive(0, 0)

# ---------------- 2. fold ----------------
reset('ground'); d.ctrl[FOLD] = DEPLOY; t_dep = None
for k in range(int(1.5/DT)):
    mj.mj_step(m, d)
    if t_dep is None and d.qpos[FOLD_Q] < 0.05: t_dep = (k+1)*DT
d.ctrl[FOLD] = STOW; t_stow = None
for k in range(int(1.5/DT)):
    mj.mj_step(m, d)
    if t_stow is None and d.qpos[FOLD_Q] > STOW - 0.05: t_stow = (k+1)*DT
report['deploy_time_s'] = t_dep; report['stow_time_s'] = t_stow
report['fold_arm_spread_rad'] = float(np.ptp(d.qpos[m.jnt_qposadr[[m.joint(f'fold_{c}').id for c in ('FL','FR','RL','RR')]]]))

# ---------------- 3. static thrust + hover ----------------
report['thrust_max_total_N'] = 4*P['t_max']
report['thrust_to_weight'] = 4*P['t_max']/(mass*9.81)
reset('deployed'); fc = FlightController(); bat = Battery(); thr_log = []
for k in range(int(8/DT)):
    pos, q, v, w = st()
    u, f = fc.step(DT, pos, q, v, w, p_des=(0, 0, min(1.0, 0.5*k*DT)))
    d.ctrl[THR] = u; mj.mj_step(m, d); bat.step(DT, f)
    if k*DT > 5: thr_log.append(u.mean())
report['hover_alt_m'] = float(d.qpos[2]); report['hover_throttle_frac'] = float(np.mean(thr_log))
report['hover_power_W_model'] = float(bat.power([mass*9.81/4]*4))
report['hover_minutes_est'] = float(bat.hover_minutes())
report['hover_minutes_range_2.3_to_3.0_g_per_W'] = [float(bat.hover_minutes()),
    float(P['pack_wh']*P['usable']/(mass*1000/3.0 + P['avionics_w'])*60)]
report['hover_pos_err_m'] = float(np.linalg.norm(d.qpos[0:3] - [0, 0, 1.0]))
# full-throttle climb check (honest lift-off test, no controller)
reset('deployed'); d.ctrl[THR] = 1.0
for _ in range(int(0.5/DT)): mj.mj_step(m, d)
report['full_throttle_climb_0.5s_m'] = float(d.qpos[2])

# ---------------- 4. scripted stair hop with video ----------------
TOP, LAND_X0 = 0.68, 1.92
reset('ground'); fc.reset(); bat = Battery()
W, H, FPS = 1280, 720, 30
r = mj.Renderer(m, H, W)
cam = mj.MjvCamera(); cam.type = mj.mjtCamera.mjCAMERA_FREE
cam.distance, cam.azimuth, cam.elevation = 1.5, -68, -18
look = np.array([0.0, 0, 0.2])
frames, stills, log = [], {}, []
phase, t, t_phase = 'drive', 0.0, 0.0
target = None; xland = 2.30; zc = TOP + 0.32
next_frame = 0.0; landed_z = None; max_tilt = 0.0; min_clear = 9
def set_phase(p):
    global phase, t_phase; phase, t_phase = p, t
while t < 16.0 and phase != 'done':
    pos, q, v, w = st(); tp = t - t_phase
    thrusts = np.zeros(4); driving = 0.0
    if phase == 'drive':
        drive(24, 24); driving = 1
        if pos[0] >= 0.42: drive(0, 0); set_phase('brake')
    elif phase == 'brake':
        drive(0, 0)
        if tp > 0.5: d.ctrl[FOLD] = DEPLOY; set_phase('deploy')
    elif phase == 'deploy':
        if d.qpos[FOLD_Q] < 0.04 and tp > 0.4: target = pos.copy(); fc.reset(); set_phase('climb')
    if phase in ('climb', 'transit', 'descend'):
        if phase == 'climb':
            target[2] = min(zc, target[2] + 0.9*DT); vd = (0, 0, 0.9 if target[2] < zc else 0)
            if pos[2] > zc - 0.06: set_phase('transit')
        elif phase == 'transit':
            target[0] = min(xland, target[0] + 0.7*DT); vd = (0.7 if target[0] < xland else 0, 0, 0)
            if abs(pos[0] - xland) < 0.03 and np.linalg.norm(v) < 0.1: set_phase('descend')
        else:
            target[2] = max(TOP - 0.05, target[2] - 0.35*DT); vd = (0, 0, -0.35)
            if pos[2] < TOP + 0.004 and tp > 0.5: set_phase('touchdown')
        u, thrusts = fc.step(DT, pos, q, v, w, p_des=target, v_des=vd)
        d.ctrl[THR] = u
        rpy = quat_to_rpy(q); max_tilt = max(max_tilt, abs(rpy[0]), abs(rpy[1]))
        if pos[0] > 0.55 and pos[0] < LAND_X0:   # over the stairs: clearance of lowest point to step below
            step_h = 0.17*min(4, max(0, math.floor((pos[0]+0.16 - 0.80)/0.28) + 1)) if pos[0]+0.16 > 0.80 else 0
            min_clear = min(min_clear, pos[2] - step_h)
    if phase == 'touchdown':
        d.ctrl[THR] = 0
        if tp > 0.4: d.ctrl[FOLD] = STOW; set_phase('stow'); landed_z = float(pos[2])
    elif phase == 'stow':
        if tp > 0.9: set_phase('roll')
    elif phase == 'roll':
        drive(24, 24); driving = 1
        if tp > 1.0: drive(0, 0); set_phase('settle')
    elif phase == 'settle':
        if tp > 0.6: set_phase('done')
    mj.mj_step(m, d); t += DT
    bat.step(DT, thrusts, driving)
    if t >= next_frame:
        next_frame += 1/FPS
        look = 0.92*look + 0.08*np.array([d.qpos[0], 0, d.qpos[2] + 0.05])
        cam.lookat[:] = look; r.update_scene(d, cam); img = r.render()
        frames.append(img)
        log.append(dict(t=round(t, 3), phase=phase, x=round(float(d.qpos[0]), 3), z=round(float(d.qpos[2]), 3),
                        fold=round(float(d.qpos[FOLD_Q]), 3), batt=round(bat.pct(), 2)))
        for key, cond in (('1_drive', phase == 'drive' and t > 0.5), ('2_deploy', phase == 'deploy' and tp > 0.2),
                          ('3_climb', phase == 'climb' and pos[2] > 0.45), ('4_transit', phase == 'transit' and pos[0] > 1.4),
                          ('5_landed', phase == 'stow' and tp > 0.05), ('6_rollaway', phase == 'roll' and tp > 0.7)):
            if cond and key not in stills: stills[key] = img
report['stair_hop'] = dict(completed=phase == 'done', end_phase=phase, duration_s=round(t, 2),
    landed_on_landing=bool(landed_z is not None and landed_z > TOP - 0.01), landed_z=landed_z,
    final_x=float(d.qpos[0]), final_z=float(d.qpos[2]), max_tilt_deg=math.degrees(max_tilt),
    min_clearance_over_steps_m=float(min_clear), battery_pct_used=round(100 - bat.pct(), 2),
    phases_log=[l for i, l in enumerate(log) if i == 0 or l['phase'] != log[i-1]['phase']])

# --------- write video with honest label ---------
from PIL import Image, ImageDraw, ImageFont
try: font = ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf', 22); fs = ImageFont.truetype('/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf', 17)
except Exception: font = fs = ImageFont.load_default()
def label(img, entry):
    im = Image.fromarray(img); dr = ImageDraw.Draw(im, 'RGBA')
    dr.rectangle([0, 0, W, 40], fill=(10, 37, 64, 200))
    dr.text((16, 9), 'TBD (working name)  ·  MuJoCo physics sim  ·  estimated parameters, not measured', font=font, fill=(255, 255, 255))
    dr.text((16, H - 30), f"t {entry['t']:5.2f}s   mode {entry['phase']:<9}  alt {entry['z']:.2f} m   battery {entry['batt']:.1f}%", font=fs, fill=(20, 30, 40))
    return np.asarray(im)
import imageio.v2 as iio
lab = [label(f, e) for f, e in zip(frames, log)]
iio.mimwrite(OUT / 'TBD_mujoco_stair_hop_v0.1.mp4', lab, fps=FPS, codec='libx264', quality=6,
             pixelformat='yuv420p', output_params=['-movflags', '+faststart'])
for k, img in stills.items():
    iio.imwrite(OUT / f'still_{k}.png', label(img, log[[i for i, f in enumerate(frames) if f is img][0]]))
json.dump(report, open(OUT / 'validation_report.json', 'w'), indent=1)
print(json.dumps({k: v for k, v in report.items() if k != 'stair_hop'}, indent=1))
print(json.dumps({k: v for k, v in report['stair_hop'].items() if k != 'phases_log'}, indent=1))
for l in report['stair_hop']['phases_log']: print(l)
