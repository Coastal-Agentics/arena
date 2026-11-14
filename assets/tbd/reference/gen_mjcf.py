"""Generate tbd.xml (TBD vehicle + floor + 17 cm staircase test scene). All numbers: see PARAMS.md."""
from pathlib import Path
import math
HERE = Path(__file__).parent

# ---------------- estimated parameters (sources in PARAMS.md) ----------------
M_TOTAL   = 0.970            # kg all-up (BOM v0.2 recommended estimate)
M_ROTOR   = 0.0624           # kg per rotor module (duct 29.8 + guard 8 + arm 2.2 + motor 18 + prop 3 + pin 1.4 g)
M_WHEEL   = 0.0112           # kg per wheel (hub 5.8 + tire 5.4 g); N20 gearmotors are in the body
M_BODY    = M_TOTAL - 4*M_ROTOR - 4*M_WHEEL
T_MAX     = 5.50             # N max thrust per duct (561 g EMAX 6S bench on 3in prop, D76 ducted unmeasured)
K_YAW     = 0.012            # m, prop reaction torque / thrust (typical 3in prop, est.)
TAU_MOTOR = 0.04             # s, motor/ESC spin-up first-order lag (est.)
W_WHEEL   = 26.0             # rad/s, N20 1:100 no-load at ~5 V (~250 rpm, est.)
TQ_WHEEL  = 0.08             # N m, wheel stall torque at ~5 V (est.)
HINGE     = (0.0125, 0.0725, 0.037)
DUCT_C    = (0.0425, 0.039, 0.0)   # duct centre in hinge-local (deployed) frame; x sign A=+ / B=-
WHEEL_C   = (0.066, 0.038, 0.022)
RISER, TREAD, NSTEP, STAIR_X0, STAIR_W = 0.17, 0.28, 4, 0.80, 1.0

def f(*v): return ' '.join(f'{x:.6g}' for x in v)

def body_inertia(m, a, b, c):   # solid box approx, dims full
    return f(m/12*(b*b+c*c), m/12*(a*a+c*c), m/12*(a*a+b*b))

rotors = []
for name, sx, sy, var, spin in [('FL', 1, 1, 'A', 1), ('FR', 1, -1, 'B', -1), ('RL', -1, 1, 'B', -1), ('RR', -1, -1, 'A', 1)]:
    # hinge-local x sign of the duct centre: A -> +, B -> -
    dsx = 1 if var == 'A' else -1
    euler = '0 0 3.14159265' if sy < 0 else '0 0 0'
    cx, cy, cz = dsx*DUCT_C[0], DUCT_C[1], DUCT_C[2]
    # ring inertia about duct axis ~ m r^2 (r~41 mm), transverse ~ half (est.)
    Ia, It = M_ROTOR*0.041**2*0.8, M_ROTOR*0.041**2*0.45
    eq = '' if name == 'FL' else f'    <joint name="link_{name}" joint1="fold_{name}" joint2="fold_FL" polycoef="0 1 0 0 0" solref="0.004 1"/>\n'
    rotors.append(dict(name=name, eq=eq, xml=f'''
      <body name="arm_{name}" pos="{f(sx*HINGE[0], sy*HINGE[1], HINGE[2])}" euler="{euler}">
        <joint name="fold_{name}" type="hinge" axis="1 0 0" range="-0.02 1.59" damping="0.02" armature="0.0005"/>
        <inertial pos="{f(cx*0.95, cy*0.92, cz)}" mass="{M_ROTOR}" diaginertia="{f(It, It, Ia)}"/>
        <geom class="vis" mesh="rotor{var}_duct" material="slate"/>
        <geom class="vis" mesh="rotor{var}_motor" material="metal"/>
        <geom class="vis" mesh="rotor{var}_prop" material="prop" name="prop_{name}"/>
        <geom class="col" name="duct_{name}" type="cylinder" pos="{f(cx, cy, cz)}" size="0.0435 0.0145" friction="0.6"/>
        <site name="thrust_{name}" pos="{f(cx, cy, cz+0.007)}" size="0.006" rgba="1 0.5 0 0.6" group="4"/>
      </body>''', act=f'''    <general name="thrust_{name}" site="thrust_{name}" gear="0 0 1 0 0 {spin*K_YAW}" ctrlrange="0 1" ctrllimited="true"
             dyntype="filter" dynprm="{TAU_MOTOR}" gaintype="fixed" gainprm="{T_MAX}" biastype="none"/>'''))

