use oopsmate_core::{Move, MoveKind, Piece, Position};
use oopsmate_eval::Evaluator;
use oopsmate_memory::Bound;
use oopsmate_movegen::{
    MAX_MOVES, MoveList, analyze, generate_captures_promotions_with_analysis, might_give_check,
    see_ge,
};

use crate::control::{SearchContext, SearchInterrupted};
use crate::picker::{MovePicker, TtMode};
use crate::qsearch::{NO_STATIC_EVAL, qsearch};
use crate::selectivity::{
    NodeState, can_use_selective_pruning, futility_margin, is_quiet_move,
    is_reducible_capture_lmr_move, lmr_reduction, needs_static_eval, null_move_depth,
    probcut_beta, probcut_depth, razor_margin, rfp_margin, should_apply_iir,
    should_prune_futility, should_prune_late_quiet, should_prune_reverse_futility,
    should_reduce_lmr, should_try_null_move, should_try_probcut, should_try_razoring,
};
use crate::tune::{PROBCUT_MIN_DEPTH, PVS_FULL_WINDOW_MOVES, scale_eval};
use crate::types::{is_mate_score, mate_score};

const LATE_BAD_CAPTURE_MIN_DEPTH: u8 = 3;
const LATE_BAD_CAPTURE_MAX_DEPTH: u8 = 8;
const LATE_BAD_CAPTURE_MIN_SEARCHED: usize = 4;
const LATE_BAD_CAPTURE_MAX_GAIN: i32 = 330;

fn try_probcut<E: Evaluator>(
    pos: &mut Position,
    analysis: &oopsmate_movegen::Analysis,
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

#[derive(Clone, Copy)]
struct CaptureHistoryRecord {
    moved: Piece,
    to: usize,
    captured: Piece,
}

impl CaptureHistoryRecord {
    const EMPTY: Self = Self {
        moved: Piece::Pawn,
        to: 0,
        captured: Piece::Pawn,
    };
}

pub(crate) fn search_node<E: Evaluator>(
    pos: &mut Position,
    mut depth: u8,
    node: NodeState,
    mut alpha: i32,
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
        raw_static_eval
            + ctx
                .history
                .correction_score(pos.side_to_move(), pos.pawn_hash())
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

    let tt_mode = if in_check {
        TtMode::ValidateInStage
    } else {
        TtMode::BlindTrust
    };
    let side = pos.side_to_move();
    let mut picker = MovePicker::new(pos, &analysis, tt_move, tt_mode);
    let mut best_move = Move::NULL;
    let mut best_score = i32::MIN / 2;
    let mut saw_legal_move = false;
    let mut searched_moves = 0usize;
    let mut searched_quiets = [Move::NULL; MAX_MOVES];
    let mut searched_quiet_count = 0usize;
    let mut searched_captures = [CaptureHistoryRecord::EMPTY; MAX_MOVES];
    let mut searched_capture_count = 0usize;

    while let Some(mv) = picker.next_move(pos, &analysis, &*ctx.history) {
        saw_legal_move = true;
        let quiet = is_quiet_move(mv);
        let maybe_check = quiet && might_give_check(pos, mv);
        let history_score = if quiet {
            ctx.history.score(side, mv)
        } else {
            0
        };
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

#[inline(always)]
#[must_use]
fn pack_static_eval(score: i32) -> i16 {
    debug_assert!(score >= i16::MIN as i32 && score <= i16::MAX as i32);
    score as i16
}

#[inline(always)]
fn should_update_correction(
    bound: Bound,
    in_check: bool,
    score: i32,
    raw_static_eval: i16,
) -> bool {
    bound == Bound::Exact
        && !in_check
        && raw_static_eval != NO_STATIC_EVAL
        && !is_mate_score(score)
        && !is_mate_score(i32::from(raw_static_eval))
}

#[inline(always)]
fn should_prune_late_bad_capture(
    pos: &Position,
    mv: Move,
    tt_move: Move,
    maybe_check: bool,
    depth: u8,
    searched_moves: usize,
    alpha: i32,
    static_eval: i32,
    can_selectively_prune: bool,
) -> bool {
    let kind = mv.kind();
    if !can_selectively_prune
        || depth < LATE_BAD_CAPTURE_MIN_DEPTH
        || depth > LATE_BAD_CAPTURE_MAX_DEPTH
        || searched_moves < LATE_BAD_CAPTURE_MIN_SEARCHED
        || mv == tt_move
        || maybe_check
        || kind == MoveKind::EnPassant
        || kind.is_promotion()
        || !kind.is_capture()
        || is_mate_score(alpha)
        || see_ge(pos, mv, 0)
    {
        return false;
    }

    let captured = pos.piece_at(mv.to()).map_or(Piece::Pawn, |(piece, _)| piece);
    static_eval + scale_eval(LATE_BAD_CAPTURE_MAX_GAIN.min(piece_value(captured))) <= alpha
}

#[inline(always)]
const fn piece_value(piece: Piece) -> i32 {
    match piece {
        Piece::Pawn => 100,
        Piece::Knight => 320,
        Piece::Bishop => 330,
        Piece::Rook => 500,
        Piece::Queen => 900,
        Piece::King => 0,
    }
}

fn capture_history_record(pos: &Position, mv: Move) -> Option<CaptureHistoryRecord> {
    let kind = mv.kind();
    if kind.is_promotion() || !(kind.is_capture() || kind == MoveKind::EnPassant) {
        return None;
    }

    let moved = pos
        .piece_at(mv.from())
        .map_or(Piece::Pawn, |(piece, _)| piece);
    let captured = if kind == MoveKind::EnPassant {
        Piece::Pawn
    } else {
        pos.piece_at(mv.to())
            .map_or(Piece::Pawn, |(piece, _)| piece)
    };

    Some(CaptureHistoryRecord {
        moved,
        to: mv.to().index(),
        captured,
    })
}

#[inline(always)]
fn search_child<E: Evaluator>(
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
        let reduced_depth =
            child_depth.saturating_sub(lmr_reduction(
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
