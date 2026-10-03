"""PettingZoo and Gymnasium envs over :class:`~saltmarsh_arena.FlatEnv`
(``docs/design/game-system.md`` §6; GATE-003 §6).

- Spaces: ``Box(-1, 1, (obs_len,), float32)`` and ``Box(-1, 1, (action_len,),
  float32)``. Tank: 176 and 4 (fire when ``action[3] > 0``). Racing: 43 and 2.
- One ``step`` is ``frame_skip`` ticks with the same actions, run in Rust, stopping
  early at the end. The reward is summed over those ticks.
- An agent *terminates* when the match ends for any reason but the tick limit, or
  when it leaves play (a tank destroyed, a car finished). It is *truncated* when the
  match hits the tick limit. Either way it drops out of ``agents``.
- ``reset(seed, options)``: ``options`` is ignored; builds, learning agents and
  ``frame_skip`` are fixed when the env is made.
- ``infos``: ``seed``, ``tick`` and ``setup_hash`` at reset; ``tick`` on every step;
  and at the end ``final_hash``, ``ticks``, ``reason``, ``winner`` (an agent name or
  ``None``) and ``winner_team``.

Observations are copies, so they stay valid after the next step. Use
:class:`~saltmarsh_arena.FlatEnv` for the zero-copy path.
"""

from __future__ import annotations

from collections.abc import Sequence
from typing import Any

import gymnasium
import numpy as np
from gymnasium import spaces
from pettingzoo import ParallelEnv  # type: ignore[import-untyped]

from .flat import Build, FlatEnv

_SEED_LIMIT = 2**63


def _spaces(flat: FlatEnv) -> tuple[spaces.Box, spaces.Box]:
    c = flat.core
    return (
        spaces.Box(-1.0, 1.0, (c.obs_len,), np.float32),
        spaces.Box(-1.0, 1.0, (c.action_len,), np.float32),
    )


def _end_info(flat: FlatEnv) -> dict[str, Any]:
    o = flat.outcome()
    assert o is not None
    team = o["winner"]
    return {
        "final_hash": flat.state_hash(),
        "ticks": o["ticks"],
        "reason": o["reason"],
        # Each agent is its own team in both games (tank duel, racing).
        "winner": None if team is None else flat.agent_names[team],
        "winner_team": team,
    }


def _done(flat: FlatEnv, row: int, over: bool, reason: str | None) -> tuple[bool, bool]:
    """(terminated, truncated) for a row after a step."""
    terminated = (over and reason != "tick_limit") or not bool(flat.active[row])
    truncated = over and reason == "tick_limit" and not terminated
    return terminated, truncated


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
        self._obs_space, self._action_space = _spaces(self.flat)
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
            seed = int(self._seeds.integers(0, _SEED_LIMIT))
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
        end = _end_info(f) if over else None
        reason = end["reason"] if end else None
        obs, rewards, terms, truncs, infos = {}, {}, {}, {}, {}
        for a in self.agents:
            k = self._row[a]
            obs[a] = f.obs[k].copy()
            rewards[a] = float(f.rewards[k])
            terms[a], truncs[a] = _done(f, k, over, reason)
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
        self.observation_space, self.action_space = _spaces(self.flat)
        self.agent = self.flat.agent_names[agent]

    def reset(
        self, *, seed: int | None = None, options: dict[str, Any] | None = None
    ) -> tuple[np.ndarray, dict[str, Any]]:
        super().reset(seed=seed)  # ``options`` is ignored, as in ArenaParallelEnv
        if seed is None:
            seed = int(self.np_random.integers(0, _SEED_LIMIT))
        f = self.flat
        f.reset(seed)
        return f.obs[0].copy(), {"seed": seed, "tick": f.tick, "setup_hash": f.setup_hash()}

    def step(self, action: Any) -> tuple[np.ndarray, float, bool, bool, dict[str, Any]]:
        f = self.flat
        f.actions[0] = action
        over = f.step()
        end = _end_info(f) if over else None
        terminated, truncated = _done(f, 0, over, end["reason"] if end else None)
        info: dict[str, Any] = {"tick": f.tick, **(end or {})}
        if (terminated or truncated) and end is None:
            info["final_hash"] = f.state_hash()
        return f.obs[0].copy(), float(f.rewards[0]), terminated, truncated, info

    def replay_json(self) -> str:
        """The current match as arena replay JSON."""
        return self.flat.replay_json()


def parallel_env(
    game: str,
    builds: Sequence[Build] | None = None,
    learning: Sequence[int] | None = None,
    frame_skip: int = 4,
    render_mode: str | None = None,
) -> ArenaParallelEnv:
    """A PettingZoo ``ParallelEnv`` for ``game`` (see :class:`ArenaParallelEnv`)."""
    return ArenaParallelEnv(game, builds, learning, frame_skip, render_mode)


def gym_env(
    game: str,
    builds: Sequence[Build] | None = None,
    agent: int = 0,
    frame_skip: int = 4,
    render_mode: str | None = None,
) -> ArenaGymEnv:
    """A Gymnasium ``Env`` for ``game`` with one learning agent (see
    :class:`ArenaGymEnv`)."""
    return ArenaGymEnv(game, builds, agent, frame_skip, render_mode)