wheels, wacts = [], []
for name, sx, sy in [('FL', 1, 1), ('FR', 1, -1), ('RL', -1, 1), ('RR', -1, -1)]:
    flip = ' euler="0 0 3.14159265"' if sy < 0 else ''
    wheels.append(f'''
      <body name="wheel_{name}" pos="{f(sx*WHEEL_C[0], sy*WHEEL_C[1], WHEEL_C[2])}">
        <joint name="wheel_{name}" type="hinge" axis="0 1 0" damping="0.0005" armature="0.0002"/>
        <inertial pos="0 0 0" mass="{M_WHEEL}" diaginertia="2.2e-6 4.0e-6 2.2e-6"/>
        <geom class="vis" mesh="wheel_tire" material="tire"{flip}/>
        <geom class="vis" mesh="wheel_hub" material="slate"{flip}/>
        <geom class="col" name="tire_{name}" type="cylinder" size="0.022 0.007" euler="1.5707963 0 0" friction="0.9 0.01 0.001" condim="4"/>
      </body>''')
    wacts.append(f'    <velocity name="drive_{name}" joint="wheel_{name}" kv="0.02" ctrlrange="-{W_WHEEL} {W_WHEEL}" forcerange="-{TQ_WHEEL} {TQ_WHEEL}" forcelimited="true"/>')

steps = []
for i in range(NSTEP):
    h = RISER*(i+1)
    x0 = STAIR_X0 + TREAD*i
    steps.append(f'    <geom name="step{i+1}" type="box" class="stair" pos="{f(x0 + TREAD/2, 0, h/2)}" size="{f(TREAD/2, STAIR_W/2, h/2)}"/>')
TOP = RISER*NSTEP
LAND_X0 = STAIR_X0 + TREAD*NSTEP
steps.append(f'    <geom name="landing" type="box" class="stair" pos="{f(LAND_X0 + 0.6, 0, TOP/2)}" size="{f(0.6, STAIR_W/2, TOP/2)}"/>')
# (handrail omitted)
#steps.append(f'    <geom name="rail_L" type="box" rgba="0.55 0.47 0.38 1" pos="{f(STAIR_X0 + TREAD*NSTEP/2, STAIR_W/2+0.02, TOP/2+0.45)}" size="{f(TREAD*NSTEP/2+0.05, 0.015, 0.02)}" euler="0 {-math.atan2(TOP, TREAD*NSTEP):.4f} 0"/>')

mesh_names = ['body_shell','body_slate','body_face','body_eyes','body_dark','wheel_tire','wheel_hub',
              'rotorA_duct','rotorA_motor','rotorA_prop','rotorB_duct','rotorB_motor','rotorB_prop']

