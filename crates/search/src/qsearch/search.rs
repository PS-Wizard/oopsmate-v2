use oopsmate_core::{Move, Position};
use oopsmate_eval::Evaluator;
use oopsmate_memory::Bound;
use oopsmate_movegen::{analyze, generate_captures_promotions_with_analysis, MoveList};

use crate::control::{SearchContext, SearchInterrupted};

use super::evasions::qsearch_evasions;
use super::ordering::next_qmove;
use super::shared::{
    delta_prune_move, is_tactical_move, is_valid_encoded_move, pack_static_eval, see_prune_move,
    NO_STATIC_EVAL,
};

pub(crate) fn qsearch<E: Evaluator>(
    pos: &mut Position,
    ply: u8,
    mut alpha: i32,
    beta: i32,
    ctx: &mut SearchContext<'_>,
    evaluator: &mut E,
) -> Result<i32, SearchInterrupted> {
    ctx.enter_node()?;
    #[cfg(feature = "telemetry")]
    {
        ctx.telemetry.q_nodes += 1;
    }

    if pos.rule50() >= 100 || pos.is_repetition() {
        return Ok(0);
    }

    let hash = pos.hash();
    let alpha_orig = alpha;
    let mut tt_move = Move::NULL;
    let mut tt_static_eval = None;

    if let Some(hit) = ctx.tt.probe(hash, ply) {
        #[cfg(feature = "telemetry")]
        {
            ctx.telemetry.tt_hits += 1;
        }
        tt_move = hit.best_move;
        if hit.static_eval != NO_STATIC_EVAL {
            tt_static_eval = Some(i32::from(hit.static_eval));
        }

        match hit.bound {
            Bound::Exact => {
                #[cfg(feature = "telemetry")]
                {
                    ctx.telemetry.tt_cutoffs += 1;
                }
                return Ok(hit.score);
            }
            Bound::Lower if hit.score >= beta => {
                #[cfg(feature = "telemetry")]
                {
                    ctx.telemetry.tt_cutoffs += 1;
                }
                return Ok(hit.score);
            }
            Bound::Upper if hit.score <= alpha => {
                #[cfg(feature = "telemetry")]
                {
                    ctx.telemetry.tt_cutoffs += 1;
                }
                return Ok(hit.score);
            }
            _ => {}
        }
    }

    let analysis = analyze(pos);
    if analysis.in_check() {
        return qsearch_evasions(pos, &analysis, tt_move, ply, alpha, beta, ctx, evaluator);
    }

    let static_eval = static_eval(pos, tt_static_eval, ctx, evaluator);
    let mut best_move = Move::NULL;
    let mut best_score = static_eval;

    if static_eval >= beta {
        ctx.tt.store(
            hash,
            ply,
            Move::NULL,
            static_eval,
            pack_static_eval(static_eval),
            0,
            Bound::Lower,
        );
        return Ok(static_eval);
    }

    if static_eval > alpha {
        alpha = static_eval;
    }

    let mut moves = MoveList::new();
    generate_captures_promotions_with_analysis(pos, &analysis, &mut moves);

    let mut skip = Move::NULL;
    if tt_move != Move::NULL
        && is_valid_encoded_move(tt_move)
        && is_tactical_move(tt_move)
        && moves.contains(tt_move)
    {
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

        if score > best_score {
            best_score = score;
            best_move = tt_move;
        }

        if score >= beta {
            ctx.tt.store(
                hash,
                ply,
                tt_move,
                score,
                pack_static_eval(static_eval),
                0,
                Bound::Lower,
            );
            return Ok(score);
        }

        if score > alpha {
            alpha = score;
        }
    }

    let mut next = 0usize;
    while let Some(mv) = next_qmove(pos, &mut moves, &mut next, skip) {
        if delta_prune_move(pos, mv, static_eval, alpha) || see_prune_move(pos, mv) {
            continue;
        }

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
            ctx.tt.store(
                hash,
                ply,
                mv,
                score,
                pack_static_eval(static_eval),
                0,
                Bound::Lower,
            );
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
    ctx.tt.store(
        hash,
        ply,
        best_move,
        best_score,
        pack_static_eval(static_eval),
        0,
        bound,
    );

    Ok(best_score)
}

#[inline(always)]
fn static_eval<E: Evaluator>(
    pos: &Position,
    tt_static_eval: Option<i32>,
    _ctx: &mut SearchContext<'_>,
    evaluator: &mut E,
) -> i32 {
    if let Some(score) = tt_static_eval {
        #[cfg(feature = "telemetry")]
        {
            _ctx.telemetry.tt_static_eval_reuses += 1;
        }
        score
    } else {
        #[cfg(feature = "telemetry")]
        {
            _ctx.telemetry.eval_calls += 1;
        }
        evaluator.evaluate(pos)
    }
}
