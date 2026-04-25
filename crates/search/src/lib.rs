mod alphabeta;
mod control;
mod limits;
mod picker;
mod qsearch;
mod root;
mod selectivity;
#[cfg(feature = "telemetry")]
mod telemetry;
mod tune;
mod types;

pub use limits::{ClockLimits, SearchLimits};
pub use root::{search, search_with_reporter};
#[cfg(feature = "telemetry")]
pub use telemetry::SearchTelemetry;
pub use types::{is_mate_score, mate_in, SearchResult, MATE_SCORE};

#[cfg(test)]
mod tests;
