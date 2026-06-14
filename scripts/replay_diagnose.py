from __future__ import annotations

import argparse
import math
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

SCRIPT_DIR = Path(__file__).resolve().parent
for candidate in (SCRIPT_DIR.parents[1], SCRIPT_DIR.parents[2]):
    if str(candidate) not in sys.path:
        sys.path.insert(0, str(candidate))

import mushroom_1


ROWS = 10
COLS = 17
PASS_MOVE = (-1, -1, -1, -1)


@dataclass(slots=True)
class MoveRecord:
    ply: int
    mover: int
    move: tuple[int, int, int, int]
    time_ms: int | None
    before: mushroom_1.MushroomState
    after: mushroom_1.MushroomState
    before_eval: float
    after_eval: float
    static_best: tuple[int, int, int, int]
    static_best_eval: float
    search_best: tuple[int, int, int, int]
    search_best_eval: float
    search_depth: int
    live_count: int
    phase: str


def parse_int(value: str | None) -> int | None:
    if value is None:
        return None
    try:
        return int(value)
    except ValueError:
        return None


def parse_init_rows(tokens: list[str], lines: list[str], start_index: int) -> tuple[list[str], int]:
    rows = tokens[1:]
    idx = start_index
    while len(rows) < ROWS and idx < len(lines):
        candidate = lines[idx].strip()
        idx += 1
        if not candidate:
            continue
        extra = candidate.split()
        rows.extend(extra)
    if len(rows) < ROWS:
        raise ValueError(f"INIT requires {ROWS} rows, got {len(rows)}")
    return rows[:ROWS], idx


def parse_transcript(path: Path) -> tuple[mushroom_1.MushroomState, list[tuple[str, tuple[int, int, int, int], int | None]], int | None, int | None]:
    raw_lines = [line.strip() for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]
    state: mushroom_1.MushroomState | None = None
    moves: list[tuple[str, tuple[int, int, int, int], int | None]] = []
    first_score: int | None = None
    second_score: int | None = None

    idx = 0
    while idx < len(raw_lines):
        line = raw_lines[idx]
        idx += 1
        parts = line.split()
        if not parts:
            continue

        head = parts[0]
        if head == "INIT":
            rows, idx = parse_init_rows(parts, raw_lines, idx)
            state = mushroom_1.MushroomState.from_rows(rows)
            continue

        if head in {"FIRST", "SECOND"}:
            if len(parts) < 5:
                continue
            move = tuple(int(v) for v in parts[1:5])
            time_ms = parse_int(parts[5]) if len(parts) >= 6 else None
            moves.append((head, move, time_ms))
            continue

        if head == "SCOREFIRST" and len(parts) >= 2:
            first_score = parse_int(parts[1])
            continue

        if head == "SCORESECOND" and len(parts) >= 2:
            second_score = parse_int(parts[1])
            continue

    if state is None:
        raise ValueError(f"{path} does not contain INIT")
    return state, moves, first_score, second_score


def render_board(state: mushroom_1.MushroomState) -> str:
    lines = ["    " + "".join(f"{c % 10}" for c in range(COLS))]
    for r in range(ROWS):
        row_chars: list[str] = []
        for c in range(COLS):
            owner = state.owners[r][c]
            value = state.values[r][c]
            if owner == mushroom_1.FIRST_PLAYER:
                row_chars.append("F")
            elif owner == mushroom_1.SECOND_PLAYER:
                row_chars.append("S")
            elif value > 0:
                row_chars.append(str(value))
            else:
                row_chars.append(".")
        lines.append(f"{r:>2}  " + "".join(row_chars))
    return "\n".join(lines)


def phase_for_live_count(live_count: int) -> str:
    if live_count <= 12:
        return "endgame"
    if live_count <= 25:
        return "midgame"
    return "opening"


def move_str(move: tuple[int, int, int, int]) -> str:
    return f"({move[0]}, {move[1]}, {move[2]}, {move[3]})"


def all_actions(state: mushroom_1.MushroomState) -> list[tuple[int, int, int, int]]:
    actions = state.legal_actions()
    return [tuple(int(v) for v in action) for action in actions]


def shallow_search(
    state: mushroom_1.MushroomState,
    depth: int,
    root_player: int,
    memo: dict[tuple[object, int, int], tuple[float, tuple[int, int, int, int] | None]],
) -> tuple[float, tuple[int, int, int, int] | None]:
    key = (state.state_key(), depth, root_player)
    if key in memo:
        return memo[key]

    if depth <= 0 or state.is_terminal():
        result = (float(state.evaluate(root_player)), None)
        memo[key] = result
        return result

    actions = all_actions(state)
    if not actions:
        result = (float(state.evaluate(root_player)), None)
        memo[key] = result
        return result

    maximizing = state.current_player() == root_player
    ordered = sorted(
        actions,
        key=lambda mv: state.apply_action(mv).evaluate(root_player),
        reverse=maximizing,
    )

    best_val = -math.inf if maximizing else math.inf
    best_move: tuple[int, int, int, int] | None = None

    for move in ordered:
        child = state.apply_action(move)
        child_val, _ = shallow_search(child, depth - 1, root_player, memo)
        value = child_val
        if maximizing:
            if value > best_val:
                best_val = value
                best_move = move
        else:
            if value < best_val:
                best_val = value
                best_move = move

    result = (best_val, best_move)
    memo[key] = result
    return result


