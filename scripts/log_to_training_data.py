from __future__ import annotations

import argparse
import csv
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterator

import mushroom_1


ROWS = 10
COLS = 17
RECT_COUNT = ROWS * (ROWS + 1) // 2 * COLS * (COLS + 1) // 2
PASS_MOVE = (-1, -1, -1, -1)


def fixed_rect_id(r1: int, c1: int, r2: int, c2: int) -> int:
    r_term = r1 * (r1 - 1) // 2 if r1 > 0 else 0
    r_index = r1 * ROWS - r_term + (r2 - r1)

    c_term = c1 * (c1 - 1) // 2 if c1 > 0 else 0
    c_index = c1 * COLS - c_term + (c2 - c1)

    cols_pairs = COLS * (COLS + 1) // 2
    return r_index * cols_pairs + c_index


def phase_for_live_count(live_count: int) -> int:
    if live_count <= 12:
        return 2
    if live_count >= 25:
        return 0
    return 1


def phase_for_state(live_count: int, legal_moves: int) -> int:
    if live_count <= 12 or legal_moves <= 10:
        return 2
    if live_count <= 25 or legal_moves <= 24:
        return 1
    return 0


def score_bucket(score: int) -> int:
    return max(0, min(7, (score // 150) + 4))


def corner_edge_bonus(move: tuple[int, int, int, int]) -> int:
    if move == PASS_MOVE:
        return 0
    r1, c1, r2, c2 = move
    bonus = 0
    touches_corner = (
        (r1 == 0 and c1 == 0)
        or (r1 == 0 and c2 == COLS - 1)
        or (r2 == ROWS - 1 and c1 == 0)
        or (r2 == ROWS - 1 and c2 == COLS - 1)
    )
    if touches_corner:
        bonus += 15
    touches_edge = r1 == 0 or r2 == ROWS - 1 or c1 == 0 or c2 == COLS - 1
    if touches_edge:
        bonus += 5
    return bonus


def static_move_score(state: mushroom_1.MushroomState, move: tuple[int, int, int, int]) -> int:
    score_delta, recaptured, fresh, live, net_area, area = state.action_score(move)
    own = area - net_area
    edge = 0
    corner = 0
    if move != PASS_MOVE:
        r1, c1, r2, c2 = move
        for r in range(r1, r2 + 1):
            for c in range(c1, c2 + 1):
                is_corner = (
                    (r == 0 and c == 0)
                    or (r == 0 and c == COLS - 1)
                    or (r == ROWS - 1 and c == 0)
                    or (r == ROWS - 1 and c == COLS - 1)
                )
                if is_corner:
                    corner += 1
                elif r == 0 or r == ROWS - 1 or c == 0 or c == COLS - 1:
                    edge += 1

    _ = score_delta
    return recaptured * 120 + fresh * 45 + live * 12 + area * 6 + edge * 8 + corner * 15 - own * 20


def move_value(state: mushroom_1.MushroomState, move: tuple[int, int, int, int]) -> float:
    return float(static_move_score(state, move))


@dataclass
class MoveRow:
    game_id: int
    ply: int
    mover: int
    rect_id: int
    phase: int
    bucket: int
    move_value: float
    outcome: float = 0.0
    margin: int = 0


@dataclass
class GameBuffer:
    game_id: int
    state: mushroom_1.MushroomState
    rows: list[MoveRow]
    first_score: int | None = None
    second_score: int | None = None

    def finalize_scores(self) -> bool:
        if self.first_score is None and self.second_score is None:
            self.first_score = self.state.score(mushroom_1.FIRST_PLAYER)
            self.second_score = self.state.score(mushroom_1.SECOND_PLAYER)
        if self.first_score is None or self.second_score is None:
            return False

        margin = self.first_score - self.second_score
        for row in self.rows:
            row.margin = margin
            if margin == 0:
                row.outcome = 0.5
            else:
                mover_wins = (row.mover == mushroom_1.FIRST_PLAYER and margin > 0) or (
                    row.mover == mushroom_1.SECOND_PLAYER and margin < 0
                )
                row.outcome = 1.0 if mover_wins else 0.0
        return True


def parse_move(parts: list[str]) -> tuple[int, int, int, int]:
    if len(parts) < 5:
        raise ValueError("move line must contain a side token and four coordinates")
    return tuple(int(value) for value in parts[1:5])  # type: ignore[return-value]


def _looks_like_board_row_line(line: str) -> bool:
    tokens = line.strip().split()
    if not tokens:
        return False
    for token in tokens:
        if len(token) != COLS or not token.isdigit():
            return False
    return True


def parse_init_rows(parts: list[str], lines_iter: Iterator[str] | None = None) -> list[str]:
    rows = list(parts[1:])
    if len(rows) >= ROWS:
        return rows[:ROWS]
    if lines_iter is None:
        raise ValueError("INIT row list is incomplete and no continuation stream was provided")
    for line in lines_iter:
        if not _looks_like_board_row_line(line):
            raise ValueError("INIT row list ended before 10 board rows were read")
        rows.extend(line.strip().split())
        if len(rows) >= ROWS:
            return rows[:ROWS]
    raise ValueError(f"INIT requires {ROWS} rows, got {len(rows)}")


def parse_raw_transcript(path: Path, game_id_start: int = 0) -> list[MoveRow]:
    rows: list[MoveRow] = []
    game: GameBuffer | None = None
    next_game_id = game_id_start

    raw_lines = [line.strip() for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]
    line_iter = iter(raw_lines)

    for line in line_iter:
        parts = line.split()
        if not parts:
            continue

        head = parts[0]
        if head == "INIT":
            if game is not None and game.finalize_scores():
                rows.extend(game.rows)
            init_rows = parse_init_rows(parts, line_iter)
            game = GameBuffer(
                game_id=next_game_id,
                state=mushroom_1.MushroomState.from_rows(init_rows),
                rows=[],
            )
            next_game_id += 1
            continue

        if game is None:
            continue

        if head in {"FIRST", "SECOND"}:
            if game.state.is_terminal():
                continue
            move = parse_move(parts)
            current_state = game.state
            if not current_state.is_legal_action(move):
                continue

            live_count = sum(1 for row in current_state.values for cell in row if cell > 0)
            legal_moves = len(current_state.legal_rectangles())
            rect_id = RECT_COUNT if move == PASS_MOVE else fixed_rect_id(*move)
            phase = phase_for_state(live_count, legal_moves)
            if move == PASS_MOVE:
                bucket = 0
                value = -10_000.0
            else:
                static_score = static_move_score(current_state, move)
                bucket = score_bucket(static_score)
                value = move_value(current_state, move)

            game.rows.append(
                MoveRow(
                    game_id=game.game_id,
                    ply=len(game.rows),
                    mover=current_state.player,
                    rect_id=rect_id,
                    phase=phase,
                    bucket=bucket,
                    move_value=value,
                )
            )
            game.state = current_state.apply_action(move)
            continue

        if head == "SCOREFIRST":
            if len(parts) >= 2:
                game.first_score = int(parts[1])
            continue

        if head == "SCORESECOND":
            if len(parts) >= 2:
                game.second_score = int(parts[1])
            if game.finalize_scores():
                rows.extend(game.rows)
                game = None
            continue

        if head == "FINISH":
            if game is not None and game.finalize_scores():
                rows.extend(game.rows)
            game = None
            continue

    if game is not None and game.finalize_scores():
        rows.extend(game.rows)

    return rows


def parse_csv_log(path: Path, game_id_start: int = 0) -> list[MoveRow]:
    rows: list[MoveRow] = []
    with path.open(newline="", encoding="utf-8") as f:
        reader = csv.DictReader(f)
        for row in reader:
            try:
                rows.append(
                    MoveRow(
                        game_id=game_id_start + int(row["game_id"]),
                        ply=int(row["ply"]),
                        mover=int(row["mover"]),
                        rect_id=int(row["rect_id"]),
                        phase=int(row["phase"]),
                        bucket=int(row["bucket"]),
                        move_value=float(row["move_value"]),
                        outcome=float(row["outcome"]),
                        margin=int(float(row["margin"])),
                    )
                )
            except (KeyError, ValueError):
                continue
    return rows


def load_rows(path: Path, game_id_start: int = 0) -> list[MoveRow]:
    with path.open(encoding="utf-8") as f:
        first_non_empty = ""
        for raw in f:
            line = raw.strip()
            if line:
                first_non_empty = line
                break
    if not first_non_empty:
        return []
    if first_non_empty.startswith("game_id,"):
        return parse_csv_log(path, game_id_start=game_id_start)
    return parse_raw_transcript(path, game_id_start=game_id_start)


def iter_input_paths(paths: list[str]) -> list[Path]:
    resolved: list[Path] = []
    for raw in paths:
        path = Path(raw)
        if path.is_dir():
            resolved.extend(sorted(p for p in path.iterdir() if p.suffix.lower() in {".txt", ".csv"}))
        else:
            resolved.append(path)
    return resolved


def main() -> int:
    parser = argparse.ArgumentParser(description="Convert Mushroom match logs into training CSV data.")
    parser.add_argument("inputs", nargs="+", help="Raw log files or CSV logs to convert.")
    parser.add_argument("--output", "-o", default="", help="Output CSV path. Defaults to stdout.")
    parser.add_argument(
        "--append",
        action="store_true",
        help="Append to the output CSV instead of overwriting it.",
    )
    args = parser.parse_args()

    input_paths = iter_input_paths(args.inputs)
    if not input_paths:
        raise SystemExit("No input logs provided")
    output_path = Path(args.output).resolve() if args.output else None

    all_rows: list[MoveRow] = []
    game_id_offset = 0
    for path in input_paths:
        if not path.exists():
            raise FileNotFoundError(path)
        if output_path is not None and path.resolve() == output_path:
            continue
        rows = load_rows(path, game_id_start=game_id_offset)
        all_rows.extend(rows)
        if rows:
            game_id_offset = max(row.game_id for row in all_rows) + 1

    if not all_rows:
        raise SystemExit("No training rows could be extracted from the provided logs")

    output_stream = sys.stdout
    file_handle = None
    try:
        if args.output:
            out_path = Path(args.output)
            out_path.parent.mkdir(parents=True, exist_ok=True)
            mode = "a" if args.append else "w"
            file_handle = out_path.open(mode, newline="", encoding="utf-8")
            output_stream = file_handle

        writer = csv.writer(output_stream)
        if not args.append or not args.output or (args.output and Path(args.output).stat().st_size == 0):
            writer.writerow(["game_id", "ply", "mover", "rect_id", "phase", "bucket", "move_value", "outcome", "margin"])

        for row in all_rows:
            writer.writerow(
                [
                    row.game_id,
                    row.ply,
                    row.mover,
                    row.rect_id,
                    row.phase,
                    row.bucket,
                    f"{row.move_value:.4f}",
                    f"{row.outcome:.2f}",
                    row.margin,
                ]
            )

        print(
            f"Extracted {len(all_rows)} rows from {len(input_paths)} log(s).",
            file=sys.stderr,
        )
        if args.output:
            print(f"Wrote training CSV to {args.output}", file=sys.stderr)
    finally:
        if file_handle is not None:
            file_handle.close()

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
