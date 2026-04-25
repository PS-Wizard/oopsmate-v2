include!(concat!(env!("OUT_DIR"), "/tune.rs"));

#[inline(always)]
#[must_use]
pub(crate) const fn scale_eval(value: i32) -> i32 {
    value * EVAL_SCORE_SCALE
}
