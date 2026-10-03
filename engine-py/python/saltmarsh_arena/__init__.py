"""The arena's deterministic Rust games as training environments.

Every game runs in Rust (`engine-py`, pyo3); Python only sends actions and reads
observations. A match driven from here is an ordinary arena replay: the same seed,
builds and actions give the same ``final_hash`` and replay JSON as a native run.

Three layers, thinnest first:

- :class:`FlatEnv`: one match over numpy views of buffers the Rust side owns
  (zero-copy, no allocation per step). Use it for speed or a custom loop.
- :func:`parallel_env`: a PettingZoo ``ParallelEnv`` (every learning agent at once).
- :func:`gym_env`: a Gymnasium ``Env`` with one learning agent; the others play
  their build's scripted behavior in Rust.
"""

from __future__ import annotations

import json
from typing import Any, cast

from . import _core
from ._core import engine_version, verify_replay
from .flat import FlatEnv, Build
from .envs import ArenaGymEnv, ArenaParallelEnv, gym_env, parallel_env

__all__ = [
    "ArenaGymEnv",
    "ArenaParallelEnv",
    "Build",
    "FlatEnv",
    "catalog",
    "default_build",
    "engine_version",
    "games",
    "gym_env",
    "parallel_env",
    "validate_build",
    "verify_replay",
]


def games() -> list[dict[str, Any]]:
    """Every game: ``{"game", "rules_version", "obs_len", "action_len", "min_agents",
    "max_agents"}``, in catalog order (tank, racing)."""
    keys = ("game", "rules_version", "obs_len", "action_len", "min_agents", "max_agents")
    return [dict(zip(keys, g)) for g in _core.games()]


def catalog(game: str) -> dict[str, Any]:
    """The game's build catalog: budget, stats with per-level values, scripted
    behaviors, presets and default build."""
    return cast(dict[str, Any], json.loads(_core.catalog_json(game)))


def default_build(game: str) -> dict[str, Any]:
    """The game's default build (tank: 3/3/3 charger; racing: 3/3/3 follower)."""
    return cast(dict[str, Any], json.loads(_core.default_build_json(game)))


def validate_build(game: str, build: Build) -> dict[str, Any]:
    """Check a build against the game's catalog: ``{"ok": True, "levels",
    "behavior", "points", "params", ...}`` or ``{"ok": False, "errors": [{"code",
    "key"}, ...]}``. The same check the viewer's ``validateBuild`` runs."""
    text = build if isinstance(build, str) else json.dumps(build)
    return cast(dict[str, Any], json.loads(_core.validate_build_json(game, text)))
