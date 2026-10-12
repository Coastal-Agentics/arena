"""``FlatEnv``: numpy views over the Rust-owned buffers of one match."""

from __future__ import annotations

import json
from collections.abc import Sequence
from typing import Any, Union

import numpy as np

from . import _core

Build = Union[str, dict[str, Any]]
"""A build: its JSON text, or the parsed dict (``{"rules_version", "levels",
"behavior"}``)."""


def build_texts(game: str, builds: Sequence[Build] | None, count: int | None = None) -> list[str]:
    """Builds as JSON text; ``None`` means the default build for every agent (tank:
    2, racing: 4, or ``count``)."""
    if builds is None:
        info = next((g for g in _core.games() if g[0] == game), None)
        if info is None:
            raise ValueError(f"unknown game {game!r}")
        n = count if count is not None else info[5]
        return [_core.default_build_json(game)] * n
    return [b if isinstance(b, str) else json.dumps(b) for b in builds]


class FlatEnv:
    """One match for a training loop, without dicts or copies.

    ``builds[i]`` is agent ``i``'s build, checked by the game's catalog. ``learning``
    lists the agents whose actions you send (default: all); every other agent plays
    its build's scripted behavior in Rust. Rows of the arrays below follow
    ``learning``.

    - ``obs``: ``float32[rows, obs_len]``, written by :meth:`reset` and :meth:`step`.
    - ``actions``: ``float32[rows, action_len]``; write into it, then :meth:`step`.
    - ``rewards``: ``float32[rows]``, each row's reward summed over the step's ticks.
    - ``active``: ``bool[rows]``, whether the row's agent is still in play.

    The arrays are views of buffers the Rust side reuses, so they change on every
    step; copy what you keep.
    """

    def __init__(
        self,
        game: str,
        builds: Sequence[Build] | None = None,
        learning: Sequence[int] | None = None,
        frame_skip: int = 4,
        seed: int = 0,
    ) -> None:
        texts = build_texts(game, builds)
        rows = list(range(len(texts))) if learning is None else [int(a) for a in learning]
        self.core = _core.Arena(game, texts, rows, frame_skip, seed)
        c = self.core
        n = len(rows)
        self.obs = np.frombuffer(c.obs_buffer, dtype=np.float32).reshape(n, c.obs_len)
        self.actions = np.frombuffer(c.action_buffer, dtype=np.float32).reshape(n, c.action_len)
        self.rewards = np.frombuffer(c.reward_buffer, dtype=np.float32)
        self._active = np.frombuffer(c.active_buffer, dtype=np.uint8)
        self.obs.flags.writeable = False
        self.rewards.flags.writeable = False

    @property
    def active(self) -> np.ndarray:
        """``bool[rows]``: whether each row's agent is still in play."""
        return self._active.view(np.bool_)

    @property
    def game(self) -> str:
        return self.core.game

    @property
    def agent_names(self) -> list[str]:
        """Every agent's name by index (``blue_0``, ``orange_1``; ``car_0`` …)."""
        return self.core.agent_names

    @property
    def learning(self) -> list[int]:
        return self.core.learning

    @property
    def tick(self) -> int:
        return self.core.tick

    @property
    def seed(self) -> int:
        return self.core.seed

    @property
    def is_over(self) -> bool:
        return self.core.is_over

    def reset(self, seed: int) -> np.ndarray:
        """Start a new match at ``seed``; returns ``obs``."""
        self.core.reset(seed)
        return self.obs

    def step(self, actions: np.ndarray | None = None) -> bool:
        """Step with ``actions`` (copied into :attr:`actions` first, if given) for up
        to ``frame_skip`` ticks. Returns whether the match is over."""
        if actions is not None:
            self.actions[...] = actions
        return self.core.step()

    def outcome(self) -> dict[str, Any] | None:
        """``None`` until the end, then ``{"winner": team | None, "ticks", "reason"}``."""
        o = self.core.outcome()
        if o is None:
            return None
        return {"winner": o[0], "ticks": o[1], "reason": o[2]}

    def state_hash(self) -> str:
        """The state hash now, as the replay's 16-digit hex ``final_hash``."""
        return self.core.state_hash()

    def setup_hash(self) -> str:
        return self.core.setup_hash()

    def replay_json(self) -> str:
        """The match so far as arena replay JSON."""
        return self.core.replay_json()
