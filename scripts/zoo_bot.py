#!/usr/bin/env python3
from __future__ import annotations

import argparse
import random
import sys
from dataclasses import dataclass
from typing import Callable

from mushroom_1 import (
    COLS,
    FIRST_PLAYER,
    Move,
    MushroomState,
    MinimaxConfig,
    MinimaxSearch,
    PASS_MOVE,
    ROWS,
    SECOND_PLAYER,
    choose_greedy_action,
    opponent,
)


def _is_edge(move: Move) -> bool:
    return move[0] == 0 or move[2] == ROWS - 1 or move[1] == 0 or move[3] == COLS - 1


def _is_corner(move: Move) -> bool:
    return (
        (move[0] == 0 and move[1] == 0)
        or (move[0] == 0 and move[3] == COLS - 1)
        or (move[2] == ROWS - 1 and move[1] == 0)
        or (move[2] == ROWS - 1 and move[3] == COLS - 1)
    )


def _move_rank(state: MushroomState, move: Move) -> tuple[int, int, int, int, int, int, int, int]:
    score_delta, recaptured, fresh, live, net_area, area = state.action_score(move)
    return (
        score_delta,
        recaptured,
        fresh,
        live,
        net_area,
        area,
        -move[0],
        -move[1] - move[2] - move[3],
    )


def _area_rank(state: MushroomState, move: Move) -> tuple[int, int, int, int, int, int, int, int]:
    score_delta, recaptured, fresh, live, net_area, area = state.action_score(move)
    return (
        area,
        score_delta,
        recaptured,
        fresh,
        live,
        net_area,
        -move[0],
        -move[1] - move[2] - move[3],
    )


def _recapture_rank(state: MushroomState, move: Move) -> tuple[int, int, int, int, int, int, int, int]:
    score_delta, recaptured, fresh, live, net_area, area = state.action_score(move)
    return (
        recaptured,
        score_delta,
        fresh,
        live,
        net_area,
        area,
        -move[0],
        -move[1] - move[2] - move[3],
    )


def _fresh_rank(state: MushroomState, move: Move) -> tuple[int, int, int, int, int, int, int, int]:
    score_delta, recaptured, fresh, live, net_area, area = state.action_score(move)
    return (
        fresh,
        score_delta,
        recaptured,
        live,
        net_area,
        area,
        -move[0],
        -move[1] - move[2] - move[3],
    )


def _edge_rank(state: MushroomState, move: Move) -> tuple[int, int, int, int, int, int, int, int]:
    score_delta, recaptured, fresh, live, net_area, area = state.action_score(move)
    edge_bonus = int(_is_edge(move))
    corner_bonus = int(_is_corner(move))
    return (
        corner_bonus,
        edge_bonus,
        score_delta,
        recaptured,
        fresh,
        live,
        net_area,
        area,
    )


def _corner_rank(state: MushroomState, move: Move) -> tuple[int, int, int, int, int, int, int, int]:
    score_delta, recaptured, fresh, live, net_area, area = state.action_score(move)
    corner_bonus = int(_is_corner(move))
    edge_bonus = int(_is_edge(move))
    return (
        corner_bonus,
        edge_bonus,
        score_delta,
        recaptured,
        fresh,
        live,
        net_area,
        area,
    )


def _reply_pressure(state: MushroomState, move: Move) -> tuple[float, int]:
    next_state = state.apply_action(move)
    replies = next_state.legal_rectangles()
    if not replies:
        return 0.0, 0
    return max(next_state.action_score(reply)[0] for reply in replies), len(replies)


def _reply_aware_score(state: MushroomState, move: Move, reply_weight: float = 0.80) -> tuple[float, int, int, int, int, int, int]:
    immediate = state.action_score(move)
    reply_score, reply_count = _reply_pressure(state, move)
    mobility_penalty = min(reply_count, 16) * 0.03
    return (
        immediate[0] - reply_weight * reply_score - mobility_penalty,
        *immediate[1:],
    )


def _reply_aware_strict_score(state: MushroomState, move: Move) -> tuple[float, int, int, int, int, int, int]:
    immediate = state.action_score(move)
    reply_score, reply_count = _reply_pressure(state, move)
    mobility_penalty = min(reply_count, 20) * 0.05
    return (
        immediate[0] - 1.10 * reply_score - mobility_penalty,
        *immediate[1:],
    )


def _defensive_score(state: MushroomState, move: Move) -> tuple[float, int, int, int, int, int, int]:
    lead = state.score(state.player) - state.score(opponent(state.player))
    immediate = state.action_score(move)
    reply_score, reply_count = _reply_pressure(state, move)
    if lead > 0:
        weight = 0.90 + min(lead, 12) * 0.06
        area_penalty = min(immediate[5], 30) * 0.10
    else:
        weight = 0.70
        area_penalty = min(immediate[5], 30) * 0.04
    mobility_penalty = min(reply_count, 18) * 0.03
    return (
        immediate[0] - weight * reply_score - mobility_penalty - area_penalty,
        *immediate[1:],
    )


