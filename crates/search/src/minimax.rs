use oopsmate_core::Position;
use oopsmate_eval::Evaluator;
use oopsmate_movegen::{MoveList, generate_all, is_square_attacked};

use crate::control::{SearchContext, SearchInterrupted};
use crate::types::mate_score;

pub(crate) fn search_node<E: Evaluator>(
    pos: &mut Position,
    depth: u8,
    ply: u8,
    ctx: &mut SearchContext<'_>,
    evaluator: &E,
) -> Result<i32, SearchInterrupted> {
    ctx.enter_node()?;

    if pos.rule50() >= 100 || pos.is_repetition() {
        return Ok(0);
    }

    if depth == 0 {
        if !in_check(pos) {
            return Ok(evaluator.evaluate(pos));
        }
    }

    let mut moves = MoveList::new();
    generate_all(pos, &mut moves);

    if moves.len() == 0 {
        return Ok(if in_check(pos) { -mate_score(ply) } else { 0 });
    }

    if depth == 0 {
        return Ok(evaluator.evaluate(pos));
    }

    let mut best = i32::MIN / 2;
    for &mv in moves.as_slice() {
        pos.make_move(mv);
        let score = match search_node(pos, depth - 1, ply + 1, ctx, evaluator) {
            Ok(score) => -score,
            Err(err) => {
                pos.unmake_move(mv);
                return Err(err);
            }
        };
        pos.unmake_move(mv);

        if score > best {
            best = score;
        }
    }

    Ok(best)
}

#[inline(always)]
#[must_use]
pub(crate) fn in_check(pos: &Position) -> bool {
    let us = pos.side_to_move();
    let king_sq = pos.board().king_square(us);
    is_square_attacked(pos, king_sq, us.flip())
}
