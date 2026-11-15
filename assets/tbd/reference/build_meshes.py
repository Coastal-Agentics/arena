"""Export decimated visual meshes for the TBD MuJoCo model from the CAD v0.2 deployed GLB.
GLB is glTF Y-up; world (CAD) frame is X fwd, Y left, Z up:  world = (gx, -gz, gy).
Groups:  body (by material), wheel (hub+tire, wheel-local), rotor A/B (hinge-local, deployed)."""
import json, numpy as np, trimesh
from pathlib import Path
HERE = Path(__file__).parent
OUT = HERE / 'meshes'; OUT.mkdir(exist_ok=True)
scene = trimesh.load(HERE / '../cad/TBD_assembly_v0.2_flight.glb')
G2W = np.array([[1,0,0,0],[0,0,-1,0],[0,1,0,0],[0,0,0,1]], float)

def part(node):
    T, g = scene.graph[node]
    m = scene.geometry[g].copy(); m.apply_transform(G2W @ T)
    m.visual = trimesh.visual.ColorVisuals(m)
    return m

nodes = list(scene.graph.nodes_geometry)
HX, HY, HZ = 0.0125, 0.0725, 0.037
def to_hinge_local(m, sx, sy):
    m = m.copy(); m.apply_translation([-sx*HX, -sy*HY, -HZ])
    if sy < 0: m.apply_transform(trimesh.transformations.rotation_matrix(np.pi, [0,0,1]))
    return m

groups = {k: [] for k in ['body_shell','body_slate','body_face','body_eyes','body_dark','wheel_tire','wheel_hub',
                          'rotorA_duct','rotorA_motor','rotorA_prop','rotorB_duct','rotorB_motor','rotorB_prop']}
for n in nodes:
    m = part(n)
    if n.startswith(('tire_TPU_FL','wheel_hub_FL')):
        m.apply_translation([-0.066, -0.038, -0.022])
        groups['wheel_tire' if n.startswith('tire') else 'wheel_hub'].append(m); continue
    if n.startswith(('tire_TPU','wheel_hub')): continue
    if n.endswith('_flight') and any(k in n for k in ('duct_ring','motor_mount','fold_arm','REF_motor','REF_prop','hinge_pin')):
        corner = n.split('_')[-2]                 # FL FR RL RR
        if corner not in ('FL','FR'): continue    # A: FL (== RR in hinge-local), B: FR (== RL)
        var = 'A' if corner == 'FL' else 'B'
        sx = 1; sy = 1 if corner[1] == 'L' else -1
        m = to_hinge_local(m, sx, sy)
        kind = 'prop' if 'prop' in n else 'motor' if ('REF_motor' in n or 'hinge_pin' in n) else 'duct'
        groups[f'rotor{var}_{kind}'].append(m); continue
    if n == 'upper_shell': groups['body_shell'].append(m)
    elif n == 'face_eye_panel': groups['body_face'].append(m)
    elif n.startswith('eye_diffusers'): groups['body_eyes'].append(m)
    elif n.startswith('REF_'):
        if any(k in n for k in ('pushrod','servo_horn','guide_rods','lock_pin')): groups['body_dark'].append(m)
        # internal electronics not shown (hidden inside the shell)
    else: groups['body_slate'].append(m)   # tub, door, bumpers, pods, drive frame, carriage

TARGET = dict(body_shell=3000, body_slate=4000, body_face=600, body_eyes=200, body_dark=600, wheel_tire=600, wheel_hub=300,
              rotorA_duct=2500, rotorB_duct=2500, rotorA_motor=300, rotorB_motor=300, rotorA_prop=344, rotorB_prop=344)
info = {}
for k, ms in groups.items():
    m = trimesh.util.concatenate(ms)
    n0 = len(m.faces)
    if len(m.faces) > TARGET[k]:
        m = m.simplify_quadric_decimation(face_count=TARGET[k])
    m.export(OUT / f'{k}.stl')
    info[k] = dict(parts=len(ms), faces_in=n0, faces_out=len(m.faces), bounds=m.bounds.round(4).tolist())
json.dump(info, open(OUT / 'mesh_info.json', 'w'), indent=1)
for k, v in info.items(): print(k, v)