def _defensive_when_losing_score(state: MushroomState, move: Move) -> tuple[float, int, int, int, int, int, int]:
    lead = state.score(state.player) - state.score(opponent(state.player))
    immediate = state.action_score(move)
    reply_score, reply_count = _reply_pressure(state, move)
    if lead > 0:
        weight = 0.78
        area_penalty = min(immediate[5], 30) * 0.03
    else:
        weight = 0.98 + min(-lead, 12) * 0.04
        area_penalty = min(immediate[5], 30) * 0.12
    mobility_penalty = min(reply_count, 18) * 0.04
    return (
        immediate[0] - weight * reply_score - mobility_penalty - area_penalty,
        *immediate[1:],
    )


def _pass_abuser_choice(state: MushroomState) -> Move:
    rectangles = state.legal_rectangles()
    if not rectangles:
        return PASS_MOVE

    lead = state.score(state.player) - state.score(opponent(state.player))
    live_count = sum(1 for row in state.values for value in row if value > 0)
    if lead >= 4 and len(rectangles) <= 6:
        return PASS_MOVE
    if live_count <= 12 or len(rectangles) <= 2:
        return PASS_MOVE
    return max(rectangles, key=lambda move: _move_rank(state, move))


def _pass_safe_choice(state: MushroomState) -> Move:
    rectangles = state.legal_rectangles()
    if not rectangles:
        return PASS_MOVE
    lead = state.score(state.player) - state.score(opponent(state.player))
    live_count = sum(1 for row in state.values for value in row if value > 0)
    if lead >= 8 and live_count <= 18:
        return PASS_MOVE
    ranked = sorted(rectangles, key=lambda move: _reply_aware_score(state, move), reverse=True)
    return ranked[0] if ranked else PASS_MOVE


def _random_top_choice(state: MushroomState, top_k: int, rng: random.Random) -> Move:
    rectangles = state.legal_rectangles()
    if not rectangles:
        return PASS_MOVE
    ranked = sorted(rectangles, key=lambda move: _move_rank(state, move), reverse=True)
    top_k = max(1, min(top_k, len(ranked)))
    return rng.choice(ranked[:top_k])


def _minimax_choice(state: MushroomState, depth: int, budget_ms: int) -> Move:
    search = MinimaxSearch(
        MinimaxConfig(
            max_depth=depth,
            time_budget_ms=max(1, budget_ms),
            use_alpha_beta=True,
            use_transposition_table=True,
        )
    )
    result = search.search(state)
    return result.action if result.action is not None else PASS_MOVE


def _greedy_balanced_choice(state: MushroomState) -> Move:
    rectangles = state.legal_rectangles()
    if not rectangles:
        return PASS_MOVE
    return max(rectangles, key=lambda move: _move_rank(state, move))


def _mixed_tactical_choice(state: MushroomState, budget_ms: int, rng: random.Random) -> Move:
    rectangles = state.legal_rectangles()
    if not rectangles:
        return PASS_MOVE
    if len(rectangles) <= 3:
        return max(rectangles, key=lambda move: _recapture_rank(state, move))
    roll = rng.random()
    if roll < 0.35:
        return max(rectangles, key=lambda move: _reply_aware_score(state, move))
    if roll < 0.70:
        return max(rectangles, key=lambda move: _defensive_score(state, move))
    return _minimax_choice(state, 2, budget_ms)


def _live_count(state: MushroomState) -> int:
    return sum(1 for row in state.values for value in row if value > 0)


@dataclass
class ZooContext:
    mode: str
    rng: random.Random

    def choose(self, state: MushroomState, budget_ms: int) -> Move:
        rectangles = state.legal_rectangles()
        if not rectangles:
            return PASS_MOVE

        mode = self.mode
        if mode == "greedy_area":
            return max(rectangles, key=lambda move: _area_rank(state, move))
        if mode == "greedy_recapture":
            return max(rectangles, key=lambda move: _recapture_rank(state, move))
        if mode == "greedy_fresh":
            return max(rectangles, key=lambda move: _fresh_rank(state, move))
        if mode == "greedy_edge":
            return max(rectangles, key=lambda move: _edge_rank(state, move))
        if mode == "greedy_corner":
            return max(rectangles, key=lambda move: _corner_rank(state, move))
        if mode == "reply_aware":
            return max(rectangles, key=lambda move: _reply_aware_score(state, move))
        if mode == "reply_aware_strict":
            return max(rectangles, key=lambda move: _reply_aware_strict_score(state, move))
        if mode == "defensive_when_leading":
            return max(rectangles, key=lambda move: _defensive_score(state, move))
        if mode == "defensive_when_losing":
            return max(rectangles, key=lambda move: _defensive_when_losing_score(state, move))
        if mode == "pass_abuser":
            return _pass_abuser_choice(state)
        if mode == "pass_safe":
            return _pass_safe_choice(state)
        if mode == "random_top_3":
            return _random_top_choice(state, 3, self.rng)
        if mode == "random_top_5":
            return _random_top_choice(state, 5, self.rng)
        if mode == "random_top_7":
            return _random_top_choice(state, 7, self.rng)
        if mode == "minimax_depth_1":
            return _minimax_choice(state, 1, budget_ms)
        if mode == "minimax_depth_2":
            return _minimax_choice(state, 2, budget_ms)
        if mode == "minimax_depth_3":
            return _minimax_choice(state, 3, budget_ms)
        if mode == "minimax_depth_4":
            return _minimax_choice(state, 4, budget_ms)
        if mode == "greedy_balanced":
            return _greedy_balanced_choice(state)
        if mode == "mixed_tactical":
            return _mixed_tactical_choice(state, budget_ms, self.rng)
        if mode == "endgame_expert":
            if _live_count(state) <= 20:
                return _minimax_choice(state, 4, budget_ms)
            return _random_top_choice(state, 5, self.rng)
        return choose_greedy_action(state)


