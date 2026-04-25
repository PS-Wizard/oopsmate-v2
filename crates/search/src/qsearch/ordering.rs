use oopsmate_core::{Move, Position};
use oopsmate_movegen::MoveList;

use super::shared::score_qmove;

pub(super) fn next_qmove(
    pos: &Position,
    moves: &mut MoveList,
    next: &mut usize,
    skip: Move,
) -> Option<Move> {
    while *next < moves.len() {
        let mut best = *next;
        let mut best_score = score_qmove(pos, moves.as_slice()[best]);

        for index in (*next + 1)..moves.len() {
            let score = score_qmove(pos, moves.as_slice()[index]);
            if score > best_score {
                best = index;
                best_score = score;
            }
        }

        moves.swap(*next, best);
        let mv = moves.as_slice()[*next];
        *next += 1;

        if mv == skip {
            continue;
        }

        return Some(mv);
    }

    None
}
