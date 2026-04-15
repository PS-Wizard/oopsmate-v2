use oopsmate_core::Move;

pub const MATE_SCORE: i32 = 30_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchResult {
    pub best_move: Option<Move>,
    pub score: i32,
    pub depth: u8,
    pub nodes: u64,
    pub time_ms: u64,
}

#[inline(always)]
#[must_use]
pub const fn mate_score(ply: u8) -> i32 {
    MATE_SCORE - ply as i32
}

#[inline(always)]
#[must_use]
pub const fn is_mate_score(score: i32) -> bool {
    score >= MATE_SCORE - 255 || score <= -MATE_SCORE + 255
}

#[must_use]
pub fn mate_in(score: i32) -> Option<i32> {
    if !is_mate_score(score) {
        return None;
    }

    let plies = MATE_SCORE - score.abs();
    let moves = (plies + 1) / 2;
    Some(if score > 0 { moves } else { -moves })
}