def best_static_move(state: mushroom_1.MushroomState, root_player: int) -> tuple[tuple[int, int, int, int], float]:
    actions = all_actions(state)
    if not actions:
        return PASS_MOVE, float(state.evaluate(root_player))

    maximizing = state.current_player() == root_player
    scored = []
    for move in actions:
        child = state.apply_action(move)
        scored.append((float(child.evaluate(root_player)), move))

    scored.sort(key=lambda item: item[0], reverse=maximizing)
    best_score, best_move = scored[0]
    return best_move, best_score


def classify_move(
    record: MoveRecord,
    _losing_player: int,
    gap_threshold: float,
) -> str:
    actual = record.after_eval
    static_gap = record.static_best_eval - actual
    search_gap = record.search_best_eval - actual
    actual_is_static_best = record.move == record.static_best
    actual_is_search_best = record.move == record.search_best

    if actual_is_search_best and not actual_is_static_best:
        return "eval_or_horizon_suspect"
    if actual_is_static_best and search_gap > gap_threshold:
        return "search_suspect"
    if not actual_is_search_best and search_gap >= gap_threshold:
        return "search_suspect"
    if static_gap >= gap_threshold:
        return "eval_or_horizon_suspect"
    return "unclear"


def analyze_file(path: Path, search_depth: int, gap_threshold: float) -> str:
    state, moves, first_score, second_score = parse_transcript(path)
    records: list[MoveRecord] = []
    illegal_move: tuple[int, str, tuple[int, int, int, int], int | None, mushroom_1.MushroomState] | None = None

    for ply, (side_name, move, time_ms) in enumerate(moves):
        if state.is_terminal():
            break
        mover = state.current_player()
        before = state
        before_eval = float(before.evaluate(mover))
        live_count = sum(1 for row in before.values for cell in row if cell > 0)
        phase = phase_for_live_count(live_count)
        if not before.is_legal_action(move):
            illegal_move = (ply, side_name, move, time_ms, before)
            break

        after = before.apply_action(move)
        after_eval = float(after.evaluate(mover))
        static_best_move, static_best_eval = best_static_move(before, mover)
        search_best_eval, search_best_move = shallow_search(before, search_depth, mover, {})
        record = MoveRecord(
            ply=ply,
            mover=mover,
            move=move,
            time_ms=time_ms,
            before=before,
            after=after,
            before_eval=before_eval,
            after_eval=after_eval,
            static_best=static_best_move,
            static_best_eval=static_best_eval,
            search_best=search_best_move or static_best_move,
            search_best_eval=search_best_eval,
            search_depth=search_depth,
            live_count=live_count,
            phase=phase,
        )
        records.append(record)
        state = after

    if first_score is None or second_score is None:
        first_score, second_score = state.scores()

    if illegal_move is not None:
        ply, side_name, move, time_ms, before = illegal_move
        out: list[str] = []
        out.append(f"FILE: {path.name}")
        out.append("REPLAY_STATUS: illegal_move")
        out.append(f"ILLEGAL_PLY: {ply}")
        out.append(f"ILLEGAL_SIDE: {side_name}")
        out.append(f"ILLEGAL_MOVE: {move_str(move)}")
        if time_ms is not None:
            out.append(f"ILLEGAL_TIME_MS: {time_ms}")
        out.append("")
        out.append("BOARD BEFORE:")
        out.append(render_board(before))
        out.append("")
        out.append("NOTE: this transcript does not match the current legality rules used by replay_diagnose.py.")
        return "\n".join(out)

    margin = first_score - second_score
    if margin > 0:
        winner = mushroom_1.FIRST_PLAYER
        loser = mushroom_1.SECOND_PLAYER
    elif margin < 0:
        winner = mushroom_1.SECOND_PLAYER
        loser = mushroom_1.FIRST_PLAYER
    else:
        winner = mushroom_1.NO_OWNER
        loser = mushroom_1.NO_OWNER

    loser_name = "DRAW" if loser == mushroom_1.NO_OWNER else ("FIRST" if loser == mushroom_1.FIRST_PLAYER else "SECOND")

    losing_records = [r for r in records if r.mover == loser] if loser != mushroom_1.NO_OWNER else []
    culprit: MoveRecord | None = None
    if losing_records:
        culprit = max(
            losing_records,
            key=lambda r: max(r.search_best_eval - r.after_eval, r.static_best_eval - r.after_eval),
        )

    out: list[str] = []
    out.append(f"FILE: {path.name}")
    out.append(f"FINAL: FIRST={first_score} SECOND={second_score} MARGIN={margin} WINNER={'DRAW' if winner == mushroom_1.NO_OWNER else ('FIRST' if winner == mushroom_1.FIRST_PLAYER else 'SECOND')}")
    out.append(f"SEARCH_DEPTH: {search_depth}")
    out.append("")

    if culprit is None:
        out.append("No losing move candidate found.")
        out.append("This game is a draw or has no recorded moves for the losing side.")
        return "\n".join(out)

    classification = classify_move(culprit, loser, gap_threshold)
    out.append(f"LOSING_SIDE: {loser_name}")
    out.append(f"PHASE: {culprit.phase} (live={culprit.live_count})")
    out.append(f"PLY: {culprit.ply}")
    out.append(f"LOGGED_MOVE: {move_str(culprit.move)}")
    out.append(f"STATIC_BEST: {move_str(culprit.static_best)} score={culprit.static_best_eval:.2f}")
    out.append(f"SEARCH_BEST: {move_str(culprit.search_best)} score={culprit.search_best_eval:.2f}")
    out.append(f"ACTUAL_AFTER_EVAL: {culprit.after_eval:.2f}")
    out.append(f"GAP_TO_SEARCH_BEST: {culprit.search_best_eval - culprit.after_eval:.2f}")
    out.append(f"GAP_TO_STATIC_BEST: {culprit.static_best_eval - culprit.after_eval:.2f}")
    out.append(f"CLASSIFICATION: {classification}")
    out.append("")
    out.append("BOARD BEFORE:")
    out.append(render_board(culprit.before))
    out.append("")
    out.append("BOARD AFTER:")
    out.append(render_board(culprit.after))
    out.append("")

    actions = all_actions(culprit.before)
    scored_static = []
    scored_search = []
    memo: dict[tuple[object, int], tuple[float, tuple[int, int, int, int] | None]] = {}
    for move in actions:
        child = culprit.before.apply_action(move)
        scored_static.append((float(child.evaluate(culprit.mover)), move))
        search_val, _ = shallow_search(culprit.before.apply_action(move), search_depth - 1, culprit.mover, memo) if search_depth > 0 else (float(child.evaluate(culprit.mover)), None)
        scored_search.append((search_val, move))

    scored_static.sort(key=lambda item: item[0], reverse=True)
    scored_search.sort(key=lambda item: item[0], reverse=True)

    out.append(f"TOP_STATIC_{min(5, len(scored_static))}:")
    for score, move in scored_static[:5]:
        out.append(f"  {move_str(move):>18}  {score:8.2f}")

    out.append(f"TOP_SEARCH_{min(5, len(scored_search))}:")
    for score, move in scored_search[:5]:
        out.append(f"  {move_str(move):>18}  {score:8.2f}")

    top_culprits = sorted(
        losing_records,
        key=lambda r: max(r.search_best_eval - r.after_eval, r.static_best_eval - r.after_eval),
        reverse=True,
    )[:3]
    out.append("")
    out.append("TOP_CULPRITS:")
    for idx, rec in enumerate(top_culprits, start=1):
        out.append(
            f"  #{idx} ply={rec.ply} move={move_str(rec.move)} phase={rec.phase} "
            f"gap_search={rec.search_best_eval - rec.after_eval:.2f} "
            f"gap_static={rec.static_best_eval - rec.after_eval:.2f}"
        )

    return "\n".join(out)


