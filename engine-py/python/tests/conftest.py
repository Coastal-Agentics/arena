import importlib
import json
import os
import pathlib
from types import ModuleType

import numpy as np
import pytest

FIXTURE = pathlib.Path(__file__).resolve().parents[2] / "tests" / "fixtures" / "determinism.json"


def arena_env(name: str) -> str:
    """Read ``COASTAL_ARENA_<name>`` from the environment (empty if unset)."""
    return os.environ.get(f"COASTAL_ARENA_{name}", "")


# What the test run expects installed: "none" (the bare wheel, numpy only), "all"
# (the [all] extras), or unset (whatever is there; env tests skip without extras).
EXTRAS = arena_env("EXTRAS")


def need(module: str) -> ModuleType:
    """Import an extra's module for an env test: skip without it, unless the run
    expects every extra (COASTAL_ARENA_EXTRAS=all), where a missing one fails."""
    if EXTRAS == "all":
        return importlib.import_module(module)
    return pytest.importorskip(module)


def reference_actions(step: int, rows: int, action_len: int) -> np.ndarray:
    """engine_py::reference_action: ((7*step + 5*row + 3*j) % 17) / 8 - 1, exact in f32."""
    k = np.arange(rows)[:, None]
    j = np.arange(action_len)[None, :]
    return (((7 * step + 5 * k + 3 * j) % 17) / 8.0 - 1.0).astype(np.float32)


@pytest.fixture(scope="session")
def cases() -> list[dict]:
    return json.loads(FIXTURE.read_text())["cases"]
