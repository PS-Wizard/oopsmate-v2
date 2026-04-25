use oopsmate_core::{Move, Position};
use oopsmate_eval::Evaluator;
use oopsmate_memory::Bound;
use oopsmate_movegen::{generate_evasions_with_analysis, Analysis, MoveList};

use crate::control::{SearchContext, SearchInterrupted};
use crate::types::mate_score;

use super::ordering::next_qmove;
use super::search::qsearch;
use super::shared::{is_valid_encoded_move, NO_STATIC_EVAL};

pub(super) fn qsearch_evasions<E: Evaluator>(
    pos: &mut Position,
    analysis: &Analysis,
    tt_move: Move,
    ply: u8,
    mut alpha: i32,
    beta: i32,
    ctx: &mut SearchContext<'_>,
    evaluator: &mut E,
) -> Result<i32, SearchInterrupted> {
    let hash = pos.hash();
    let alpha_orig = alpha;
    let mut moves = MoveList::new();
    generate_evasions_with_analysis(pos, analysis, &mut moves);

    if moves.is_empty() {
        let score = -mate_score(ply);
        ctx.tt.store(hash, ply, Move::NULL, score, NO_STATIC_EVAL, 0, Bound::Exact);
        return Ok(score);
    }

    let mut best_move = Move::NULL;
    let mut best_score = i32::MIN / 2;
    let mut next = 0usize;
    let mut skip = Move::NULL;

    if tt_move != Move::NULL && is_valid_encoded_move(tt_move) && moves.contains(tt_move) {
        skip = tt_move;
        evaluator.push_move(pos, tt_move);
        pos.make_move(tt_move);
        let score = match qsearch(pos, ply + 1, -beta, -alpha, ctx, evaluator) {
            Ok(score) => -score,
            Err(err) => {
                pos.unmake_move(tt_move);
                evaluator.pop_move();
                return Err(err);
            }
        };
        pos.unmake_move(tt_move);
        evaluator.pop_move();

        best_score = score;
        best_move = tt_move;

        if score >= beta {
            ctx.tt.store(hash, ply, tt_move, score, NO_STATIC_EVAL, 0, Bound::Lower);
            return Ok(score);
        }

        if score > alpha {
            alpha = score;
        }
    }

    while let Some(mv) = next_qmove(pos, &mut moves, &mut next, skip) {
        evaluator.push_move(pos, mv);
        pos.make_move(mv);
        let score = match qsearch(pos, ply + 1, -beta, -alpha, ctx, evaluator) {
            Ok(score) => -score,
            Err(err) => {
                pos.unmake_move(mv);
                evaluator.pop_move();
                return Err(err);
            }
        };
        pos.unmake_move(mv);
        evaluator.pop_move();

        if score > best_score {
            best_score = score;
            best_move = mv;
        }

        if score >= beta {
            ctx.tt.store(hash, ply, mv, score, NO_STATIC_EVAL, 0, Bound::Lower);
            return Ok(score);
        }

        if score > alpha {
            alpha = score;
        }
    }

    let bound = if best_score <= alpha_orig {
        Bound::Upper
    } else {
        Bound::Exact
    };
    ctx.tt.store(hash, ply, best_move, best_score, NO_STATIC_EVAL, 0, bound);

    Ok(best_score)
}
