pub const MATE_SCORE: i32 = 30_000;

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

#[inline(always)]
pub(crate) const fn normalize_score(score: i32, ply: u8) -> i32 {
    if score >= MATE_SCORE - 255 {
        score + ply as i32
    } else if score <= -MATE_SCORE + 255 {
        score - ply as i32
    } else {
        score
    }
}

#[inline(always)]
pub(crate) const fn denormalize_score(score: i32, ply: u8) -> i32 {
    if score >= MATE_SCORE - 255 {
        score - ply as i32
    } else if score <= -MATE_SCORE + 255 {
        score + ply as i32
    } else {
        score
    }
}
