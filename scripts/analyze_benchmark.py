from __future__ import annotations

import argparse
import csv
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path


@dataclass
class Agg:
    count: int = 0
    wins: float = 0.0
    move_value_sum: float = 0.0
    margin_sum: float = 0.0

    def add(self, outcome: float, move_value: float, margin: float) -> None:
        self.count += 1
        self.wins += outcome
        self.move_value_sum += move_value
        self.margin_sum += margin

    @property
    def win_rate(self) -> float:
        return self.wins / self.count if self.count else 0.0

    @property
    def avg_move_value(self) -> float:
        return self.move_value_sum / self.count if self.count else 0.0

    @property
    def avg_margin(self) -> float:
        return self.margin_sum / self.count if self.count else 0.0


def fmt_pct(value: float) -> str:
    return f"{value * 100:5.1f}%"


def fmt_float(value: float) -> str:
    return f"{value:8.2f}"


def main() -> int:
    parser = argparse.ArgumentParser(description="Analyze Mushroom benchmark CSV logs.")
    parser.add_argument("csv_path", help="Path to benchmark CSV, e.g. benchmark_vs_cordyceps_50.csv")
    parser.add_argument("--top", type=int, default=8, help="How many buckets to show per table.")
    args = parser.parse_args()

    path = Path(args.csv_path)
    if not path.exists():
        raise FileNotFoundError(path)

    overall = Agg()
    by_phase: dict[str, Agg] = defaultdict(Agg)
    by_bucket: dict[str, Agg] = defaultdict(Agg)
    by_mover: dict[str, Agg] = defaultdict(Agg)
    by_bucket_phase: dict[tuple[str, str], Agg] = defaultdict(Agg)
    rows: list[dict[str, str]] = []

    with path.open(newline="", encoding="utf-8") as f:
        reader = csv.DictReader(f)
        for row in reader:
            rows.append(row)
            outcome = float(row["outcome"])
            move_value = float(row["move_value"])
            margin = float(row["margin"])
            phase = row["phase"]
            bucket = row["bucket"]
            mover = row["mover"]

            overall.add(outcome, move_value, margin)
            by_phase[phase].add(outcome, move_value, margin)
            by_bucket[bucket].add(outcome, move_value, margin)
            by_mover[mover].add(outcome, move_value, margin)
            by_bucket_phase[(phase, bucket)].add(outcome, move_value, margin)

    def print_table(title: str, data: dict[str, Agg]) -> None:
        print(f"\n{title}")
        print(f"{'key':>8} {'n':>6} {'win%':>8} {'avg_move':>10} {'avg_margin':>11}")
        for key, agg in sorted(data.items(), key=lambda kv: (-kv[1].count, kv[0])):
            print(
                f"{key:>8} {agg.count:6d} {fmt_pct(agg.win_rate):>8} "
                f"{fmt_float(agg.avg_move_value):>10} {fmt_float(agg.avg_margin):>11}"
            )

    print(f"File: {path}")
    print(f"Rows: {overall.count}")
    print(f"Overall win% (mover perspective): {fmt_pct(overall.win_rate)}")
    print(f"Overall avg move_value: {fmt_float(overall.avg_move_value)}")
    print(f"Overall avg margin: {fmt_float(overall.avg_margin)}")

    print_table("By phase", by_phase)
    print_table("By bucket", by_bucket)
    print_table("By mover", by_mover)

    # Show the strongest phase/bucket pairs by sample count.
    print("\nBy phase + bucket")
    print(f"{'phase':>8} {'bucket':>8} {'n':>6} {'win%':>8} {'avg_move':>10}")
    for (phase, bucket), agg in sorted(by_bucket_phase.items(), key=lambda kv: (-kv[1].count, kv[0][0], kv[0][1]))[: max(args.top, 1)]:
        print(
            f"{phase:>8} {bucket:>8} {agg.count:6d} {fmt_pct(agg.win_rate):>8} {fmt_float(agg.avg_move_value):>10}"
        )

    # Simple high-vs-low move value split.
    values = sorted(float(row["move_value"]) for row in rows)
    if values:
        median = values[len(values) // 2]
        low = Agg()
        high = Agg()
        for row in rows:
            outcome = float(row["outcome"])
            move_value = float(row["move_value"])
            margin = float(row["margin"])
            if move_value <= median:
                low.add(outcome, move_value, margin)
            else:
                high.add(outcome, move_value, margin)

        print("\nMove value split")
        print(f"Median move_value: {fmt_float(median)}")
        print(f"{'group':>8} {'n':>6} {'win%':>8} {'avg_move':>10} {'avg_margin':>11}")
        for label, agg in (("low", low), ("high", high)):
            print(
                f"{label:>8} {agg.count:6d} {fmt_pct(agg.win_rate):>8} "
                f"{fmt_float(agg.avg_move_value):>10} {fmt_float(agg.avg_margin):>11}"
            )

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
