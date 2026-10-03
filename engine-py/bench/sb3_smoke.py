"""SB3 PPO smoke run (GATE-003 M3, optional): `python bench/sb3_smoke.py [game] [steps]`.

Needs `stable-baselines3` (and torch, CPU is fine), which the wheel does not depend
on. Trains PPO on the single-agent Gymnasium env against the build's scripted
opponent, then plays one deterministic episode and verifies its replay natively.
"""

import sys
import time

import saltmarsh_arena as sa
from stable_baselines3 import PPO

game = sys.argv[1] if len(sys.argv) > 1 else "tank"
steps = int(sys.argv[2]) if len(sys.argv) > 2 else 10_000

env = sa.gym_env(game)
t = time.perf_counter()
model = PPO("MlpPolicy", env, n_steps=1024, batch_size=256, seed=0, device="cpu", verbose=0)
model.learn(total_timesteps=steps)
train = time.perf_counter() - t

obs, info = env.reset(seed=1)
done, ret = False, 0.0
while not done:
    action, _ = model.predict(obs, deterministic=True)
    obs, reward, term, trunc, info = env.step(action)
    ret += reward
    done = term or trunc
assert sa.verify_replay(game, env.replay_json()) == info["final_hash"]
print(f"{game}: PPO {steps} steps in {train:.1f}s; eval return {ret:.3f}, "
      f"{info.get('reason', 'agent left play')}, final_hash {info['final_hash']} verified")
