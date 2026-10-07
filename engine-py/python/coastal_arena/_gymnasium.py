"""The Gymnasium env (extra ``coastal-arena[gym]``); see ``_episode.py``."""

from __future__ import annotations

from collections.abc import Sequence
from typing import Any

import gymnasium
import numpy as np

from ._episode import SEED_LIMIT, done, end_info, flat_spaces
from .flat import Build, FlatEnv


class ArenaGymEnv(gymnasium.Env[np.ndarray, np.ndarray]):
    """A Gymnasium ``Env``: agent ``agent`` learns, every other agent plays its
    build's scripted behavior in Rust. The episode ends when the match does or when
    the agent leaves play (the match is then cut there)."""

    metadata = {"render_modes": []}

    def __init__(
        self,
        game: str,
        builds: Sequence[Build] | None = None,
        agent: int = 0,
        frame_skip: int = 4,
        render_mode: str | None = None,
    ) -> None:
        if render_mode is not None:
            raise ValueError("no render modes; watch the replay in the viewer")
        self.flat = FlatEnv(game, builds, [agent], frame_skip)
        self.observation_space, self.action_space = flat_spaces(self.flat)
        self.agent = self.flat.agent_names[agent]

    def reset(
        self, *, seed: int | None = None, options: dict[str, Any] | None = None
    ) -> tuple[np.ndarray, dict[str, Any]]:
        super().reset(seed=seed)  # ``options`` is ignored, as in ArenaParallelEnv
        if seed is None:
            seed = int(self.np_random.integers(0, SEED_LIMIT))
        f = self.flat
        f.reset(seed)
        return f.obs[0].copy(), {"seed": seed, "tick": f.tick, "setup_hash": f.setup_hash()}

    def step(self, action: Any) -> tuple[np.ndarray, float, bool, bool, dict[str, Any]]:
        f = self.flat
        f.actions[0] = action
        over = f.step()
        end = end_info(f) if over else None
        terminated, truncated = done(f, 0, over, end["reason"] if end else None)
        info: dict[str, Any] = {"tick": f.tick, **(end or {})}
        if (terminated or truncated) and end is None:
            info["final_hash"] = f.state_hash()
        return f.obs[0].copy(), float(f.rewards[0]), terminated, truncated, info

    def replay_json(self) -> str:
        """The current match as arena replay JSON."""
        return self.flat.replay_json()
