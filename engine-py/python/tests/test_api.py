import json

import numpy as np
import pytest

import saltmarsh_arena as sa
from saltmarsh_arena import _core


def tank(a, s, d, behavior="kiter"):
    return {"rules_version": 1, "levels": {"attack": a, "speed": s, "defense": d},
            "behavior": {"kind": "scripted", "id": behavior}}


def car(p, t, g, behavior="follower"):
    return {"rules_version": 1, "levels": {"power": p, "top_speed": t, "grip": g},
            "behavior": {"kind": "scripted", "id": behavior}}


def test_games_and_catalogs():
    assert [(g["game"], g["obs_len"], g["action_len"], g["min_agents"], g["max_agents"]) for g in sa.games()] == [
        ("tank", 176, 4, 2, 2),
        ("racing", 43, 2, 1, 4),
    ]
    for g in ("tank", "racing"):
        cat = sa.catalog(g)
        assert cat["default_build"] == sa.default_build(g)
        v = sa.validate_build(g, sa.default_build(g))
        assert v["ok"] and v["game"] == g and v["points"] == cat["budget"]
    bad = sa.validate_build("tank", tank(5, 3, 3))
    assert bad == {"ok": False, "errors": [{"code": "over_budget", "key": "levels"}]}
    assert sa.validate_build("chess", "{}")["errors"][0]["code"] == "wrong_game"
    assert sa.validate_build("racing", "not json")["errors"][0]["code"] == "invalid_json"
    with pytest.raises(ValueError, match="unknown game"):
        sa.catalog("chess")
    assert sa.engine_version()


def test_flat_env_buffers_are_views():
    f = sa.FlatEnv("racing", [car(3, 3, 3), car(5, 2, 2, "cutter"), car(2, 2, 5, "blocker")], learning=[2, 0])
    assert f.obs.shape == (2, 43) and f.obs.dtype == np.float32
    assert f.actions.shape == (2, 2) and f.rewards.shape == (2,)
    assert f.agent_names == ["car_0", "car_1", "car_2"] and f.learning == [2, 0]
    ptr = f.obs.__array_interface__["data"][0]
    before = f.obs.copy()
    f.actions[:] = [[1.0, 0.0], [1.0, 0.5]]
    assert f.step() is False
    assert f.obs.__array_interface__["data"][0] == ptr  # rewritten in place
    assert not np.array_equal(before, f.obs)
    assert f.tick == 4
    assert np.all(f.obs >= -1) and np.all(f.obs <= 1)
    assert f.active.tolist() == [True, True]
    with pytest.raises(ValueError):
        f.obs[0, 0] = 2.0  # read-only view
    # The buffer can't be resized under a view.
    with pytest.raises(BufferError):
        f.core.obs_buffer.extend(b"0000")


def test_errors_name_the_problem():
    with pytest.raises(ValueError, match="unknown game"):
        sa.FlatEnv("chess")
    with pytest.raises(ValueError, match="tank takes 2 builds, got 1"):
        sa.FlatEnv("tank", [tank(3, 3, 3)])
    with pytest.raises(ValueError, match=r"builds\[1\] is invalid: .*over_budget"):
        sa.FlatEnv("tank", [tank(3, 3, 3), tank(5, 3, 3)])
    with pytest.raises(ValueError, match="racing takes 1 to 4 builds, got 5"):
        sa.FlatEnv("racing", [car(3, 3, 3)] * 5)
    with pytest.raises(ValueError, match="out of range"):
        sa.FlatEnv("tank", learning=[2])
    with pytest.raises(ValueError, match="listed twice"):
        sa.FlatEnv("tank", learning=[0, 0])
    with pytest.raises(ValueError, match="frame_skip"):
        sa.FlatEnv("tank", frame_skip=0)
    champ = {**tank(3, 3, 3), "behavior": {"kind": "champion", "ref": "gen-1"}}
    with pytest.raises(ValueError, match="champion build"):
        sa.FlatEnv("tank", [tank(3, 3, 3), champ], learning=[0])
    sa.FlatEnv("tank", [tank(3, 3, 3), champ], learning=[1])  # a learning champion is fine
    with pytest.raises(ValueError, match="verify|mismatch|hash"):
        f = sa.FlatEnv("tank")
        f.step()
        bad = json.loads(f.replay_json())
        bad["final_hash"] = "0" * 16
        sa.verify_replay("tank", json.dumps(bad))
    with pytest.raises(ValueError):
        sa.verify_replay("racing", sa.FlatEnv("tank").replay_json())


def test_scripted_match_and_outcome():
    f = sa.FlatEnv("tank", [tank(5, 3, 1, "kiter"), tank(4, 1, 4, "charger")], learning=[], seed=42)
    steps = 0
    while not f.step():
        steps += 1
    o = f.outcome()
    assert o["reason"] in {"last_standing", "all_destroyed", "tick_limit"}
    assert f.tick == o["ticks"] and f.is_over
    assert f.step() is True and f.tick == o["ticks"]  # a no-op once over
    assert sa.verify_replay("tank", f.replay_json()) == f.state_hash()
    replay = json.loads(f.replay_json())
    assert replay["seed"] == "42" and replay["setup_hash"] == f.setup_hash()


def test_reset_repeats_a_seed():
    f = sa.FlatEnv("racing", [car(3, 3, 3)] * 2)
    runs = []
    for seed in (5, 6, 5):
        f.reset(seed)
        for _ in range(50):
            f.step(np.array([[1.0, 0.2], [0.8, -0.1]], np.float32))
        runs.append((f.state_hash(), f.replay_json()))
    assert runs[0] == runs[2] and runs[0] != runs[1]


def test_large_seed_and_nan_actions():
    f = sa.FlatEnv("tank", seed=2**64 - 1)
    assert f.seed == 2**64 - 1
    f.actions[:] = np.nan
    f.step()
    assert np.isfinite(f.obs).all()
    with pytest.raises(OverflowError):
        f.reset(2**64)
