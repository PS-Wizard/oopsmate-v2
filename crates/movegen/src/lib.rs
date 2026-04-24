// This crate generates legal moves directly from precomputed constraints rather
// than producing a pseudo-legal list and filtering it afterward.
mod analysis;
mod attacks;
mod generate;
mod king;
mod leapers;
mod legal;
mod list;
mod pawns;
mod perft;
mod sliders;
mod stage;
mod util;

pub use analysis::{Analysis, analyze};
pub use attacks::{is_square_attacked, is_square_attacked_with_occ};
pub use generate::{
    generate_all, generate_all_with_analysis, generate_captures_promotions,
    generate_captures_promotions_with_analysis, generate_evasions, generate_evasions_with_analysis,
    generate_quiets, generate_quiets_with_analysis,
};
pub use legal::{is_legal, is_pseudo_legal};
pub use list::{MAX_MOVES, MoveList};
pub use perft::{
    KIWIPETE, PERFT_CASES, POSITION_3, POSITION_4, POSITION_5, POSITION_6, PerftCase, STARTPOS,
    perft,
};
pub use stage::GenerationStage;

#[cfg(test)]
mod tests;
