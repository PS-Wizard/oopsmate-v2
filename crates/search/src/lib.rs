mod control;
mod limits;
mod minimax;
mod root;
mod types;

pub use limits::{ClockLimits, SearchLimits};
pub use root::{search, search_with_reporter};
pub use types::{MATE_SCORE, SearchResult, is_mate_score, mate_in};

#[cfg(test)]
mod tests;
