"""TBD flight + drive controller (simple, hand-tuned). Mirrored 1:1 in web/tbd_control.js.
Cascade: position/velocity PD -> desired tilt + collective; attitude PD (body rates) -> torques;
analytic X-mixer -> 4 duct thrusts -> actuator ctrl (0..1 of T_MAX).  Also a battery/power model."""
import math, numpy as np

G = 9.81
P = dict(
    mass=0.970, t_max=5.50, k_yaw=0.012, x_arm=0.055, y_arm=0.1115,
    I=(0.0046, 0.0040, 0.0075),          # composite inertia estimate (body box + 4 rotor point masses)
    att_wn=14.0, att_zeta=0.9, yaw_kp=16.0, yaw_kd=8.0,
    pos_kp=5.0, pos_kd=4.0, alt_kp=8.0, alt_kd=5.0, alt_ki=2.0,
    max_tilt=0.40, max_acc=4.0,
    # power model (BOM v0.1): 2.3 g/W at hover, P ~ T^1.5 off hover, 6S 850 mAh = 18.9 Wh, 80 % usable
    g_per_w=2.3, avionics_w=6.0, pack_wh=18.9, usable=0.80, wheel_w=1.0,
)
SPIN = np.array([1, -1, -1, 1])            # FL FR RL RR (yaw reaction sign, matches tbd.xml gear)
XS = np.array([1, 1, -1, -1]) * P['x_arm']
YS = np.array([1, -1, 1, -1]) * P['y_arm']

def quat_to_rpy(q):
    w, x, y, z = q
    roll = math.atan2(2*(w*x + y*z), 1 - 2*(x*x + y*y))
    pitch = math.asin(max(-1, min(1, 2*(w*y - z*x))))
    yaw = math.atan2(2*(w*z + x*y), 1 - 2*(y*y + z*z))
    return roll, pitch, yaw

def wrap(a): return (a + math.pi) % (2*math.pi) - math.pi

def mix(F, tx, ty, tz):
    """thrust per duct (N) for collective F and body torques; symmetric X layout, analytic inverse."""
    f = F/4 + tx/(4*YS) - ty/(4*XS) + tz/(4*SPIN*P['k_yaw'])
    return np.clip(f, 0, P['t_max'])

class FlightController:
    def __init__(self):
        self.iz = 0.0
    def reset(self): self.iz = 0.0
    def step(self, dt, pos, quat, vel, omega_body, p_des, v_des=(0, 0, 0), yaw_des=0.0, a_ff=(0, 0, 0)):
        roll, pitch, yaw = quat_to_rpy(quat)
        e = np.asarray(p_des) - pos; ev = np.asarray(v_des) - vel
        a = P['pos_kp']*e[:2] + P['pos_kd']*ev[:2] + np.asarray(a_ff[:2])
        n = np.linalg.norm(a)
        if n > P['max_acc']: a *= P['max_acc']/n
        cy, sy = math.cos(yaw), math.sin(yaw)
        axb, ayb = cy*a[0] + sy*a[1], -sy*a[0] + cy*a[1]
        pitch_d = max(-P['max_tilt'], min(P['max_tilt'], math.atan2(axb, G)))
        roll_d = max(-P['max_tilt'], min(P['max_tilt'], -math.atan2(ayb, G)))
        self.iz = max(-2, min(2, self.iz + e[2]*dt))
        az = P['alt_kp']*e[2] + P['alt_kd']*ev[2] + P['alt_ki']*self.iz + a_ff[2]
        F = P['mass']*(G + az)/max(0.5, math.cos(roll)*math.cos(pitch))
        wn, z = P['att_wn'], P['att_zeta']
        Ix, Iy, Iz = P['I']
        tx = Ix*(wn*wn*(roll_d - roll) - 2*z*wn*omega_body[0])
        ty = Iy*(wn*wn*(pitch_d - pitch) - 2*z*wn*omega_body[1])
        tz = Iz*(P['yaw_kp']*wrap(yaw_des - yaw) - P['yaw_kd']*omega_body[2])
        f = mix(F, tx, ty, tz)
        return f / P['t_max'], f

class Battery:
    def __init__(self):
        self.wh_used = 0.0
        self.th = P['mass']*G/4                                   # N per duct at hover
        self.ph = (self.th/G*1000)/P['g_per_w']                   # W per duct at hover
    def power(self, thrusts_n, driving=0.0):
        p = sum(self.ph*(max(t, 0)/self.th)**1.5 for t in thrusts_n)
        return p + P['avionics_w'] + driving*4*P['wheel_w']
    def step(self, dt, thrusts_n, driving=0.0):
        self.wh_used += self.power(thrusts_n, driving)*dt/3600
        return self.pct()
    def pct(self): return max(0.0, 100*(1 - self.wh_used/(P['pack_wh']*P['usable'])))
    def hover_minutes(self):
        return P['pack_wh']*P['usable']/(4*self.ph + P['avionics_w'])*60
