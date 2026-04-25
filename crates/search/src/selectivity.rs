use oopsmate_core::{Move, MoveKind, Piece, Position};

use crate::tune::{
    FUTILITY_MARGIN_1, FUTILITY_MARGIN_2, FUTILITY_MARGIN_3, FUTILITY_MARGIN_4, FUTILITY_MARGIN_5,
    FUTILITY_MARGIN_6, FUTILITY_MARGIN_7, FUTILITY_MAX_DEPTH, IIR_CUT_MIN_DEPTH, IIR_PV_MIN_DEPTH,
    LATE_QUIET_PRUNE_MAX_DEPTH, LATE_QUIET_PRUNE_MIN_DEPTH, LATE_QUIET_PRUNE_MOVE_MULT,
    LATE_QUIET_PRUNE_MOVE_OFFSET, LMR_FULL_DEPTH_MOVES, LMR_HISTORY_BAD, LMR_HISTORY_GOOD,
    LMR_MIN_DEPTH, NULL_MOVE_MIN_DEPTH, PROBCUT_MARGIN, PROBCUT_MIN_DEPTH, PROBCUT_REDUCTION,
    RAZOR_MARGIN_1, RAZOR_MARGIN_2, RAZOR_MARGIN_3, RAZOR_MAX_DEPTH, RFP_MARGIN_1, RFP_MARGIN_2,
    RFP_MARGIN_3, RFP_MARGIN_4, RFP_MARGIN_5, RFP_MARGIN_6, RFP_MARGIN_7, RFP_MAX_DEPTH,
};
use crate::types::is_mate_score;

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
    pub(crate) const fn child(self, pv_node: bool, alpha: i32, beta: i32) -> Self {
        Self::new(self.ply + 1, pv_node, alpha, beta)
    }
}

