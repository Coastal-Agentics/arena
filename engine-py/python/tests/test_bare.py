"""Thin by default: the base install is numpy only, and the envs name their extra."""

import subprocess
import sys
import textwrap

import pytest

from conftest import EXTRAS

BASE = """
import sys
import numpy as np
import coastal_arena as ca
f = ca.FlatEnv("racing", learning=[0])
f.step(np.ones((1, 2), np.float32))
assert ca.verify_replay("racing", f.replay_json()) == f.state_hash()
assert ca.catalog("tank")["default_build"] == ca.default_build("tank")
assert ca.validate_build("racing", ca.default_build("racing"))["ok"]
assert len(ca.games()) == 2 and ca.engine_version()
loaded = sorted(m for m in ("gymnasium", "pettingzoo") if m in sys.modules)
assert loaded == [], loaded
print("ok")
"""


def run(code: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run([sys.executable, "-c", textwrap.dedent(code)], capture_output=True, text=True)


def test_base_api_never_imports_the_extras():
    r = run(BASE)
    assert r.returncode == 0 and r.stdout.strip() == "ok", r.stderr


# Each env with its extra's modules blocked (a None entry in sys.modules makes the
# import fail), so the message is checked whether or not the extras are installed.
@pytest.mark.parametrize("call,blocked,extra", [
    ("ca.parallel_env('tank')", ["pettingzoo"], "coastal-arena[pettingzoo]"),
    ("ca.parallel_env('tank')", ["gymnasium", "pettingzoo"], "coastal-arena[pettingzoo]"),
    ("ca.gym_env('racing')", ["gymnasium"], "coastal-arena[gym]"),
    ("ca.ArenaGymEnv", ["gymnasium"], "coastal-arena[gym]"),
    ("ca.ArenaParallelEnv", ["pettingzoo"], "coastal-arena[pettingzoo]"),
])
def test_a_missing_extra_names_itself(call, blocked, extra):
    r = run(f"""
    import sys
    for m in {blocked!r}:
        sys.modules[m] = None
    import coastal_arena as ca
    try:
        {call}
    except ca.MissingExtraError as e:
        assert isinstance(e, ImportError)
        print(e)
    else:
        raise SystemExit("no error")
    """)
    assert r.returncode == 0, r.stderr
    assert f'pip install "{extra}"' in r.stdout and "coastal-arena[all]" in r.stdout


def test_extras_match_the_run():
    import importlib.util

    have = {m: importlib.util.find_spec(m) is not None for m in ("gymnasium", "pettingzoo")}
    if EXTRAS == "none":
        assert have == {"gymnasium": False, "pettingzoo": False}, "the bare run has extras installed"
        import coastal_arena as ca

        with pytest.raises(ca.MissingExtraError, match=r"coastal-arena\[pettingzoo\]"):
            ca.parallel_env("tank")
        with pytest.raises(ca.MissingExtraError, match=r"coastal-arena\[gym\]"):
            ca.gym_env("tank")
    elif EXTRAS == "all":
        assert have == {"gymnasium": True, "pettingzoo": True}
    else:
        pytest.skip("COASTAL_ARENA_EXTRAS not set")
