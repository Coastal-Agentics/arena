"""Python-run episodes against native Rust runs: the same replay bytes and final_hash.

The native side is engine_py::reference_episode (Match::step_policies with closures,
no Session), and its final hashes are pinned in tests/fixtures/determinism.json by
the Rust test engine-py/tests/fixture.rs.
"""

import json
import os
import pathlib

import pytest

import saltmarsh_arena as sa
from saltmarsh_arena import _core
from conftest import reference_actions


def record(name, replay):
    """Write a Python-run replay for the native verifier (`examples/verify_replay.rs`)."""
    out = os.environ.get("SALTMARSH_ARENA_REPLAY_DIR")
    if out:
        path = pathlib.Path(out)
        path.mkdir(parents=True, exist_ok=True)
        (path / f"{name}.json").write_text(replay)


def texts(case):
    return [json.dumps(b) for b in case["builds"]]


def native(case, max_steps=None):
    if max_steps is None:
        max_steps = case["max_steps"]
    return _core.reference_replay(case["game"], texts(case), case["learning"], case["frame_skip"], case["seed"], max_steps)


def run_flat(case):
    f = sa.FlatEnv(case["game"], case["builds"], case["learning"], case["frame_skip"], seed=case["seed"])
    rows, n = len(case["learning"]), f.core.action_len
    limit = case["max_steps"] if case["max_steps"] is not None else 2**32
    step = 0
    while step < limit:
        over = f.step(reference_actions(step, rows, n))
        step += 1
        if over:
            break
    return f.replay_json(), f.state_hash()


def run_parallel(case):
    env = sa.parallel_env(case["game"], case["builds"], case["learning"], case["frame_skip"])
    env.reset(seed=case["seed"])
    rows, n = len(case["learning"]), env.flat.core.action_len
    limit = case["max_steps"] if case["max_steps"] is not None else 2**32
    step, last = 0, {}
    while env.agents and step < limit:
        acts = reference_actions(step, rows, n)
        actions = {a: acts[k] for k, a in enumerate(env.possible_agents) if a in env.agents}
        _, _, _, _, infos = env.step(actions)
        last.update(infos)
        step += 1
    return env.replay_json(), env.flat.state_hash(), last


@pytest.mark.parametrize("name", [
    "tank-duel-both-learning", "tank-blue-learning-vs-charger", "tank-1000-steps", "tank-all-scripted",
    "racing-four-learning", "racing-car2-learning-vs-scripted", "racing-1000-steps", "racing-all-scripted",
    "racing-solo",
])
def test_python_run_equals_native_run(cases, name):
    case = next(c for c in cases if c["name"] == name)
    ref = native(case)
    replay, final = run_flat(case)
    record(name, replay)
    assert replay == ref
    assert final == case["final_hash"] == json.loads(ref)["final_hash"]
    assert sa.verify_replay(case["game"], replay) == case["final_hash"]
    r = json.loads(replay)
    assert r["outcome"] == case["outcome"]
    assert len(r["actions"]) == case["ticks"]


@pytest.mark.parametrize("name", ["tank-duel-both-learning", "tank-1000-steps", "racing-four-learning", "racing-car2-learning-vs-scripted"])
def test_parallel_env_run_equals_native_run(cases, name):
    case = next(c for c in cases if c["name"] == name)
    if not case["learning"]:
        pytest.skip("no PettingZoo agents")
    replay, final, infos = run_parallel(case)
    if case["max_steps"] is None and case["game"] == "racing":
        # Agents leave as their cars finish; the scripted cars race on to the end, so
        # compare with the native run cut at the same tick.
        ticks = json.loads(replay)["actions"]
        assert final == json.loads(_core.reference_replay(case["game"], texts(case), case["learning"],
                                                          case["frame_skip"], case["seed"],
                                                          -(-len(ticks) // case["frame_skip"])))["final_hash"]
    else:
        assert replay == native(case)
        assert final == case["final_hash"]
        if case["outcome"] is not None:
            assert {i["final_hash"] for i in infos.values()} == {case["final_hash"]}
    assert sa.verify_replay(case["game"], replay) == final


def test_gym_env_run_equals_native_run(cases):
    case = next(c for c in cases if c["name"] == "tank-blue-learning-vs-charger")
    env = sa.gym_env("tank", case["builds"], agent=0, frame_skip=case["frame_skip"])
    env.reset(seed=case["seed"])
    step, done = 0, False
    while not done:
        _, _, term, trunc, info = env.step(reference_actions(step, 1, 4)[0])
        step += 1
        done = term or trunc
    assert env.replay_json() == native(case)
    assert info["final_hash"] == case["final_hash"]
    assert info["winner"] == "orange_1" and info["reason"] == "last_standing"