def main() -> int:
    parser = argparse.ArgumentParser(description="Replay Mushroom transcripts and diagnose likely search/eval mistakes.")
    parser.add_argument("files", nargs="+", help="Transcript files such as 2.txt and 14.txt")
    parser.add_argument("--search-depth", type=int, default=2, help="Shallow minimax depth used for diagnosis")
    parser.add_argument("--gap-threshold", type=float, default=3.0, help="Score gap needed to call a move suspicious")
    parser.add_argument("--out-dir", type=Path, default=Path("scripts") / "replay_logs", help="Where to write per-game log files")
    parser.add_argument("--stdout", action="store_true", help="Also print the report to stdout")
    args = parser.parse_args()

    if args.search_depth > 2:
        print("[warn] search-depth > 2 may be slow; consider adding alpha-beta if you need deeper replay", file=sys.stderr)

    out_dir: Path = args.out_dir
    out_dir.mkdir(parents=True, exist_ok=True)

    for file_arg in args.files:
        path = Path(file_arg)
        if not path.exists():
            raise FileNotFoundError(path)
        report = analyze_file(path, max(args.search_depth, 0), args.gap_threshold)
        out_path = out_dir / f"{path.stem}.replay.log"
        out_path.write_text(report + "\n", encoding="utf-8")
        print(f"[ok] wrote {out_path}")
        if args.stdout:
            print()
            print(report)
            print()

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
