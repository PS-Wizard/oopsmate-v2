use oopsmate_core::{Move, MoveKind, Piece, Position};
use oopsmate_eval::Evaluator;
use oopsmate_memory::Bound;
use oopsmate_movegen::{analyze, might_give_check};

use crate::control::{SearchContext, SearchInterrupted};
use crate::picker::{MovePicker, TtMode};
use crate::qsearch::{qsearch, NO_STATIC_EVAL};
use crate::tune::{
    FUTILITY_MARGIN_1, FUTILITY_MARGIN_2, FUTILITY_MARGIN_3, FUTILITY_MARGIN_4, FUTILITY_MARGIN_5,
    FUTILITY_MARGIN_6, FUTILITY_MARGIN_7, FUTILITY_MAX_DEPTH, LATE_QUIET_PRUNE_MAX_DEPTH,
    LATE_QUIET_PRUNE_MIN_DEPTH, LATE_QUIET_PRUNE_MOVE_MULT, LATE_QUIET_PRUNE_MOVE_OFFSET,
    LMR_FULL_DEPTH_MOVES, LMR_MIN_DEPTH, LMR_REDUCTION, NULL_MOVE_MIN_DEPTH, NULL_MOVE_REDUCTION,
    PVS_FULL_WINDOW_MOVES, RAZOR_MARGIN_1, RAZOR_MARGIN_2, RAZOR_MARGIN_3, RAZOR_MAX_DEPTH,
    RFP_MARGIN_1, RFP_MARGIN_2, RFP_MARGIN_3, RFP_MARGIN_4, RFP_MARGIN_5, RFP_MARGIN_6,
    RFP_MARGIN_7, RFP_MAX_DEPTH,
};
use crate::types::{is_mate_score, mate_score};

#[derive(Clone, Copy)]
pub(crate) struct NodeState {
    pub(crate) ply: u8,
    pub(crate) pv_node: bool,
    pub(crate) cut_node: bool,
}

impl NodeState {
    #[inline(always)]
    #[must_use]
    pub(crate) const fn new(ply: u8, pv_node: bool, alpha: i32, beta: i32) -> Self {
        Self {
            ply,
            pv_node,
            cut_node: beta == alpha + 1,
        }
    }

    #[inline(always)]
    #[must_use]
    const fn child(self, pv_node: bool, alpha: i32, beta: i32) -> Self {
        Self::new(self.ply + 1, pv_node, alpha, beta)
    }
}

