"""Python step throughput, the same loops as `examples/bench.rs`:
`python bench/bench.py [steps]` (after installing the wheel).

FlatEnv is the zero-copy path (write the action view, call step). ParallelEnv adds
the PettingZoo dicts and observation copies, and needs the [pettingzoo] extra.
"""

import sys
import time

import numpy as np

import coastal_arena as ca

STEPS = int(sys.argv[1]) if len(sys.argv) > 1 else 200_000


def patterns(rows, n):
    k = np.arange(rows)[:, None]
    j = np.arange(n)[None, :]
    return [(((7 * s + 5 * k + 3 * j) % 17) / 8.0 - 1.0).astype(np.float32) for s in range(17)]


def flat(game, builds, learning, fs, steps):
    f = ca.FlatEnv(game, builds, learning, fs)
    pats = patterns(len(learning), f.core.action_len)
    acts, step, core = f.actions, f.core.step, f.core
    seed, ticks = 0, 0
    t = time.perf_counter()
    for s in range(steps):
        acts[...] = pats[s % 17]
        before = core.tick
        over = step()
        ticks += core.tick - before
        if over:
            seed += 1
            core.reset(seed)
    secs = time.perf_counter() - t
    return steps / secs, ticks / secs


def parallel(game, builds, learning, fs, steps):
    env = ca.parallel_env(game, builds, learning, fs)
    pats = patterns(len(learning), env.flat.core.action_len)
    names = env.possible_agents
    seed = 0
    env.reset(seed=seed)
    t = time.perf_counter()
    for s in range(steps):
        p = pats[s % 17]
        env.step({a: p[k] for k, a in enumerate(names) if a in env.agents})
        if not env.agents:
            seed += 1
            env.reset(seed=seed)
    return steps / (time.perf_counter() - t)


def main():
    t = ca.default_build("tank")
    r = ca.default_build("racing")
    cases = [
        ("tank 2 learning", "tank", [t, t], [0, 1]),
        ("tank 1 learning + charger", "tank", [t, t], [0]),
        ("racing 4 learning", "racing", [r] * 4, [0, 1, 2, 3]),
        ("racing 1 learning + 3 followers", "racing", [r] * 4, [0]),
    ]
    print("case,frame_skip,flat_steps_per_sec,flat_ticks_per_sec,parallel_env_steps_per_sec")
    for fs in (1, 4):
        for name, game, builds, learning in cases:
            a, b = flat(game, builds, learning, fs, STEPS)
            c = parallel(game, builds, learning, fs, STEPS // 4)
            print(f"{name},{fs},{a:.0f},{b:.0f},{c:.0f}")


if __name__ == "__main__":
    main()
