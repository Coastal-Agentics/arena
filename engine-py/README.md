# saltmarsh-arena

The arena's deterministic Rust games (Tank Arena, Racing) as Python training
environments: a zero-copy `FlatEnv` over numpy views, a PettingZoo `ParallelEnv`, and a
single-agent Gymnasium `Env`.

The base install needs only numpy. The envs are extras: `pip install
"saltmarsh-arena[pettingzoo]"`, `"saltmarsh-arena[gym]"` or `"saltmarsh-arena[all]"`. The simulation stays in Rust; every match is an
ordinary arena replay that verifies natively and plays in the browser viewer.

```python
import saltmarsh_arena as sa                       # with the [pettingzoo] extra

env = sa.parallel_env("racing", learning=[0, 1])   # cars 2 and 3 drive scripted
obs, infos = env.reset(seed=7)
while env.agents:
    actions = {a: env.action_space(a).sample() for a in env.agents}
    obs, rewards, terms, truncs, infos = env.step(actions)
print(infos, sa.verify_replay("racing", env.replay_json()))
```

Built from `engine-py/` with maturin (`maturin build --release`). Not published yet.
See [docs/engine/python.md](https://github.com/Coastal-Agentics/arena/blob/main/docs/engine/python.md).
