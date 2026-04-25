use oopsmate_core::{Move, Position};
use oopsmate_eval::Evaluator;
use oopsmate_memory::Bound;
use oopsmate_movegen::analyze;

use crate::control::{SearchContext, SearchInterrupted};
use crate::qsearch::{qsearch, NO_STATIC_EVAL};
use crate::selectivity::{
    can_use_selective_pruning, needs_static_eval, null_move_depth, razor_margin, rfp_margin,
    should_apply_iir, should_prune_reverse_futility, should_try_null_move, should_try_probcut,
    should_try_razoring, NodeState,
};
use crate::tune::PROBCUT_MIN_DEPTH;
use crate::types::is_mate_score;

use super::move_loop::search_moves;
use super::probcut::try_probcut;
use super::shared::pack_static_eval;

pub(crate) fn search_node<E: Evaluator>(
    pos: &mut Position,
    mut depth: u8,
    node: NodeState,
    alpha: i32,
    beta: i32,
    ctx: &mut SearchContext<'_>,
    evaluator: &mut E,
) -> Result<i32, SearchInterrupted> {
    if depth == 0 {
        return qsearch(pos, node.ply, alpha, beta, ctx, evaluator);
    }

    ctx.enter_node()?;
    #[cfg(feature = "telemetry")]
    {
        ctx.telemetry.main_nodes += 1;
    }

    if pos.rule50() >= 100 || pos.is_repetition() {
        return Ok(0);
    }

    let hash = pos.hash();
    let alpha_orig = alpha;
    let mut tt_move = Move::NULL;
    let mut stored_static_eval = NO_STATIC_EVAL;

    if let Some(hit) = ctx.tt.probe(hash, node.ply) {
        #[cfg(feature = "telemetry")]
        {
            ctx.telemetry.tt_hits += 1;
        }
        tt_move = hit.best_move;
        stored_static_eval = hit.static_eval;
        if hit.depth >= depth {
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
    }

    let analysis = analyze(pos);
    let in_check = analysis.in_check();
    let can_selectively_prune = can_use_selective_pruning(pos, node, alpha, beta, in_check);
    let need_probcut_eval =
        !node.pv_node && !in_check && depth >= PROBCUT_MIN_DEPTH && !is_mate_score(beta);
    let static_eval = if needs_static_eval(depth, can_selectively_prune) || need_probcut_eval {
        let raw_static_eval = if stored_static_eval != NO_STATIC_EVAL {
            #[cfg(feature = "telemetry")]
            {
                ctx.telemetry.tt_static_eval_reuses += 1;
            }
            i32::from(stored_static_eval)
        } else {
            #[cfg(feature = "telemetry")]
            {
                ctx.telemetry.eval_calls += 1;
            }
            let score = evaluator.evaluate(pos);
            stored_static_eval = pack_static_eval(score);
            score
        };
        raw_static_eval + ctx.history.correction_score(pos.side_to_move(), pos.pawn_hash())
    } else {
        0
    };

    if should_try_razoring(depth, static_eval, alpha, can_selectively_prune) {
        let margin = razor_margin(depth);
        let window_alpha = alpha - margin;
        let score = qsearch(
            pos,
            node.ply,
            window_alpha,
            window_alpha + 1,
            ctx,
            evaluator,
        )?;
        if score < window_alpha {
            #[cfg(feature = "telemetry")]
            {
                ctx.telemetry.razor_cutoffs += 1;
            }
            return Ok(score);
        }
    }

    if should_prune_reverse_futility(depth, static_eval, beta, can_selectively_prune) {
        #[cfg(feature = "telemetry")]
        {
            ctx.telemetry.rfp_cutoffs += 1;
        }
        let score = static_eval - rfp_margin(depth);
        ctx.tt.store(
            hash,
            node.ply,
            Move::NULL,
            score,
            stored_static_eval,
            depth,
            Bound::Lower,
        );
        return Ok(score);
    }

    if should_try_null_move(depth, static_eval, beta, can_selectively_prune) {
        #[cfg(feature = "telemetry")]
        {
            ctx.telemetry.null_attempts += 1;
        }
        evaluator.push_null_move();
        pos.make_null_move();
        let score = match search_node(
            pos,
            null_move_depth(depth, static_eval, beta),
            node.child(false, -beta, -beta + 1),
            -beta,
            -beta + 1,
            ctx,
            evaluator,
        ) {
            Ok(score) => -score,
            Err(err) => {
                pos.unmake_null_move();
                evaluator.pop_move();
                return Err(err);
            }
        };
        pos.unmake_null_move();
        evaluator.pop_move();

        if score >= beta {
            #[cfg(feature = "telemetry")]
            {
                ctx.telemetry.null_cutoffs += 1;
            }
            ctx.tt.store(
                hash,
                node.ply,
                Move::NULL,
                beta,
                stored_static_eval,
                depth,
                Bound::Lower,
            );
            return Ok(beta);
        }
    }

    if should_apply_iir(depth, node, tt_move) {
        depth -= 1;
    }

    if should_try_probcut(depth, node, beta, in_check, static_eval) {
        #[cfg(feature = "telemetry")]
        {
            ctx.telemetry.probcut_attempts += 1;
        }
        if let Some((mv, score)) = try_probcut(pos, &analysis, depth, node, beta, ctx, evaluator)? {
            ctx.tt.store(
                hash,
                node.ply,
                mv,
                score,
                stored_static_eval,
                depth,
                Bound::Lower,
            );
            return Ok(score);
        }
    }

    search_moves(
        pos,
        &analysis,
        depth,
        node,
        alpha_orig,
        alpha,
        beta,
        tt_move,
        stored_static_eval,
        static_eval,
        in_check,
        hash,
        ctx,
        evaluator,
    )
}