pub(crate) fn search_node<E: Evaluator>(
    pos: &mut Position,
    depth: u8,
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

    if pos.rule50() >= 100 || pos.is_repetition() {
        return Ok(0);
    }

    let hash = pos.hash();
    let alpha_orig = alpha;
    let mut tt_move = Move::NULL;

    if let Some(hit) = ctx.tt.probe(hash, node.ply) {
        tt_move = hit.best_move;
        if hit.depth >= depth {
            match hit.bound {
                Bound::Exact => return Ok(hit.score),
                Bound::Lower if hit.score >= beta => return Ok(hit.score),
                Bound::Upper if hit.score <= alpha => return Ok(hit.score),
                _ => {}
            }
        }
    }

    let analysis = analyze(pos);
    let in_check = analysis.in_check();
    let can_selectively_prune = can_use_selective_pruning(pos, node, alpha, beta, in_check);
    let static_eval = if needs_static_eval(depth, can_selectively_prune) {
        evaluator.evaluate(pos)
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
            return Ok(score);
        }
    }

    if should_prune_reverse_futility(depth, static_eval, beta, can_selectively_prune) {
        let score = static_eval - rfp_margin(depth);
        ctx.tt.store(
            hash,
            node.ply,
            Move::NULL,
            score,
            NO_STATIC_EVAL,
            depth,
            Bound::Lower,
        );
        return Ok(score);
    }

    if should_try_null_move(depth, static_eval, beta, can_selectively_prune) {
        evaluator.push_null_move();
        pos.make_null_move();
        let score = match search_node(
            pos,
            depth - 1 - NULL_MOVE_REDUCTION,
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
            ctx.tt.store(
                hash,
                node.ply,
                Move::NULL,
                beta,
                NO_STATIC_EVAL,
                depth,
                Bound::Lower,
            );
            return Ok(beta);
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

    while let Some(mv) = picker.next_move(pos, &analysis, &*ctx.history) {
        saw_legal_move = true;
        let quiet = is_quiet_move(mv);
        let maybe_check = quiet && might_give_check(pos, mv);

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
            maybe_check,
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

        if score > best_score {
            best_score = score;
            best_move = mv;
        }

        if score >= beta {
            if quiet {
                ctx.history.reward_quiet_cutoff(side, mv, depth);
            }
            ctx.tt.store(
                hash,
                node.ply,
                mv,
                score,
                NO_STATIC_EVAL,
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
            NO_STATIC_EVAL,
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
    ctx.tt.store(
        hash,
        node.ply,
        best_move,
        best_score,
        NO_STATIC_EVAL,
        depth,
        bound,
    );

    Ok(best_score)
}

#[inline(always)]
fn can_use_selective_pruning(
    pos: &Position,
    node: NodeState,
    alpha: i32,
    beta: i32,
    in_check: bool,
) -> bool {
    node.cut_node
        && !in_check
        && !is_mate_score(alpha)
        && !is_mate_score(beta)
        && has_non_pawn_material(pos)
}

#[inline(always)]
const fn needs_static_eval(depth: u8, can_selectively_prune: bool) -> bool {
    can_selectively_prune
        && (depth >= NULL_MOVE_MIN_DEPTH
            || depth <= FUTILITY_MAX_DEPTH
            || depth <= RFP_MAX_DEPTH
            || depth <= RAZOR_MAX_DEPTH)
}

#[inline(always)]
fn should_try_razoring(
    depth: u8,
    static_eval: i32,
    alpha: i32,
    can_selectively_prune: bool,
) -> bool {
    can_selectively_prune && depth <= RAZOR_MAX_DEPTH && static_eval + razor_margin(depth) < alpha
}

#[inline(always)]
const fn razor_margin(depth: u8) -> i32 {
    match depth {
        1 => RAZOR_MARGIN_1,
        2 => RAZOR_MARGIN_2,
        _ => RAZOR_MARGIN_3,
    }
}

#[inline(always)]
fn should_prune_reverse_futility(
    depth: u8,
    static_eval: i32,
    beta: i32,
    can_selectively_prune: bool,
) -> bool {
    can_selectively_prune && depth <= RFP_MAX_DEPTH && static_eval - rfp_margin(depth) >= beta
}

#[inline(always)]
const fn rfp_margin(depth: u8) -> i32 {
    match depth {
        1 => RFP_MARGIN_1,
        2 => RFP_MARGIN_2,
        3 => RFP_MARGIN_3,
        4 => RFP_MARGIN_4,
        5 => RFP_MARGIN_5,
        6 => RFP_MARGIN_6,
        _ => RFP_MARGIN_7,
    }
}

#[inline(always)]
fn should_try_null_move(
    depth: u8,
    static_eval: i32,
    beta: i32,
    can_selectively_prune: bool,
) -> bool {
    can_selectively_prune
        && depth > NULL_MOVE_REDUCTION
        && depth >= NULL_MOVE_MIN_DEPTH
        && static_eval >= beta
}

#[inline(always)]
fn should_prune_futility(
    mv: Move,
    tt_move: Move,
    quiet: bool,
    maybe_check: bool,
    depth: u8,
    alpha: i32,
    static_eval: i32,
    can_selectively_prune: bool,
) -> bool {
    can_selectively_prune
        && depth <= FUTILITY_MAX_DEPTH
        && mv != tt_move
        && quiet
        && static_eval + futility_margin(depth) <= alpha
        && !maybe_check
}

#[inline(always)]
fn should_prune_late_quiet(
    mv: Move,
    tt_move: Move,
    quiet: bool,
    maybe_check: bool,
    depth: u8,
    searched_moves: usize,
    can_selectively_prune: bool,
) -> bool {
    can_selectively_prune
        && depth >= LATE_QUIET_PRUNE_MIN_DEPTH
        && depth <= LATE_QUIET_PRUNE_MAX_DEPTH
        && searched_moves >= late_quiet_prune_moves(depth)
        && mv != tt_move
        && quiet
        && !maybe_check
}

#[inline(always)]
const fn late_quiet_prune_moves(depth: u8) -> usize {
    depth as usize * LATE_QUIET_PRUNE_MOVE_MULT - LATE_QUIET_PRUNE_MOVE_OFFSET
}

#[inline(always)]
const fn futility_margin(depth: u8) -> i32 {
    match depth {
        1 => FUTILITY_MARGIN_1,
        2 => FUTILITY_MARGIN_2,
        3 => FUTILITY_MARGIN_3,
        4 => FUTILITY_MARGIN_4,
        5 => FUTILITY_MARGIN_5,
        6 => FUTILITY_MARGIN_6,
        _ => FUTILITY_MARGIN_7,
    }
}

#[inline(always)]
fn has_non_pawn_material(pos: &Position) -> bool {
    let board = pos.board();
    let side = pos.side_to_move();
    let pieces =
        board.color_bb(side) & !(board.piece_bb(Piece::Pawn) | board.piece_bb(Piece::King));
    pieces != 0
}

#[inline(always)]
fn search_child<E: Evaluator>(
    pos: &mut Position,
    depth: u8,
    node: NodeState,
    mv: Move,
    tt_move: Move,
    quiet: bool,
    maybe_check: bool,
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
        maybe_check,
        in_check,
        depth,
        node,
        searched_moves,
        try_null_window,
    ) {
        let reduced_depth = child_depth.saturating_sub(LMR_REDUCTION);
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
            return Ok(score);
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

#[inline(always)]
fn should_reduce_lmr(
    mv: Move,
    tt_move: Move,
    quiet: bool,
    maybe_check: bool,
    in_check: bool,
    depth: u8,
    node: NodeState,
    searched_moves: usize,
    try_null_window: bool,
) -> bool {
    try_null_window
        && !node.pv_node
        && !in_check
        && depth >= LMR_MIN_DEPTH
        && searched_moves >= LMR_FULL_DEPTH_MOVES
        && mv != tt_move
        && quiet
        && !maybe_check
}

#[inline(always)]
const fn is_quiet_move(mv: Move) -> bool {
    matches!(
        mv.kind(),
        MoveKind::Quiet | MoveKind::DoublePush | MoveKind::Castle
    )
}
