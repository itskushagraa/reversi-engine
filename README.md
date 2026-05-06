# Reversi (Othello) Engine

[![Rust](https://img.shields.io/badge/Rust-2021-b7410e?logo=rust&logoColor=white)](https://www.rust-lang.org/) ![Engine](https://img.shields.io/badge/engine-negamax%20%2B%20alpha--beta-2f855a) ![Board](https://img.shields.io/badge/board-u64%20bitboards-2563eb) ![Search](https://img.shields.io/badge/search-iterative%20deepening-7c3aed) ![Web UI](https://img.shields.io/badge/web-local%20HTTP%20UI-0891b2) ![Tests](https://img.shields.io/badge/tests-cargo%20test-16a34a) [![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

![Reversi web UI](docs/assets/reversi-web-ui.png)

a reversi engine written in rust.

the board is represented with two `u64` bitboards, one for black and one for white. move generation uses directional bit shifts with edge masks to avoid wraparound across files.

the search is negamax with alpha beta pruning. `best_move` uses iterative deepening and keeps the best move from the previous depth around for move ordering. there is also a basic transposition table keyed by the black and white bitboards.

the evaluation function changes by game phase:

- early game: mobility and avoiding bad corner-adjacent squares
- middle game: mobility, corners, and stable edge/corner discs
- late game: corners, stability, and disc count

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

## notes

the default ai depth is 7.

the web version uses the same rust engine through a small local http server. it also has side selection, depth selection, and move history viewing.

## license

mit.
