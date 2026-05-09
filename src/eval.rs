//! Position evaluation heuristics for Reversi search.

use crate::board::{Board, Player};

const CORNERS: [u64; 4] = [1_u64 << 0, 1_u64 << 7, 1_u64 << 56, 1_u64 << 63];

const CORNER_ADJACENT: [[u64; 3]; 4] = [
    [1_u64 << 1, 1_u64 << 8, 1_u64 << 9],
    [1_u64 << 6, 1_u64 << 14, 1_u64 << 15],
    [1_u64 << 48, 1_u64 << 49, 1_u64 << 57],
    [1_u64 << 54, 1_u64 << 55, 1_u64 << 62],
];

/// Evaluates the board using chess convention: positive = White winning, negative = Black winning.
/// Normalized to −10.0 … +10.0. Terminal positions are mapped via disc-count difference.
pub fn eval_for_display(board: &Board) -> f32 {
    if board.game_over() {
        let (black, white) = board.score();
        let diff = white as i32 - black as i32; // positive = White wins
        return (diff as f32 / 6.4).clamp(-10.0, 10.0);
    }

    let raw = evaluate(board);
    let from_white = match board.current_player() {
        Player::White => raw,
        Player::Black => -raw,
    };
    (from_white as f32 / 40.0).clamp(-10.0, 10.0)
}

/// Evaluates the board from the current player's perspective.
pub fn evaluate(board: &Board) -> i32 {
    if board.game_over() {
        return terminal_score(board);
    }

    let current = board.current_player();
    let opponent = current.opponent();
    let own = board.bits_for(current);
    let opp = board.bits_for(opponent);
    let occupied = board.occupied_count();

    let phase = Phase::from_occupied(occupied);
    let mobility = mobility_score(board, own, opp);
    let corners = corner_score(own, opp);
    let adjacent = corner_adjacency_score(own, opp);
    let stability = stability_score(own, opp);
    let discs = own.count_ones() as i32 - opp.count_ones() as i32;

    match phase {
        Phase::Early => mobility * 12 + corners * 40 + adjacent * 15 + stability * 4,
        Phase::Mid => mobility * 8 + corners * 45 + adjacent * 10 + stability * 10,
        Phase::Late => mobility * 2 + corners * 50 + stability * 14 + discs * 8,
    }
}

#[derive(Debug, Clone, Copy)]
enum Phase {
    Early,
    Mid,
    Late,
}

impl Phase {
    fn from_occupied(occupied: u32) -> Self {
        if occupied < 20 {
            Self::Early
        } else if occupied < 55 {
            Self::Mid
        } else {
            Self::Late
        }
    }
}

fn terminal_score(board: &Board) -> i32 {
    let (black, white) = board.score();
    let diff = match board.current_player() {
        Player::Black => black as i32 - white as i32,
        Player::White => white as i32 - black as i32,
    };

    diff * 10_000
}

fn mobility_score(board: &Board, own: u64, opp: u64) -> i32 {
    let own_moves = board.legal_moves(own, opp).count_ones() as i32;
    let opp_moves = board.legal_moves(opp, own).count_ones() as i32;
    own_moves - opp_moves
}

fn corner_score(own: u64, opp: u64) -> i32 {
    CORNERS
        .iter()
        .map(|corner| ownership_delta(*corner, own, opp))
        .sum()
}

fn corner_adjacency_score(own: u64, opp: u64) -> i32 {
    let mut score = 0;
    for (corner, adjacent_squares) in CORNERS.iter().zip(CORNER_ADJACENT.iter()) {
        if corner & (own | opp) != 0 {
            continue;
        }

        for square in adjacent_squares {
            score -= ownership_delta(*square, own, opp);
        }
    }
    score
}

fn stability_score(own: u64, opp: u64) -> i32 {
    stable_bits(own, opp).count_ones() as i32 - stable_bits(opp, own).count_ones() as i32
}

fn stable_bits(player: u64, opponent: u64) -> u64 {
    let occupied = player | opponent;
    let mut stable = player & corners_mask();

    for edge in EDGE_LINES {
        let edge_bits = edge.iter().fold(0, |acc, square| acc | square);
        if occupied & edge_bits == edge_bits {
            stable |= player & edge_bits;
            continue;
        }

        stable |= stable_from_edge_corner(player, edge.iter().copied());
        stable |= stable_from_edge_corner(player, edge.iter().rev().copied());
    }

    stable
}

fn stable_from_edge_corner<I>(player: u64, edge: I) -> u64
where
    I: IntoIterator<Item = u64>,
{
    let mut stable = 0;
    for square in edge {
        if square & player == 0 {
            break;
        }
        stable |= square;
    }
    stable
}

fn corners_mask() -> u64 {
    CORNERS.iter().fold(0, |acc, corner| acc | corner)
}

fn ownership_delta(square: u64, own: u64, opp: u64) -> i32 {
    if square & own != 0 {
        1
    } else if square & opp != 0 {
        -1
    } else {
        0
    }
}

const EDGE_LINES: [[u64; 8]; 4] = [
    [
        1_u64 << 0,
        1_u64 << 1,
        1_u64 << 2,
        1_u64 << 3,
        1_u64 << 4,
        1_u64 << 5,
        1_u64 << 6,
        1_u64 << 7,
    ],
    [
        1_u64 << 56,
        1_u64 << 57,
        1_u64 << 58,
        1_u64 << 59,
        1_u64 << 60,
        1_u64 << 61,
        1_u64 << 62,
        1_u64 << 63,
    ],
    [
        1_u64 << 0,
        1_u64 << 8,
        1_u64 << 16,
        1_u64 << 24,
        1_u64 << 32,
        1_u64 << 40,
        1_u64 << 48,
        1_u64 << 56,
    ],
    [
        1_u64 << 7,
        1_u64 << 15,
        1_u64 << 23,
        1_u64 << 31,
        1_u64 << 39,
        1_u64 << 47,
        1_u64 << 55,
        1_u64 << 63,
    ],
];
