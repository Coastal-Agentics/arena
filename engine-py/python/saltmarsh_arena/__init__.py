"""The arena's deterministic Rust games as training environments.

Every game runs in Rust (`engine-py`, pyo3); Python only sends actions and reads
observations. A match driven from here is an ordinary arena replay: the same seed,
builds and actions give the same ``final_hash`` and replay JSON as a native run.

Three layers, thinnest first:

- :class:`FlatEnv`: one match over numpy views of buffers the Rust side owns
  (zero-copy, no allocation per step). Use it for speed or a custom loop.
- :func:`parallel_env`: a PettingZoo ``ParallelEnv`` (every learning agent at once).
  Needs ``pip install "saltmarsh-arena[pettingzoo]"``.
- :func:`gym_env`: a Gymnasium ``Env`` with one learning agent; the others play
  their build's scripted behavior in Rust. Needs ``pip install "saltmarsh-arena[gym]"``.

The base install needs only numpy: :class:`FlatEnv`, the catalog calls and
:func:`verify_replay` never import Gymnasium or PettingZoo. The env modules are
imported on first use, and a missing extra raises :class:`MissingExtraError` (an
``ImportError``) naming it.
"""

from __future__ import annotations

import importlib
import json
from collections.abc import Sequence
from types import ModuleType
from typing import TYPE_CHECKING, Any, cast

from . import _core
from ._core import engine_version, verify_replay
from .flat import Build, FlatEnv

if TYPE_CHECKING:
    from ._gymnasium import ArenaGymEnv
    from ._pettingzoo import ArenaParallelEnv

__all__ = [
    "ArenaGymEnv",
    "ArenaParallelEnv",
    "Build",
    "FlatEnv",
    "MissingExtraError",
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


class MissingExtraError(ImportError):
    """An env needs an optional extra that is not installed (or does not load)."""


# Env module -> (the extra that installs its dependencies, what they are).
_EXTRAS = {
    "_pettingzoo": ("pettingzoo", "PettingZoo and Gymnasium"),
    "_gymnasium": ("gym", "Gymnasium"),
}


def _env_module(name: str) -> ModuleType:
    """Import ``saltmarsh_arena.<name>``, or raise MissingExtraError naming the extra."""
    try:
        return importlib.import_module(f".{name}", __name__)
    except ImportError as exc:
        extra, what = _EXTRAS[name]
        raise MissingExtraError(
            f"this env needs {what}, which the base saltmarsh-arena install leaves out "
            f'({exc}). Install it with: pip install "saltmarsh-arena[{extra}]" '
            f'(or "saltmarsh-arena[all]")'
        ) from exc


def parallel_env(
    game: str,
    builds: Sequence[Build] | None = None,
    learning: Sequence[int] | None = None,
    frame_skip: int = 4,
    render_mode: str | None = None,
) -> ArenaParallelEnv:
    """A PettingZoo ``ParallelEnv`` for ``game`` (see ``ArenaParallelEnv``). Needs the
    ``pettingzoo`` extra."""
    cls = _env_module("_pettingzoo").ArenaParallelEnv
    return cast("ArenaParallelEnv", cls(game, builds, learning, frame_skip, render_mode))


def gym_env(
    game: str,
    builds: Sequence[Build] | None = None,
    agent: int = 0,
    frame_skip: int = 4,
    render_mode: str | None = None,
) -> ArenaGymEnv:
    """A Gymnasium ``Env`` for ``game`` with one learning agent (see ``ArenaGymEnv``).
    Needs the ``gym`` extra."""
    cls = _env_module("_gymnasium").ArenaGymEnv
    return cast("ArenaGymEnv", cls(game, builds, agent, frame_skip, render_mode))


def __getattr__(name: str) -> Any:
    """``ArenaParallelEnv`` and ``ArenaGymEnv``, imported on first use."""
    if name == "ArenaParallelEnv":
        return _env_module("_pettingzoo").ArenaParallelEnv
    if name == "ArenaGymEnv":
        return _env_module("_gymnasium").ArenaGymEnv
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
