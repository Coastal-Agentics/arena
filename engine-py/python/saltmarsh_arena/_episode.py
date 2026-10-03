"""Shared by the PettingZoo and Gymnasium envs over :class:`~saltmarsh_arena.FlatEnv`
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

from typing import TYPE_CHECKING, Any

from .flat import FlatEnv

if TYPE_CHECKING:
    from gymnasium import spaces

SEED_LIMIT = 2**63
"""``reset(seed=None)`` draws a seed below this."""


def flat_spaces(flat: FlatEnv) -> tuple[spaces.Box, spaces.Box]:
    """``Box(-1, 1, (obs_len,))`` and ``Box(-1, 1, (action_len,))``, float32."""
    import numpy as np
    from gymnasium import spaces

    c = flat.core
    return (
        spaces.Box(-1.0, 1.0, (c.obs_len,), np.float32),
        spaces.Box(-1.0, 1.0, (c.action_len,), np.float32),
    )


def end_info(flat: FlatEnv) -> dict[str, Any]:
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


def done(flat: FlatEnv, row: int, over: bool, reason: str | None) -> tuple[bool, bool]:
    """(terminated, truncated) for a row after a step."""
    terminated = (over and reason != "tick_limit") or not bool(flat.active[row])
    truncated = over and reason == "tick_limit" and not terminated
    return terminated, truncated
