import json
import pathlib

import numpy as np
import pytest

FIXTURE = pathlib.Path(__file__).resolve().parents[2] / "tests" / "fixtures" / "determinism.json"


def reference_actions(step: int, rows: int, action_len: int) -> np.ndarray:
    """engine_py::reference_action: ((7*step + 5*row + 3*j) % 17) / 8 - 1, exact in f32."""
    k = np.arange(rows)[:, None]
    j = np.arange(action_len)[None, :]
    return (((7 * step + 5 * k + 3 * j) % 17) / 8.0 - 1.0).astype(np.float32)


@pytest.fixture(scope="session")
def cases() -> list[dict]:
    return json.loads(FIXTURE.read_text())["cases"]
