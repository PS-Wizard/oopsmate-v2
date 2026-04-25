use oopsmate_core::{Move, Position};
use oopsmate_eval::Evaluator;

use crate::control::{SearchContext, SearchInterrupted};
use crate::selectivity::{lmr_reduction, should_reduce_lmr, NodeState};

use super::node::search_node;

#[inline(always)]
pub(super) fn search_child<E: Evaluator>(
    pos: &mut Position,
    depth: u8,
    node: NodeState,
    mv: Move,
    tt_move: Move,
    quiet: bool,
    reducible_capture: bool,
    history_score: i16,
    in_check: bool,
    searched_moves: usize,
    alpha: i32,
    beta: i32,
    try_null_window: bool,
    ctx: &mut SearchContext<'_>,
    evaluator: &mut E,
) -> Result<i32, SearchInterrupted> {
    let child_depth = depth - 1;

    if should_reduce_lmr(
        mv,
        tt_move,
        quiet,
        reducible_capture,
        in_check,
        depth,
        history_score,
        searched_moves,
        try_null_window,
    ) {
        #[cfg(feature = "telemetry")]
        {
            ctx.telemetry.lmr_attempts += 1;
        }
        let reduced_depth = child_depth.saturating_sub(lmr_reduction(
            depth,
            searched_moves,
            node,
            history_score,
            reducible_capture,
        ));
        let score = -search_node(
            pos,
            reduced_depth,
            node.child(false, -alpha - 1, -alpha),
            -alpha - 1,
            -alpha,
            ctx,
            evaluator,
        )?;
        if score <= alpha {
            #[cfg(feature = "telemetry")]
            {
                ctx.telemetry.lmr_cutoffs += 1;
            }
            return Ok(score);
        }
        #[cfg(feature = "telemetry")]
        {
            ctx.telemetry.lmr_researches += 1;
        }
    }

    if try_null_window {
        let score = -search_node(
            pos,
            child_depth,
            node.child(false, -alpha - 1, -alpha),
            -alpha - 1,
            -alpha,
            ctx,
            evaluator,
        )?;
        if score <= alpha || score >= beta {
            return Ok(score);
        }
    }

    Ok(-search_node(
        pos,
        child_depth,
        node.child(node.pv_node, -beta, -alpha),
        -beta,
        -alpha,
        ctx,
        evaluator,
    )?)
}
