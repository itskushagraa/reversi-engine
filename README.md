# Reversi Engine

this is a reversi engine written in rust because bitboards are fast and rust makes it hard to write total nonsense.

the board is stored as two `u64`s: one for black, one for white. every square is just a bit. legal moves, flips, scores, and search all build off that, so there is no sleepy 2d array crawling around the board.

the engine uses negamax with alpha beta pruning, iterative deepening, and a small transposition table. the eval is phase aware too, so it cares about mobility early, corners all the time, dangerous corner-adjacent squares when they matter, stability later, and raw discs near the end.

it is not pretending to be a perfect solver. it is just a clean, sharp reversi engine that plays a pretty annoying game at depth 7.

## what is in here

- bitboard move generation
- pure board updates, so applying a move returns a new board
- pass and game over handling
- alpha beta search
- iterative deepening
- transposition table
- phase based evaluation
- a terminal runner
- a browser runner that talks to the same rust engine

## how it works

black moves first. a move is legal if it traps at least one enemy disc in any of the 8 directions. the board code shifts bitboards north, south, east, west, and diagonally, with edge masks so bits do not wrap around like they had somewhere to be.

search is negamax. every move applies a board state, flips perspective, and searches deeper. alpha beta cuts off branches that are already worse than something we have found. iterative deepening searches depth 1, then 2, then 3, and keeps the last best move around for ordering.

the transposition table caches positions by the black and white bitboards. if the same shape shows up again at enough depth, the engine reuses the score instead of doing the same work twice.

## running it

terminal:

```bash
cargo run
```

browser:

```bash
cargo run --bin web
```

then open:

```text
http://127.0.0.1:8080
```

tests:

```bash
cargo test
```

## knobs

depth is the big one. depth 3 is quick, depth 7 is the normal setting, and higher depths start getting spicy depending on the position.

the browser version also lets you pick black or white, change depth, and scrub through old positions without changing the actual game.

## quirks

the eval is intentionally practical, not academic. corners are huge, mobility matters a lot before the board fills up, and disc count mostly waits until the late game because early disc count is bait.

the stability check is conservative. it counts corners and edge patterns that are clearly stable instead of trying to prove every possible stable interior disc. that keeps the code understandable and still strong enough to be rude.

no external crates. just rust, bitboards, and a search that does not waste time politely.
