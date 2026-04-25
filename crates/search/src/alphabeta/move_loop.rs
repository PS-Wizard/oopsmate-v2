use oopsmate_core::{Move, Position};
use oopsmate_eval::Evaluator;
use oopsmate_memory::Bound;
use oopsmate_movegen::{might_give_check, Analysis, MAX_MOVES};

use crate::control::{SearchContext, SearchInterrupted};
use crate::picker::{MovePicker, TtMode};
use crate::selectivity::{
    can_use_selective_pruning, futility_margin, is_quiet_move, is_reducible_capture_lmr_move,
    should_prune_futility, should_prune_late_quiet, NodeState,
};
use crate::tune::PVS_FULL_WINDOW_MOVES;
use crate::types::mate_score;

use super::child::search_child;
use super::shared::{
    capture_history_record, should_prune_late_bad_capture, should_update_correction,
    CaptureHistoryRecord,
};

pub(super) fn search_moves<E: Evaluator>(
    pos: &mut Position,
    analysis: &Analysis,
    depth: u8,
    node: NodeState,
    alpha_orig: i32,
    mut alpha: i32,
    beta: i32,
    tt_move: Move,
    stored_static_eval: i16,
    static_eval: i32,
    in_check: bool,
    hash: u64,
    ctx: &mut SearchContext<'_>,
    evaluator: &mut E,
) -> Result<i32, SearchInterrupted> {
    let can_selectively_prune = can_use_selective_pruning(pos, node, alpha, beta, in_check);
    let tt_mode = if in_check {
        TtMode::ValidateInStage
    } else {
        TtMode::BlindTrust
    };
    let side = pos.side_to_move();
    let mut picker = MovePicker::new(pos, analysis, tt_move, tt_mode);
    let mut best_move = Move::NULL;
    let mut best_score = i32::MIN / 2;
    let mut saw_legal_move = false;
    let mut searched_moves = 0usize;
    let mut searched_quiets = [Move::NULL; MAX_MOVES];
    let mut searched_quiet_count = 0usize;
    let mut searched_captures = [CaptureHistoryRecord::EMPTY; MAX_MOVES];
    let mut searched_capture_count = 0usize;

    while let Some(mv) = picker.next_move(pos, analysis, &*ctx.history) {
        saw_legal_move = true;
        let quiet = is_quiet_move(mv);
        let maybe_check = quiet && might_give_check(pos, mv);
        let history_score = if quiet { ctx.history.score(side, mv) } else { 0 };
        let capture_record = capture_history_record(pos, mv);
        let reducible_capture = is_reducible_capture_lmr_move(mv);

        if should_prune_futility(
            mv,
            tt_move,
            quiet,
            maybe_check,
            depth,
            alpha,
            static_eval,
            can_selectively_prune,
        ) {
            let futility_score = static_eval + futility_margin(depth);
            if futility_score > best_score {
                best_score = futility_score;
            }
            #[cfg(feature = "telemetry")]
            {
                ctx.telemetry.futility_skips += 1;
            }
            continue;
        }

        if should_prune_late_quiet(
            mv,
            tt_move,
            quiet,
            maybe_check,
            depth,
            searched_moves,
            can_selectively_prune,
        ) {
            #[cfg(feature = "telemetry")]
            {
                ctx.telemetry.late_quiet_skips += 1;
            }
            continue;
        }

        if should_prune_late_bad_capture(
            pos,
            mv,
            tt_move,
            maybe_check,
            depth,
            searched_moves,
            alpha,
            static_eval,
            can_selectively_prune,
        ) {
            continue;
        }

        evaluator.push_move(pos, mv);
        pos.make_move(mv);
        let score = match search_child(
            pos,
            depth,
            node,
            mv,
            tt_move,
            quiet,
            reducible_capture,
            history_score,
            in_check,
            searched_moves,
            alpha,
            beta,
            searched_moves >= PVS_FULL_WINDOW_MOVES,
            ctx,
            evaluator,
        ) {
            Ok(score) => score,
            Err(err) => {
                pos.unmake_move(mv);
                evaluator.pop_move();
                return Err(err);
            }
        };
        pos.unmake_move(mv);
        evaluator.pop_move();
        searched_moves += 1;

        if quiet {
            searched_quiets[searched_quiet_count] = mv;
            searched_quiet_count += 1;
        } else if let Some(record) = capture_record {
            searched_captures[searched_capture_count] = record;
            searched_capture_count += 1;
        }

        if score > best_score {
            best_score = score;
            best_move = mv;
        }

        if score >= beta {
            if quiet {
                ctx.history.reward_quiet_cutoff(side, mv, depth);
                for &failed in &searched_quiets[..searched_quiet_count.saturating_sub(1)] {
                    ctx.history.penalize_quiet_fail(side, failed, depth);
                }
            } else if let Some(record) = capture_record {
                ctx.history.reward_capture_cutoff(
                    side,
                    record.moved,
                    record.to,
                    record.captured,
                    depth,
                );
                for failed in &searched_captures[..searched_capture_count.saturating_sub(1)] {
                    ctx.history.penalize_capture_fail(
                        side,
                        failed.moved,
                        failed.to,
                        failed.captured,
                        depth,
                    );
                }
            }
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

        if score > alpha {
            alpha = score;
        }
    }

    if !saw_legal_move {
        let score = if in_check { -mate_score(node.ply) } else { 0 };
        ctx.tt.store(
            hash,
            node.ply,
            Move::NULL,
            score,
            stored_static_eval,
            depth,
            Bound::Exact,
        );
        return Ok(score);
    }

    let bound = if best_score <= alpha_orig {
        Bound::Upper
    } else {
        Bound::Exact
    };
    if should_update_correction(bound, in_check, best_score, stored_static_eval) {
        ctx.history.update_correction(
            side,
            pos.pawn_hash(),
            best_score - i32::from(stored_static_eval),
            depth,
        );
    }
    ctx.tt.store(
        hash,
        node.ply,
        best_move,
        best_score,
        stored_static_eval,
        depth,
        bound,
    );

    Ok(best_score)
}
