# oopsmate-v2 roadmap

Fresh rewrite target for OopsMate.

## Project probe

Created with:
- `cargo new oopsmate-v2 --vcs none`

Current project state:
- path: `~/Projects/oopsmate-v2`
- crate type: Cargo workspace
- rust edition: `2024`
- external dependencies: `0`
- workspace members:
  - `crates/core`
  - `apps/oopsmate`

Current contents:
- `crates/core` contains the foundational board/state/types/move code
- `apps/oopsmate` is a thin binary crate wired against `core`
- root `Cargo.toml` owns workspace membership and build profiles

## Findings from the current oopsmate codebase

This roadmap is intentionally acting as the first map for the rewrite.
The focus is only on the minimum needed for a classical engine core that can:
- represent a board
- maintain legal state
- generate legal moves
- play legal moves

No eval work is planned here yet beyond keeping the future `PeSTO` route in mind.

---

## Rewrite goal

Build a lean engine core with:
- bitboards
- compact mailbox
- fixed-size state stacks
- no heap allocation in hot search / movegen paths
- zero external dependencies
- move generation designed for staged picking from day one

The guiding principle is:
- minimum viable feature
- maximum optimization

---

## What the current oopsmate already gets right

The current engine already has several good foundations worth carrying forward conceptually:

- bitboard-first board model
- mailbox alongside bitboards
- compact move encoding
- incremental make / unmake
- direct legal move generation using pin and check constraints
- attack-table-driven move generation
- no external crates from crates.io in the main engine path

So this rewrite should not reinvent everything.
It should keep the strong shape and remove friction.

---

## Main bottlenecks in current oopsmate

## 1. Board representation bottlenecks

Current shape in `oopsmate`:
- piece bitboards
- color bitboards
- mailbox as `[Option<(Piece, Color)>; 64]`
- side to move
- castling / ep / clocks
- incremental hash

### Problems

#### Mailbox representation is not as tight as it could be
`[Option<(Piece, Color)>; 64]` is readable, but not ideal for a fresh fast core.

Why it can be better:
- extra enum/tag handling
- less compact than a packed square code
- not ideal for hot-path piece lookup/update

### Better target
Use:
- piece bitboards
- color occupancies
- total occupancy
- mailbox as `[u8; 64]`
- packed square codes where `0 = empty`
- cached king squares: `[u8; 2]`

### Important current inefficiency
Current movegen repeatedly recomputes king square with bit scans like:
- `self.our(Piece::King).0.trailing_zeros()`

That appears in multiple movegen modules.
In the rewrite, king square should be stored directly.

---

## 2. State handling bottlenecks

Current shape in `oopsmate`:
- `history: Vec<GameState>`
- make pushes onto heap-backed history
- unmake pops
- repetition scans history linearly

### Problems

#### Heap-backed history in the hot path
Even with reserved capacity, this is the wrong long-term shape.
The rewrite should use fixed-capacity arrays.

#### Repetition detection is linear
Current repetition checking walks backward through history.
That is acceptable for a prototype, but not for the clean rewrite target.

#### Position ownership is too clone-friendly
The current engine clones position state at search handoff.
Not the main movegen bottleneck, but a sign the state model is not yet fully search-shaped.

### Better target
Use:
- fixed undo stack
- fixed repetition hash stack or ring
- explicit ply tracking
- no heap allocation during make/unmake/search

### Minimum rewrite state
The first version only needs:
- board state
- side to move
- castling rights
- ep square or ep file
- rule50 counter
- fullmove counter
- hash
- king squares
- undo stack
- repetition stack

---

## 3. Move generation bottlenecks

This is the most important area for the rewrite baseline.

Current `oopsmate` movegen is already legally-aware and constraint-based, which is good.
But the search path around it wastes work.

### What is good right now
- constraints are computed once per generation pass
- pinned pieces are handled directly
- check masks are used directly
- king legality is checked with attack queries
- en passant legality includes the discovered-rook check case

