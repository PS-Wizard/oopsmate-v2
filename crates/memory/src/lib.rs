mod history;
mod tt;

pub use history::HistoryTable;
pub use tt::{Bound, MATE_SCORE, TranspositionTable, TtHit, is_mate_score, mate_in, mate_score};

#[derive(Debug)]
pub struct SearchMemory {
    pub tt: TranspositionTable,
    pub history: HistoryTable,
}

impl SearchMemory {
    #[must_use]
    pub fn new(tt_mebibytes: usize) -> Self {
        Self {
            tt: TranspositionTable::new(tt_mebibytes),
            history: HistoryTable::new(),
        }
    }

    pub fn clear(&mut self) {
        self.tt.clear();
        self.history.clear();
    }

    pub fn new_search(&mut self) {
        self.tt.new_search();
    }
}
