//! Minimax engine using iterative deepening, alpha-beta pruning, and a transposition table.

use std::collections::HashMap;

use crate::board::Board;
use crate::eval;

const INF: i32 = 1_000_000_000;

/// A cached alpha-beta search result.
#[derive(Debug, Clone, Copy)]
pub struct TranspositionEntry {
    /// Search depth this entry was proven at.
    pub depth: u32,
    /// Evaluation score from the side-to-move perspective at this node.
    pub score: i32,
    /// Bound type for alpha-beta reuse.
    pub flag: EntryFlag,
}

/// The kind of bound stored in a transposition table entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryFlag {
    /// The cached score is exact.
    Exact,
    /// The cached score is a lower bound.
    LowerBound,
    /// The cached score is an upper bound.
    UpperBound,
}

/// Finds the best move for the current player up to `depth` plies using iterative deepening.
pub fn best_move(board: &Board, depth: u32) -> Option<u64> {
    let legal = board.legal_moves_list();
    if legal.is_empty() {
        return None;
    }

    let mut table = HashMap::new();
    let mut previous_best = None;

    for search_depth in 1..=depth {
        let ordered = ordered_moves(legal.clone(), previous_best);
        let mut best_score = -INF;
        let mut best_at_depth = None;
        let mut alpha = -INF;

        for mv in ordered {
            let score = -negamax_with_table(
                &board.apply_move(mv),
                search_depth - 1,
                -INF,
                -alpha,
                &mut table,
            );
            if score > best_score {
                best_score = score;
                best_at_depth = Some(mv);
            }
            alpha = alpha.max(best_score);
        }

        if best_at_depth.is_some() {
            previous_best = best_at_depth;
        }
    }

    previous_best
}

/// Searches a board with negamax and alpha-beta pruning.
pub fn negamax(board: &Board, depth: u32, alpha: i32, beta: i32) -> i32 {
    let mut table = HashMap::new();
    negamax_with_table(board, depth, alpha, beta, &mut table)
}

fn negamax_with_table(
    board: &Board,
    depth: u32,
    mut alpha: i32,
    beta: i32,
    table: &mut HashMap<(u64, u64), TranspositionEntry>,
) -> i32 {
    let original_alpha = alpha;

    if depth == 0 || board.game_over() {
        return eval::evaluate(board);
    }

    let table_key = (board.player_bits(), board.opponent_bits());

    if let Some(entry) = table.get(&table_key) {
        if entry.depth >= depth {
            let mut table_beta = beta;
            match entry.flag {
                EntryFlag::Exact => return entry.score,
                EntryFlag::LowerBound => alpha = alpha.max(entry.score),
                EntryFlag::UpperBound => table_beta = table_beta.min(entry.score),
            }

            if alpha >= table_beta {
                return entry.score;
            }
        }
    }

    let legal_moves = board.legal_moves_list();
    let score = if legal_moves.is_empty() {
        -negamax_with_table(&board.pass(), depth - 1, -beta, -alpha, table)
    } else {
        let mut best = -INF;
        for mv in order_moves_by_heuristic(board, legal_moves) {
            let child_score =
                -negamax_with_table(&board.apply_move(mv), depth - 1, -beta, -alpha, table);
            best = best.max(child_score);
            alpha = alpha.max(child_score);
            if alpha >= beta {
                break;
            }
        }
        best
    };

    let flag = if score <= original_alpha {
        EntryFlag::UpperBound
    } else if score >= beta {
        EntryFlag::LowerBound
    } else {
        EntryFlag::Exact
    };

    table.insert(table_key, TranspositionEntry { depth, score, flag });

    score
}

fn ordered_moves(mut moves: Vec<u64>, first: Option<u64>) -> Vec<u64> {
    if let Some(best) = first {
        if let Some(index) = moves.iter().position(|mv| *mv == best) {
            moves.swap(0, index);
        }
    }
    moves
}

fn order_moves_by_heuristic(board: &Board, moves: Vec<u64>) -> Vec<u64> {
    let mut scored: Vec<(i32, u64)> = moves
        .into_iter()
        .map(|mv| {
            let before = board.opponent_bits().count_ones() as i32;
            let after = board.apply_move(mv).player_bits().count_ones() as i32;
            let flips = before - after;
            let corner_bonus = if mv & corner_mask() != 0 { 100 } else { 0 };
            (corner_bonus + flips, mv)
        })
        .collect();
    scored.sort_by(|left, right| right.0.cmp(&left.0));
    scored.into_iter().map(|(_, mv)| mv).collect()
}

fn corner_mask() -> u64 {
    (1_u64 << 0) | (1_u64 << 7) | (1_u64 << 56) | (1_u64 << 63)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_finds_an_initial_move() {
        let board = Board::new();
        assert!(best_move(&board, 3).is_some());
    }
}