That core legality approach is worth preserving.

### Main bottlenecks

#### A. Eager full generation + eager full scoring
At search nodes, the engine typically:
- generates all legal moves
- copies them into another array
- scores all of them
- repeatedly selects the next best move

This is the biggest structural waste.

### Rewrite target
Move to staged move picking:
1. TT move
2. good captures/promotions
3. killers / countermove
4. quiets ordered by history
5. bad captures last

This reduces unnecessary generation and scoring work.

#### B. No dedicated evasion path
Current movegen uses pin/check masks, but still follows the same broad generation flow.
A rewrite should explicitly support:
- all legal moves
- captures/promotions
- quiets
- evasions

Special rule:
- in double check, generate king moves only

That is cleaner and faster.

#### C. Pawn generation is too piece-by-piece
Current pawn code loops pawn-by-pawn.
That is correct, but bulk bitboard generation is better for speed.

### Rewrite target for pawns
Split pawn generation into:
- bulk unpinned pawn pushes and captures via shifts
- pinned pawn handling as a slower path
- en passant legality path
- promotion path split clearly

This is one of the most valuable movegen cleanups.

#### D. Duplicate movegen logic
Current code splits logic across full-move and capture-only paths with duplication in:
- pawn generation
- slider generation

The rewrite should avoid that duplication where possible.
Not with abstraction for abstraction’s sake, but with a cleaner internal movegen structure.

#### E. Slider attack tables are heap-backed
Current `strikes` uses runtime-initialized `Vec<Vec<u64>>` for bishop/rook attack tables.
That works, but is not the cleanest storage format.

### Rewrite target
Prefer:
- flat attack table arrays
- square offsets into those arrays
- build-time generation or checked-in generated tables
- one explicit backend choice

If the target is x86_64 + BMI2 only, `pext` is acceptable.
If portability matters, magics are cleaner.

#### F. Missing movegen/search integration
Movegen is currently collector-oriented, not picker-oriented.
For the rewrite, movegen should serve the search path directly.

That means the API should be shaped for staged consumption, not only for “fill a full list”.

---

## Search tuning constants and tuning surface

Later on, the engine will need a single place for search/pruning/time-management constants so they can be tuned without hunting magic numbers across the codebase.

### Why this matters
As the search grows beyond basic alpha-beta, the following numbers will matter a lot:
- node polling interval
- aspiration window widths
- clock safety margins
- TT age / replacement thresholds
- LMR / null move / futility margins
- future pruning constants

If these stay scattered, tuning becomes messy and error-prone.

### Target shape
Add a dedicated search-constants module or config struct, then thread it through search entry points instead of hardcoding all values inside the hot code.

### Principle
Keep domain constants where they belong:
- eval tables in eval
- board / move encoding constants in core
- search / pruning / time constants in search

This becomes crucial later when tuning and benchmarking start in earnest.

---

## Move representation follow-up

A possible later optimization is to make `Move` niche-optimized so that `Option<Move>` packs down to 2 bytes instead of 4.

### Why it may matter
As search grows, `Option<Move>` will likely appear more often in:
- killer slots
- countermove tables
- search stack state
- root / PV helpers
- other search-memory structures

If `Move` can guarantee a non-zero representation, Rust can use `0` as the `None` niche for `Option<Move>`.

### Important caution
This is not an urgent optimization today, and it should not be done with a casual `unsafe` constructor that merely assumes no `a1 -> a1` quiet move is ever created.

If this change is made later, it should be done only with an airtight encoding / constructor design that preserves safety and keeps the representation explicit.

### Priority
Useful later as a structural cleanup and space optimization, but much lower priority than TT integration, move ordering, qsearch, and pruning.

---

## 4. Recommended rewrite architecture for the first milestone

The current chosen structure is a small workspace:
- `crates/core`
- `apps/oopsmate`

That keeps the board core isolated while avoiding over-splitting too early.

