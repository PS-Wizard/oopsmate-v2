mod entry;
mod score;
mod table;

pub use self::entry::{Bound, TtHit};
pub use self::score::{MATE_SCORE, is_mate_score, mate_in, mate_score};
pub use self::table::TranspositionTable;

#[cfg(test)]
mod tests;
