#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SearchTelemetry {
    pub main_nodes: u64,
    pub q_nodes: u64,
    pub eval_calls: u64,
    pub tt_hits: u64,
    pub tt_cutoffs: u64,
    pub tt_static_eval_reuses: u64,
    pub razor_cutoffs: u64,
    pub rfp_cutoffs: u64,
    pub null_attempts: u64,
    pub null_cutoffs: u64,
    pub probcut_attempts: u64,
    pub probcut_qsearch_passes: u64,
    pub probcut_cutoffs: u64,
    pub futility_skips: u64,
    pub late_quiet_skips: u64,
    pub lmr_attempts: u64,
    pub lmr_cutoffs: u64,
    pub lmr_researches: u64,
}

impl SearchTelemetry {
    #[inline(always)]
    pub fn add(&mut self, other: Self) {
        self.main_nodes += other.main_nodes;
        self.q_nodes += other.q_nodes;
        self.eval_calls += other.eval_calls;
        self.tt_hits += other.tt_hits;
        self.tt_cutoffs += other.tt_cutoffs;
        self.tt_static_eval_reuses += other.tt_static_eval_reuses;
        self.razor_cutoffs += other.razor_cutoffs;
        self.rfp_cutoffs += other.rfp_cutoffs;
        self.null_attempts += other.null_attempts;
        self.null_cutoffs += other.null_cutoffs;
        self.probcut_attempts += other.probcut_attempts;
        self.probcut_qsearch_passes += other.probcut_qsearch_passes;
        self.probcut_cutoffs += other.probcut_cutoffs;
        self.futility_skips += other.futility_skips;
        self.late_quiet_skips += other.late_quiet_skips;
        self.lmr_attempts += other.lmr_attempts;
        self.lmr_cutoffs += other.lmr_cutoffs;
        self.lmr_researches += other.lmr_researches;
    }
}