#[inline(always)]
pub(crate) fn can_use_selective_pruning(
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
pub(crate) const fn needs_static_eval(depth: u8, can_selectively_prune: bool) -> bool {
    can_selectively_prune
        && (depth >= NULL_MOVE_MIN_DEPTH
            || depth <= FUTILITY_MAX_DEPTH
            || depth <= RFP_MAX_DEPTH
            || depth <= RAZOR_MAX_DEPTH)
}

#[inline(always)]
pub(crate) fn should_try_razoring(
    depth: u8,
    static_eval: i32,
    alpha: i32,
    can_selectively_prune: bool,
) -> bool {
    can_selectively_prune && depth <= RAZOR_MAX_DEPTH && static_eval + razor_margin(depth) < alpha
}

#[inline(always)]
pub(crate) const fn razor_margin(depth: u8) -> i32 {
    match depth {
        1 => RAZOR_MARGIN_1,
        2 => RAZOR_MARGIN_2,
        _ => RAZOR_MARGIN_3,
    }
}

#[inline(always)]
pub(crate) fn should_prune_reverse_futility(
    depth: u8,
    static_eval: i32,
    beta: i32,
    can_selectively_prune: bool,
) -> bool {
    can_selectively_prune && depth <= RFP_MAX_DEPTH && static_eval - rfp_margin(depth) >= beta
}

#[inline(always)]
pub(crate) const fn rfp_margin(depth: u8) -> i32 {
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
pub(crate) fn should_try_null_move(
    depth: u8,
    static_eval: i32,
    beta: i32,
    can_selectively_prune: bool,
) -> bool {
    can_selectively_prune && depth >= NULL_MOVE_MIN_DEPTH && static_eval >= beta
}

#[inline(always)]
pub(crate) fn should_apply_iir(depth: u8, node: NodeState, tt_move: Move) -> bool {
    tt_move == Move::NULL
        && ((node.pv_node && depth >= IIR_PV_MIN_DEPTH)
            || (node.cut_node && depth >= IIR_CUT_MIN_DEPTH))
}

#[inline(always)]
pub(crate) fn should_try_probcut(
    depth: u8,
    node: NodeState,
    beta: i32,
    in_check: bool,
    static_eval: i32,
) -> bool {
    !node.pv_node
        && !in_check
        && !is_mate_score(beta)
        && depth >= PROBCUT_MIN_DEPTH
        && static_eval >= beta - PROBCUT_MARGIN
}

#[inline(always)]
pub(crate) const fn probcut_beta(beta: i32) -> i32 {
    beta + PROBCUT_MARGIN
}

#[inline(always)]
pub(crate) const fn probcut_depth(depth: u8) -> u8 {
    depth.saturating_sub(PROBCUT_REDUCTION)
}

#[inline(always)]
pub(crate) fn null_move_depth(depth: u8, static_eval: i32, beta: i32) -> u8 {
    let eval_excess = static_eval.saturating_sub(beta).max(0);
    let reduction_bonus = (eval_excess / 200).min(4) as u8;
    let reduction = depth / 3 + 3 + reduction_bonus;
    depth.saturating_sub(reduction)
}

#[inline(always)]
pub(crate) fn should_prune_futility(
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
pub(crate) fn should_prune_late_quiet(
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
pub(crate) const fn late_quiet_prune_moves(depth: u8) -> usize {
    depth as usize * LATE_QUIET_PRUNE_MOVE_MULT - LATE_QUIET_PRUNE_MOVE_OFFSET
}

#[inline(always)]
pub(crate) const fn futility_margin(depth: u8) -> i32 {
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
pub(crate) fn should_reduce_lmr(
    mv: Move,
    tt_move: Move,
    quiet: bool,
    in_check: bool,
    depth: u8,
    _history_score: i16,
    searched_moves: usize,
    try_null_window: bool,
) -> bool {
    try_null_window
        && !in_check
        && depth >= LMR_MIN_DEPTH
        && searched_moves >= LMR_FULL_DEPTH_MOVES
        && mv != tt_move
        && quiet
}

#[inline(always)]
pub(crate) const fn lmr_reduction(
    depth: u8,
    searched_moves: usize,
    node: NodeState,
    history_score: i16,
) -> u8 {
    let mut reduction = if depth >= 12 && searched_moves >= 16 {
        4
    } else if depth >= 8 && searched_moves >= 8 {
        3
    } else if depth >= 5 && searched_moves >= 4 {
        2
    } else {
        1
    };

    if history_score >= LMR_HISTORY_GOOD && reduction > 1 {
        reduction -= 1;
    }

    if history_score <= LMR_HISTORY_BAD {
        reduction += 1;
    }

    if node.pv_node && reduction > 1 {
        reduction -= 1;
    }

    if reduction >= depth {
        depth - 1
    } else {
        reduction
    }
}

#[inline(always)]
pub(crate) const fn is_quiet_move(mv: Move) -> bool {
    matches!(
        mv.kind(),
        MoveKind::Quiet | MoveKind::DoublePush | MoveKind::Castle
    )
}

#[inline(always)]
fn has_non_pawn_material(pos: &Position) -> bool {
    let board = pos.board();
    let side = pos.side_to_move();
    let pieces =
        board.color_bb(side) & !(board.piece_bb(Piece::Pawn) | board.piece_bb(Piece::King));
    pieces != 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use oopsmate_core::Square;

    #[test]
    fn good_history_reduces_lmr() {
        let non_pv = NodeState::new(1, false, -1, 0);
        assert_eq!(lmr_reduction(8, 8, non_pv, 128), 2);
    }

    #[test]
    fn bad_history_increases_lmr() {
        let non_pv = NodeState::new(1, false, -1, 0);
        assert_eq!(lmr_reduction(8, 8, non_pv, -64), 4);
    }

    #[test]
    fn probcut_uses_configured_reduction() {
        assert_eq!(probcut_depth(5), 1);
        assert_eq!(probcut_depth(8), 4);
    }

    #[test]
    fn iir_uses_node_kind_thresholds() {
        let pv = NodeState::new(1, true, -10, 10);
        let cut = NodeState::new(1, false, -1, 0);
        let all = NodeState::new(1, false, -10, 10);

        assert!(should_apply_iir(IIR_PV_MIN_DEPTH, pv, Move::NULL));
        assert!(should_apply_iir(IIR_CUT_MIN_DEPTH, cut, Move::NULL));
        assert!(!should_apply_iir(IIR_CUT_MIN_DEPTH - 1, cut, Move::NULL));
        assert!(!should_apply_iir(
            IIR_PV_MIN_DEPTH,
            pv,
            Move::new(Square::from_raw(0), Square::from_raw(1), MoveKind::Quiet),
        ));
        assert!(!should_apply_iir(16, all, Move::NULL));
    }
}
