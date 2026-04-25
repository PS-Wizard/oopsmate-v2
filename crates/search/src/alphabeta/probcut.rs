use oopsmate_core::{Move, MoveKind, Position};
use oopsmate_eval::Evaluator;
use oopsmate_movegen::{generate_captures_promotions_with_analysis, see_ge, Analysis, MoveList};

use crate::control::{SearchContext, SearchInterrupted};
use crate::qsearch::qsearch;
use crate::selectivity::{probcut_beta, probcut_depth, NodeState};

use super::node::search_node;

pub(super) fn try_probcut<E: Evaluator>(
    pos: &mut Position,
    analysis: &Analysis,
    depth: u8,
    node: NodeState,
    beta: i32,
    ctx: &mut SearchContext<'_>,
    evaluator: &mut E,
) -> Result<Option<(Move, i32)>, SearchInterrupted> {
    let prob_beta = probcut_beta(beta);
    let reduced_depth = probcut_depth(depth);
    let mut moves = MoveList::new();
    generate_captures_promotions_with_analysis(pos, analysis, &mut moves);

    for &mv in moves.as_slice() {
        if !probcut_candidate(pos, mv) {
            continue;
        }

        evaluator.push_move(pos, mv);
        pos.make_move(mv);

        let qscore = match qsearch(
            pos,
            node.ply + 1,
            -prob_beta,
            -prob_beta + 1,
            ctx,
            evaluator,
        ) {
            Ok(score) => -score,
            Err(err) => {
                pos.unmake_move(mv);
                evaluator.pop_move();
                return Err(err);
            }
        };

        let score = if qscore >= prob_beta && reduced_depth > 0 {
            #[cfg(feature = "telemetry")]
            {
                ctx.telemetry.probcut_qsearch_passes += 1;
            }
            match search_node(
                pos,
                reduced_depth,
                node.child(false, -prob_beta, -prob_beta + 1),
                -prob_beta,
                -prob_beta + 1,
                ctx,
                evaluator,
            ) {
                Ok(score) => -score,
                Err(err) => {
                    pos.unmake_move(mv);
                    evaluator.pop_move();
                    return Err(err);
                }
            }
        } else {
            qscore
        };

        pos.unmake_move(mv);
        evaluator.pop_move();

        if score >= prob_beta {
            #[cfg(feature = "telemetry")]
            {
                ctx.telemetry.probcut_cutoffs += 1;
            }
            return Ok(Some((mv, beta)));
        }
    }

    Ok(None)
}

#[inline(always)]
fn probcut_candidate(pos: &Position, mv: Move) -> bool {
    let kind = mv.kind();
    kind.is_promotion()
        || ((kind.is_capture() || kind == MoveKind::EnPassant) && see_ge(pos, mv, 0))
}
