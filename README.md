# oopsmate-v2

Clean-slate rewrite of OopsMate.

## Status

This project is the new home for the rewrite.
Current goals:
- zero external dependencies
- fast legal move generation
- clean board/state core
- library + binary crate layout

## Structure

- `crates/core` — core board/state/types/move foundation
- `apps/oopsmate` — thin binary crate
- `roadmap.md` — rewrite findings, bottlenecks, and implementation direction

## Build

```bash
cargo build
cargo test
cargo run -p oopsmate-v2
```

## Notes

- release builds are tuned in `Cargo.toml`
- local CPU tuning is set in `.cargo/config.toml`
- the detailed rewrite plan lives in `roadmap.md`