xml = f'''<!-- TBD (working name) hybrid ground/flight vehicle - MuJoCo physics model v0.1.
     Coastal Agentics internal concept. Masses, thrust, motor and servo numbers are ESTIMATES from the
     BOM / CAD v0.2 (see PARAMS.md); nothing here is measured. Generated by gen_mjcf.py. -->
<mujoco model="TBD_v0.1">
  <compiler angle="radian" meshdir="meshes" autolimits="true"/>
  <option timestep="0.002" integrator="implicitfast" density="1.225" viscosity="1.8e-5" gravity="0 0 -9.81"/>
  <size memory="16M"/>
  <statistic center="0.6 0 0.3" extent="1.6"/>
  <visual>
    <global offwidth="1280" offheight="720" azimuth="-130" elevation="-18"/>
    <quality shadowsize="4096"/>
    <rgba rangefinder="1 1 0 0"/>
    <map znear="0.05" shadowclip="8"/>
    <headlight ambient="0.35 0.35 0.38" diffuse="0.5 0.5 0.5" specular="0.1 0.1 0.1"/>
  </visual>
  <default>
    <default class="vis"><geom type="mesh" contype="0" conaffinity="0" group="2" density="0"/></default>
    <default class="col"><geom group="3" rgba="0.9 0.3 0.2 0.25" density="0"/></default>
    <default class="stair"><geom material="stair" friction="0.8"/></default>
  </default>
  <asset>
    <texture name="sky" type="skybox" builtin="gradient" rgb1="0.80 0.88 0.95" rgb2="0.98 0.98 0.98" width="256" height="256"/>
    <texture name="grid" type="2d" builtin="checker" rgb1="0.86 0.86 0.84" rgb2="0.80 0.80 0.78" width="512" height="512" mark="edge" markrgb="0.7 0.7 0.68"/>
    <material name="floor" texture="grid" texrepeat="8 8" reflectance="0"/>
    <material name="stair" rgba="0.72 0.62 0.50 1"/>
    <material name="shell" rgba="0.93 0.92 0.88 1" specular="0.4" shininess="0.6"/>
    <material name="slate" rgba="0.30 0.34 0.39 1" specular="0.2"/>
    <material name="smoke" rgba="0.06 0.07 0.08 1" specular="0.8" shininess="0.9"/>
    <material name="eye" rgba="0.95 0.97 1.0 1" emission="1"/>
    <material name="metal" rgba="0.55 0.56 0.58 1" specular="0.6"/>
    <material name="tire" rgba="0.14 0.15 0.16 1"/>
    <material name="prop" rgba="0.12 0.13 0.15 0.85"/>
{chr(10).join(f'    <mesh name="{m}" file="{m}.stl"/>' for m in mesh_names)}
  </asset>

  <worldbody>
    <light name="sun" pos="1 -1 3" dir="-0.15 0.25 -1" directional="true" castshadow="true" diffuse="0.6 0.6 0.6"/>
    <geom name="floor" type="plane" size="6 6 0.05" material="floor" friction="0.9"/>
{chr(10).join(steps)}
    <camera name="side" pos="0.9 -2.4 0.75" xyaxes="1 0 0 0 0.3 1" mode="fixed"/>

    <body name="tbd" pos="0 0 0.0005">
      <freejoint name="root"/>
      <inertial pos="0 0 0.055" mass="{M_BODY:.4f}" diaginertia="{body_inertia(M_BODY, 0.21, 0.112, 0.10)}"/>
      <geom class="vis" mesh="body_shell" material="shell"/>
      <geom class="vis" mesh="body_slate" material="slate"/>
      <geom class="vis" mesh="body_face" material="smoke"/>
      <geom class="vis" mesh="body_eyes" material="eye" name="eyes"/>
      <geom class="vis" mesh="body_dark" material="metal"/>
      <geom class="col" name="body_col" type="box" pos="0 0 0.064" size="0.102 0.055 0.047" friction="0.4"/>
      <geom class="col" name="dome_col" type="ellipsoid" pos="0 0 0.095" size="0.09 0.048 0.025" friction="0.4"/>
      <site name="imu" pos="0 0 0.06" size="0.005"/>
      <site name="down" pos="0.048 0 0.014" zaxis="0 0 -1" size="0.004"/>
      <camera name="chase" mode="trackcom" pos="-0.9 -0.9 0.55" xyaxes="0.707 -0.707 0 0.25 0.25 0.93"/>
{''.join(wheels)}
{''.join(r["xml"] for r in rotors)}
    </body>
  </worldbody>

  <equality>
{''.join(r["eq"] for r in rotors)}  </equality>

  <actuator>
{chr(10).join(r["act"] for r in rotors)}
{chr(10).join(wacts)}
    <!-- One ES08MD II servo drives all four arms through the carriage/push-rod linkage (equality-coupled here).
         ctrl = target fold angle (rad): 0 = deployed (flight), 1.571 = stowed (ground). The stiff hold also stands in
         for the spring lock pins that resist thrust torque on the arms. -->
    <position name="fold" joint="fold_FL" kp="6" kv="0.15" ctrlrange="0 1.5708" forcerange="-2.5 2.5" forcelimited="true"/>
  </actuator>

  <sensor>
    <gyro name="gyro" site="imu"/>
    <accelerometer name="accel" site="imu"/>
    <framequat name="quat" objtype="site" objname="imu"/>
    <rangefinder name="range_down" site="down"/>
    <jointpos name="fold_angle" joint="fold_FL"/>
  </sensor>

  <keyframe>
    <key name="ground" qpos="0 0 0.0005 1 0 0 0  0 0 0 0  1.5708 1.5708 1.5708 1.5708" ctrl="0 0 0 0  0 0 0 0  1.5708"/>
    <key name="deployed" qpos="0 0 0.0005 1 0 0 0  0 0 0 0  0 0 0 0" ctrl="0 0 0 0  0 0 0 0  0"/>
  </keyframe>
</mujoco>
'''
(HERE / 'tbd.xml').write_text(xml)
print('wrote tbd.xml; M_BODY=%.4f' % M_BODY, 'top', TOP, 'landing x0', LAND_X0)