class ZooProtocolBot:
    def __init__(self, mode: str, seed: int) -> None:
        self.mode = mode
        self.rng = random.Random(seed)
        self.state: MushroomState | None = None
        self.my_player = FIRST_PLAYER
        self.my_time_left_ms: int | None = None
        self.opp_time_left_ms: int | None = None

    def handle_command(self, line: str, input_stream) -> str | None:
        parts = line.strip().split()
        if not parts:
            return None

        cmd = parts[0]
        if cmd == "READY":
            self.my_player = FIRST_PLAYER if len(parts) < 2 or parts[1] == "FIRST" else SECOND_PLAYER
            return "OK"
        if cmd == "INIT":
            rows = list(parts[1:])
            while len(rows) < ROWS:
                next_line = input_stream.readline()
                if not next_line:
                    break
                rows.extend(next_line.strip().split())
            board_rows = rows[:ROWS]
            self.state = MushroomState.from_rows(board_rows)
            self.rng.seed(_stable_seed(self.mode, board_rows, 42))
            return None
        if cmd == "TIME":
            if self.state is None or self.state.is_terminal():
                return _format_move(PASS_MOVE)
            self.my_time_left_ms = _parse_int(parts, 1)
            self.opp_time_left_ms = _parse_int(parts, 2)
            budget_ms = self._budget_ms()
            move = ZooContext(self.mode, self.rng).choose(self.state, budget_ms)
            if not self.state.is_legal_action(move):
                move = PASS_MOVE
            self.state = self.state.apply_action(move)
            return _format_move(move)
        if cmd == "OPP":
            if self.state is not None and not self.state.is_terminal() and len(parts) >= 5:
                try:
                    move = tuple(int(value) for value in parts[1:5])  # type: ignore[assignment]
                except ValueError:
                    return None
                if self.state.is_legal_action(move):
                    self.state = self.state.apply_action(move)
            return None
        if cmd == "FINISH":
            raise SystemExit(0)
        return None

    def _budget_ms(self) -> int:
        if self.my_time_left_ms is None:
            return 25
        usable = max(0, self.my_time_left_ms - 300)
        if self.mode.startswith("minimax"):
            return max(200, min(500, usable // 5))
        return max(5, min(60, usable // 20))


def _format_move(move: Move) -> str:
    return f"{move[0]} {move[1]} {move[2]} {move[3]}"


def _parse_int(parts: list[str], index: int) -> int | None:
    if len(parts) <= index:
        return None
    try:
        return int(parts[index])
    except ValueError:
        return None


def _stable_seed(mode: str, rows: list[str], base_seed: int) -> int:
    seed = base_seed & 0xFFFFFFFF
    for ch in mode:
        seed = (seed * 131 + ord(ch)) & 0xFFFFFFFF
    for row in rows:
        for ch in row:
            seed = (seed * 131 + ord(ch)) & 0xFFFFFFFF
    return seed


def main() -> None:
    parser = argparse.ArgumentParser(description="Mushroom zoo bot presets")
    parser.add_argument(
        "--mode",
        default="greedy_area",
        choices=[
            "greedy_area",
            "greedy_recapture",
            "greedy_fresh",
            "greedy_edge",
            "greedy_corner",
            "reply_aware",
            "reply_aware_strict",
            "defensive_when_leading",
            "defensive_when_losing",
            "pass_abuser",
            "pass_safe",
            "random_top_3",
            "random_top_5",
            "random_top_7",
            "minimax_depth_1",
            "minimax_depth_2",
            "minimax_depth_3",
            "minimax_depth_4",
            "greedy_balanced",
            "mixed_tactical",
            "endgame_expert",
        ],
    )
    parser.add_argument("--seed", type=int, default=42)
    args = parser.parse_args()

    bot = ZooProtocolBot(args.mode, args.seed)
    input_stream = sys.stdin
    output_stream = sys.stdout

    for line in input_stream:
        response = bot.handle_command(line, input_stream)
        if response is not None:
            output_stream.write(response + "\n")
            output_stream.flush()


if __name__ == "__main__":
    main()
