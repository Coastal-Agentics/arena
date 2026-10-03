"""The PettingZoo env (extra ``saltmarsh-arena[pettingzoo]``); see ``_episode.py``."""

from __future__ import annotations

from collections.abc import Sequence
from typing import Any

import numpy as np
from gymnasium import spaces
from pettingzoo import ParallelEnv  # type: ignore[import-untyped]

from ._episode import SEED_LIMIT, done, end_info, flat_spaces
from .flat import Build, FlatEnv


class ArenaParallelEnv(ParallelEnv):  # type: ignore[misc]
    """A PettingZoo ``ParallelEnv``: every learning agent acts each step.

    ``builds[i]`` is agent ``i``'s build (default: the game's default build for
    every agent, two tanks or four cars). ``learning`` picks the agents you control
    (default: all); the rest play their build's scripted behavior in Rust and are not
    PettingZoo agents.
    """

    metadata = {"name": "saltmarsh_arena_v0", "render_modes": [], "is_parallelizable": True}

    def __init__(
        self,
        game: str,
        builds: Sequence[Build] | None = None,
        learning: Sequence[int] | None = None,
        frame_skip: int = 4,
        render_mode: str | None = None,
    ) -> None:
        if render_mode is not None:
            raise ValueError("no render modes; watch the replay in the viewer")
        self.render_mode = None
        self.flat = FlatEnv(game, builds, learning, frame_skip)
        self.metadata = {**self.metadata, "name": f"saltmarsh_arena_{game}_v0"}
        names = self.flat.agent_names
        self.possible_agents = [names[a] for a in self.flat.learning]
        self.agents: list[str] = []
        self._row = {a: k for k, a in enumerate(self.possible_agents)}
        self._obs_space, self._action_space = flat_spaces(self.flat)
        self._seeds = np.random.default_rng()

    def observation_space(self, agent: str) -> spaces.Box:
        return self._obs_space

    def action_space(self, agent: str) -> spaces.Box:
        return self._action_space

    def reset(
        self, seed: int | None = None, options: dict[str, Any] | None = None
    ) -> tuple[dict[str, np.ndarray], dict[str, dict[str, Any]]]:
        # ``options`` is accepted and ignored: builds, learning agents and frame_skip
        # are fixed at construction (one config per env).
        if seed is None:
            seed = int(self._seeds.integers(0, SEED_LIMIT))
        f = self.flat
        f.reset(seed)
        self.agents = [a for a in self.possible_agents if f.active[self._row[a]]]
        info = {"seed": seed, "tick": f.tick, "setup_hash": f.setup_hash()}
        obs = {a: f.obs[self._row[a]].copy() for a in self.agents}
        return obs, {a: dict(info) for a in self.agents}

    def step(self, actions: dict[str, Any]) -> tuple[
        dict[str, np.ndarray],
        dict[str, float],
        dict[str, bool],
        dict[str, bool],
        dict[str, dict[str, Any]],
    ]:
        f = self.flat
        f.actions[...] = 0.0
        for a, act in actions.items():
            k = self._row.get(a)
            if k is not None:
                f.actions[k] = act
        over = f.step()
        end = end_info(f) if over else None
        reason = end["reason"] if end else None
        obs, rewards, terms, truncs, infos = {}, {}, {}, {}, {}
        for a in self.agents:
            k = self._row[a]
            obs[a] = f.obs[k].copy()
            rewards[a] = float(f.rewards[k])
            terms[a], truncs[a] = done(f, k, over, reason)
            infos[a] = {"tick": f.tick, **(end or {})}
        self.agents = [a for a in self.agents if not (terms[a] or truncs[a])]
        return obs, rewards, terms, truncs, infos

    def render(self) -> None:
        return None

    def close(self) -> None:
        return None

    def replay_json(self) -> str:
        """The current match as arena replay JSON."""
        return self.flat.replay_json()
