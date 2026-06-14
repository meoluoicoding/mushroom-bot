import sys
import os
import time
# Add parent directory to path so we can import mushroom module
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import mushroom_1
from mushroom_1 import MushroomState, PASS_MOVE, MushroomSearchPolicy, default_mushroom_policy_config


class Bot:
    def __init__(self):
        self.state = None
        self.my_player = 1
        self.policy = MushroomSearchPolicy(default_mushroom_policy_config())

    def handle_ready(self, parts):
        self.my_player = 1 if len(parts) < 2 or parts[1] == "FIRST" else -1
        print("OK")
        sys.stdout.flush()

    def handle_init(self, parts, input_stream):
        rows = list(parts[1:])
        while len(rows) < 10:
            line = input_stream.readline()
            if not line:
                break
            rows.extend(line.strip().split())
        rows = rows[:10]
        self.state = MushroomState.from_rows(rows)
        self.policy = MushroomSearchPolicy(default_mushroom_policy_config())

    def handle_time(self, parts):
        if not self.state or self.state.is_terminal():
            print("-1 -1 -1 -1")
            sys.stdout.flush()
            return

        t1 = int(parts[1]) if len(parts) > 1 else 10000
        t2 = int(parts[2]) if len(parts) > 2 else 10000

        if self.my_player == 1:
            self.policy.set_time(t1, t2)
        else:
            self.policy.set_time(t2, t1)

        move = self.policy(self.state)
        self.state = self.state.apply_action(move)
        print(f"{move[0]} {move[1]} {move[2]} {move[3]}")
        sys.stdout.flush()

    def handle_opp(self, parts):
        if self.state and not self.state.is_terminal() and len(parts) >= 5:
            try:
                move = (int(parts[1]), int(parts[2]), int(parts[3]), int(parts[4]))
                if self.state.is_legal_action(move):
                    self.state = self.state.apply_action(move)
            except ValueError:
                pass

    def main(self):
        input_stream = sys.stdin
        for line in input_stream:
            parts = line.strip().split()
            if not parts:
                continue
            cmd = parts[0]
            if cmd == "READY":
                self.handle_ready(parts)
            elif cmd == "INIT":
                self.handle_init(parts, input_stream)
            elif cmd == "TIME":
                self.handle_time(parts)
            elif cmd == "OPP":
                self.handle_opp(parts)
            elif cmd == "FINISH":
                break


if __name__ == "__main__":
    bot = Bot()
    bot.main()
