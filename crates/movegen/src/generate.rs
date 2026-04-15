use oopsmate_core::Position;

use crate::analysis::{Analysis, analyze};
use crate::king;
use crate::leapers;
use crate::list::MoveList;
use crate::pawns;
use crate::sliders;
use crate::stage::{GenerationStage, is_evasions};

#[inline(always)]
pub fn generate_all(pos: &Position, list: &mut MoveList) {
    generate::<{ GenerationStage::All as u8 }>(pos, list);
}

#[inline(always)]
pub fn generate_captures_promotions(pos: &Position, list: &mut MoveList) {
    generate::<{ GenerationStage::CapturesPromotions as u8 }>(pos, list);
}

#[inline(always)]
pub fn generate_quiets(pos: &Position, list: &mut MoveList) {
    generate::<{ GenerationStage::Quiets as u8 }>(pos, list);
}

#[inline(always)]
pub fn generate_evasions(pos: &Position, list: &mut MoveList) {
    generate::<{ GenerationStage::Evasions as u8 }>(pos, list);
}

#[inline(always)]
pub(crate) fn generate<const STAGE: u8>(pos: &Position, list: &mut MoveList) {
    let analysis = analyze(pos);
    generate_with_analysis::<STAGE>(pos, &analysis, list);
}

#[inline(always)]
pub(crate) fn generate_with_analysis<const STAGE: u8>(
    pos: &Position,
    analysis: &Analysis,
    list: &mut MoveList,
) {
    list.clear();

    if is_evasions::<STAGE>() && !analysis.in_check() {
        return;
    }

    king::generate::<STAGE>(pos, analysis, list);

    if analysis.double_check() {
        return;
    }

    pawns::generate::<STAGE>(pos, analysis, list);
    leapers::generate::<STAGE>(pos, analysis, list);
    sliders::generate::<STAGE>(pos, analysis, list);
}
