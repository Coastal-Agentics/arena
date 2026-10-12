"""The PettingZoo and Gymnasium conformance tests (GATE-003 M3 acceptance), for both games."""

import numpy as np
import pytest

from conftest import need

need("gymnasium")
need("pettingzoo")

from gymnasium.utils.env_checker import check_env  # noqa: E402
from pettingzoo.test import parallel_api_test, parallel_seed_test  # noqa: E402

import coastal_arena as ca  # noqa: E402

SETUPS = [
    ("tank", None, None),
    ("tank", None, [1]),
    ("racing", None, None),
    ("racing", [ca.default_build("racing")] * 3, [0, 2]),
    ("racing", [ca.default_build("racing")], None),
]


@pytest.mark.parametrize("game,builds,learning", SETUPS)
def test_parallel_api(game, builds, learning):
    parallel_api_test(ca.parallel_env(game, builds, learning), num_cycles=400)


@pytest.mark.parametrize("game,builds,learning", SETUPS)
def test_parallel_seed(game, builds, learning):
    parallel_seed_test(lambda: ca.parallel_env(game, builds, learning))


@pytest.mark.parametrize("game", ["tank", "racing"])
def test_gymnasium_checker(game):
    check_env(ca.gym_env(game), skip_render_check=True)


def test_parallel_episode_shape():
    env = ca.parallel_env("tank", learning=[0, 1], frame_skip=8)
    obs, infos = env.reset(seed=3)
    assert env.agents == ["blue_0", "orange_1"]
    assert set(infos["blue_0"]) == {"seed", "tick", "setup_hash"}
    space = env.observation_space("blue_0")
    assert space.shape == (176,) and space.dtype == np.float32
    assert env.action_space("orange_1").shape == (4,)
    assert all(space.contains(o) for o in obs.values())
    total = {a: 0.0 for a in env.agents}
    while env.agents:
        acts = {a: env.action_space(a).sample() for a in env.agents}
        obs, rew, term, trunc, infos = env.step(acts)
        for a, r in rew.items():
            total[a] += r
        assert all(space.contains(o) for o in obs.values())
    end = next(iter(infos.values()))
    assert {"final_hash", "reason", "winner", "winner_team", "ticks"} <= set(end)
    assert end["final_hash"] == ca.verify_replay("tank", env.replay_json())
    assert end["winner"] in {None, "blue_0", "orange_1"}
    if end["reason"] == "tick_limit":
        assert all(trunc.values()) or any(term.values())
    else:
        assert all(term.values())


def test_racing_cars_leave_as_they_finish():
    # Car 1 learns and sits still; three scripted cars finish and the race ends.
    env = ca.parallel_env("racing", [ca.default_build("racing")] * 4, learning=[1])
    env.reset(seed=1)
    while env.agents:
        _, _, term, trunc, infos = env.step({"car_1": np.zeros(2, np.float32)})
    assert infos["car_1"]["reason"] == "finished" and term == {"car_1": True}
    assert infos["car_1"]["winner"] in {"car_0", "car_2", "car_3"}


def test_render_modes_are_rejected():
    with pytest.raises(ValueError, match="render"):
        ca.parallel_env("tank", render_mode="human")
