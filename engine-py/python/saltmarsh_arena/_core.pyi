"""Type stubs for the Rust module ``saltmarsh_arena._core`` (``engine-py/src/python.rs``)."""

from __future__ import annotations

from typing import final

from typing_extensions import Self

__all__ = [
    "Arena",
    "catalog_json",
    "default_build_json",
    "engine_version",
    "games",
    "reference_replay",
    "validate_build_json",
    "verify_replay",
]

@final
class Arena:
    """One match of a game for a training tool.

    Owns four ``bytearray`` buffers (made once; view them with ``np.frombuffer``):
    observations (``rows * obs_len`` float32), actions (``rows * action_len`` float32),
    rewards (``rows`` float32) and active flags (``rows`` uint8). Rows follow
    ``learning``. ``step()`` reads the action buffer and rewrites the others.
    """

    def __new__(
        cls,
        game: str,
        builds: list[str],
        learning: list[int],
        frame_skip: int = 4,
        seed: int = 0,
    ) -> Self:
        """``builds[i]`` (JSON) is agent ``i``'s build, checked by the game's catalog.
        Agents not in ``learning`` play their build's scripted behavior. Raises
        ``ValueError`` for an unknown game, a bad build or build count, a bad
        ``learning`` list, a champion build on a scripted agent, or ``frame_skip < 1``."""
    def reset(self, seed: int) -> None:
        """Start a new match at ``seed`` (0 to 2**64 - 1)."""
    def step(self) -> bool:
        """Up to ``frame_skip`` ticks with the action buffer's actions. Returns whether
        the match is over (a no-op with zero rewards once it is)."""
    @property
    def game(self) -> str: ...
    @property
    def agent_names(self) -> list[str]:
        """Every agent's name by index: tank ``blue_0``, ``orange_1``; racing ``car_0`` … ``car_3``."""
    @property
    def learning(self) -> list[int]: ...
    @property
    def obs_len(self) -> int: ...
    @property
    def action_len(self) -> int: ...
    @property
    def frame_skip(self) -> int: ...
    @property
    def obs_buffer(self) -> bytearray: ...
    @property
    def action_buffer(self) -> bytearray: ...
    @property
    def reward_buffer(self) -> bytearray: ...
    @property
    def active_buffer(self) -> bytearray: ...
    @property
    def tick(self) -> int: ...
    @property
    def seed(self) -> int: ...
    @property
    def is_over(self) -> bool: ...
    def outcome(self) -> tuple[int | None, int, str] | None:
        """``(winner_team, ticks, reason)`` once over; reason is ``last_standing``,
        ``all_destroyed``, ``tick_limit`` or ``finished``."""
    def state_hash(self) -> str:
        """16-digit hex, the replay's ``final_hash`` format."""
    def setup_hash(self) -> str: ...
    def replay_json(self) -> str: ...

def games() -> list[tuple[str, int, int, int, int, int]]:
    """``(id, rules_version, obs_len, action_len, min_agents, max_agents)`` per game."""

def catalog_json(game: str) -> str: ...
def default_build_json(game: str) -> str: ...
def validate_build_json(game: str, build: str) -> str: ...
def verify_replay(game: str, json: str) -> str:
    """``Replay::verify`` natively; returns the ``final_hash``. Raises ``ValueError``."""

def reference_replay(
    game: str,
    builds: list[str],
    learning: list[int],
    frame_skip: int,
    seed: int,
    max_steps: int | None = None,
) -> str:
    """The determinism tests' native episode as replay JSON (``engine_py::reference_episode``)."""

def engine_version() -> str: ...
