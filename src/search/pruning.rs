use crate::board::Board;
use crate::search::Search;
use crate::types::*;

impl Search {
    pub(crate) fn tactical_priority(&self, board: &Board, mv: Move) -> i32 {
        if mv == PASS {
            return -10_000;
        }

        let (_sd, recaptured, fresh, _live, _own, area) = board.action_score(mv);
        let live_count = board.live_mask.popcount() as i32;
        let mut score = 0i32;

        if recaptured > 0 {
            score += 120 * recaptured;
        }
        if fresh > 0 {
            score += 18 * fresh;
        }
        if area >= 8 {
            score += 24;
        } else if area >= 5 {
            score += 12;
        }

        if live_count <= 24 {
            score += 24 * recaptured + 8 * fresh;
        }
        if live_count <= 16 {
            score += 16;
        }

        let (r1, c1, r2, c2) = mv;
        let touches_corner = (r1 == 0 && c1 == 0)
            || (r1 == 0 && c2 == COLS as i8 - 1)
            || (r2 == ROWS as i8 - 1 && c1 == 0)
            || (r2 == ROWS as i8 - 1 && c2 == COLS as i8 - 1);
        if touches_corner {
            score += 30;
        } else if r1 == 0 || c1 == 0 || r2 == ROWS as i8 - 1 || c2 == COLS as i8 - 1 {
            score += 10;
        }

        score
    }

    pub fn is_tactical_move(&self, board: &Board, mv: Move) -> bool {
        self.tactical_priority(board, mv) >= 80
    }

    pub fn see(&self, board: &Board, mv: Move, _rects: &[crate::movegen::RectInfo]) -> i32 {
        board.rectangle_exchange_eval(mv, 2)
    }
}
