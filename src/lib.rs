pub const ENGINE_NAME: &str = "oopsmate-v2";

#[must_use]
pub const fn engine_name() -> &'static str {
    ENGINE_NAME
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_name_is_stable() {
        assert_eq!(engine_name(), "oopsmate-v2");
    }
}