Suggested early layout:

- `crates/core/src/types.rs`
- `crates/core/src/board.rs`
- `crates/core/src/hash.rs`
- `crates/core/src/moves.rs`
- `crates/core/src/undo.rs`
- `crates/core/src/position.rs`
- `crates/core/src/fen.rs`
- `apps/oopsmate/src/main.rs`

Optional later split once the core is stable:
- `crates/attacks`
- `crates/movegen`
- `crates/search`
- `crates/nnue`
- `crates/uci`

### Why this shape
Because the first target is still correctness + speed of the board core,
but the workspace boundary keeps the engine binary thin and the core reusable.

---

## 5. Recommended board core for oopsmate-v2

### Board data
Use something close to:
- `pieces: [u64; 6]`
- `colors: [u64; 2]`
- `occ: u64`
- `board: [u8; 64]`
- `king_sq: [u8; 2]`
- `side_to_move: u8`
- `castling: u8`
- `ep_square: u8` or sentinel
- `rule50: u16`
- `fullmove: u16`
- `hash: u64`

### Undo data
Store only what is needed to reverse:
- previous castling
- previous ep
- previous rule50
- previous hash
- captured piece code
- moved piece metadata only if necessary

### Repetition data
Store hashes in a fixed stack/ring.
No heap-backed history vec.

---

## 6. Recommended movegen API for oopsmate-v2

Design for search from day one.

### Core generation modes
- `generate_all`
- `generate_captures_promotions`
- `generate_quiets`
- `generate_evasions`

### Internal analysis struct
Before generating, compute once:
- king square
- us occupancy
- them occupancy
- all occupancy
- pinned mask
- checkers mask
- check mask

This avoids re-deriving the same facts across generators.

### Pawn path
Use bulk generation for unpinned pawns:
- single pushes
- double pushes
- left captures
- right captures
- promotions

Use slower dedicated handling for:
- pinned pawns
- en passant
- special evasion cases

### Slider path
If staying BMI2-only:
- isolate `pext` in one backend module
- flat tables only

### King path
- store king squares directly
- use direct attacked-square queries
- special-case castling cleanly

---

## 7. Biggest practical wins over current oopsmate

If prioritizing impact for the rewrite baseline, the order should be:

### 1. Staged move picking
This is the biggest structural speed gain in the move generation/search pipeline.

### 2. Fixed-size state stacks
This removes heap-oriented state handling from the core.

### 3. Bulk pawn generation
This improves one of the busiest move classes significantly.

### 4. Dedicated evasion generation
This removes waste in check nodes and improves correctness structure.

### 5. Flat attack tables
This cleans up storage and cache behavior.

### 6. Cached king squares
Small change, good payoff, should be standard.

---

## 8. What not to build yet

Do not add yet:
- NNUE
- more crates than the current minimal workspace needs
- compile-time heuristic matrices
- builder tooling
- portability layers unless explicitly needed now
- piece-list complexity unless profiling proves it helps
- broad trait-heavy abstractions

The first clean version only needs to:
- load a board
- make/unmake moves
- generate legal moves
- pass perft
- eventually search shallowly and play legal chess

---

## 9. Proposed first implementation milestone

The first real milestone for `oopsmate-v2` should be:

1. define compact core types
2. build board representation
3. add FEN loader
4. add make/unmake
5. add attack generation
6. add legal move generation
7. add perft
8. validate correctness before any search work

### Success criteria
- zero external dependencies
- no heap allocation in movegen / make / unmake hot paths
- legal move generation only
- perft-correct
- structure ready for staged move picking

---

## 10. Current status of oopsmate-v2

Right now this project is intentionally empty except for the default Cargo scaffold.
That is good.

It means the rewrite can start from a clean surface without carrying over:
- repo clutter
- old eval paths
- feature-flag sprawl
- auxiliary crates
- legacy state plumbing

This is the correct place to build the new engine core.
