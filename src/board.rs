//! Bitboard board state, legal move generation, move application, and display.

use std::fmt;

const NOT_FILE_A: u64 = 0xfefefefefefefefe;
const NOT_FILE_H: u64 = 0x7f7f7f7f7f7f7f7f;

const DIRECTIONS: [fn(u64) -> u64; 8] = [
    shift_north,
    shift_south,
    shift_east,
    shift_west,
    shift_north_east,
    shift_north_west,
    shift_south_east,
    shift_south_west,
];

/// The contents of a single board cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    /// No disc occupies the square.
    Empty,
    /// A black disc occupies the square.
    Black,
    /// A white disc occupies the square.
    White,
}

/// The side to move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Player {
    /// Black moves first in Reversi.
    Black,
    /// White moves second.
    White,
}

impl Player {
    /// Returns the opposing player.
    pub fn opponent(self) -> Self {
        match self {
            Self::Black => Self::White,
            Self::White => Self::Black,
        }
    }
}

impl fmt::Display for Player {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Black => write!(f, "Black"),
            Self::White => write!(f, "White"),
        }
    }
}

/// A Reversi board represented by two 64-bit bitboards and a side to move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Board {
    /// All black discs.
    pub black: u64,
    /// All white discs.
    pub white: u64,
    current_player: Player,
}

impl Board {
    /// Creates the standard initial Reversi position with Black to move.
    pub fn new() -> Self {
        let black = (1_u64 << 28) | (1_u64 << 35);
        let white = (1_u64 << 27) | (1_u64 << 36);

        Self {
            black,
            white,
            current_player: Player::Black,
        }
    }

    /// Creates a board from explicit bitboards and the current player.
    pub fn from_parts(black: u64, white: u64, current_player: Player) -> Self {
        Self {
            black,
            white,
            current_player,
        }
    }

    /// Returns whose turn it is.
    pub fn current_player(&self) -> Player {
        self.current_player
    }

    /// Returns the bitboard for the side to move.
    pub fn player_bits(&self) -> u64 {
        self.bits_for(self.current_player)
    }

    /// Returns the bitboard for the side not to move.
    pub fn opponent_bits(&self) -> u64 {
        self.bits_for(self.current_player.opponent())
    }

    /// Returns the bitboard for the requested player.
    pub fn bits_for(&self, player: Player) -> u64 {
        match player {
            Player::Black => self.black,
            Player::White => self.white,
        }
    }

    /// Prints a readable board with `a`-`h` file labels and `1`-`8` rank labels.
    pub fn display(&self) {
        println!("  a b c d e f g h");
        for row in 0..8 {
            print!("{} ", row + 1);
            for col in 0..8 {
                let bit = 1_u64 << (row * 8 + col);
                let marker = match self.get_cell(bit) {
                    Cell::Empty => '.',
                    Cell::Black => 'B',
                    Cell::White => 'W',
                };
                print!("{marker} ");
            }
            println!("{}", row + 1);
        }
        println!("  a b c d e f g h");
    }

    /// Returns the contents of a square represented as a one-bit bitboard.
    pub fn get_cell(&self, sq: u64) -> Cell {
        if self.black & sq != 0 {
            Cell::Black
        } else if self.white & sq != 0 {
            Cell::White
        } else {
            Cell::Empty
        }
    }

    /// Returns all legal move squares for the given player and opponent bitboards.
    pub fn legal_moves(&self, player: u64, opponent: u64) -> u64 {
        let empty = !(player | opponent);
        let mut moves = 0;

        for dir in DIRECTIONS {
            let mut candidates = dir(player) & opponent;
            for _ in 0..5 {
                candidates |= dir(candidates) & opponent;
            }
            moves |= dir(candidates) & empty;
        }

        moves
    }

    /// Returns legal moves for the current player, each as a one-bit bitboard.
    pub fn legal_moves_list(&self) -> Vec<u64> {
        bits_to_list(self.legal_moves(self.player_bits(), self.opponent_bits()))
    }

    /// Applies a legal move for the current player and returns the resulting board.
    pub fn apply_move(&self, square: u64) -> Self {
        let player = self.player_bits();
        let opponent = self.opponent_bits();
        let mut flips = 0;

        for dir in DIRECTIONS {
            flips |= flips_in_direction(square, player, opponent, dir);
        }

        let new_player = player | square | flips;
        let new_opponent = opponent & !flips;
        let mut next = *self;

        match self.current_player {
            Player::Black => {
                next.black = new_player;
                next.white = new_opponent;
            }
            Player::White => {
                next.white = new_player;
                next.black = new_opponent;
            }
        }

        next.current_player = self.current_player.opponent();
        next
    }

    /// Returns a board with only the side to move switched.
    pub fn pass(&self) -> Self {
        let mut next = *self;
        next.current_player = self.current_player.opponent();
        next
    }

    /// Returns true when neither side has any legal move.
    pub fn game_over(&self) -> bool {
        self.legal_moves(self.black, self.white) == 0
            && self.legal_moves(self.white, self.black) == 0
    }

    /// Returns the current disc counts as `(black_count, white_count)`.
    pub fn score(&self) -> (u32, u32) {
        (self.black.count_ones(), self.white.count_ones())
    }

    /// Returns the number of occupied squares.
    pub fn occupied_count(&self) -> u32 {
        (self.black | self.white).count_ones()
    }
}

/// Shifts a bitboard one rank toward the top of the displayed board.
pub fn shift_north(bits: u64) -> u64 {
    bits >> 8
}

/// Shifts a bitboard one rank toward the bottom of the displayed board.
pub fn shift_south(bits: u64) -> u64 {
    bits << 8
}

/// Shifts a bitboard one file to the right with edge wrap prevented.
pub fn shift_east(bits: u64) -> u64 {
    (bits & NOT_FILE_H) << 1
}

/// Shifts a bitboard one file to the left with edge wrap prevented.
pub fn shift_west(bits: u64) -> u64 {
    (bits & NOT_FILE_A) >> 1
}

/// Shifts a bitboard diagonally up and right with edge wrap prevented.
pub fn shift_north_east(bits: u64) -> u64 {
    (bits & NOT_FILE_H) >> 7
}

/// Shifts a bitboard diagonally up and left with edge wrap prevented.
pub fn shift_north_west(bits: u64) -> u64 {
    (bits & NOT_FILE_A) >> 9
}

/// Shifts a bitboard diagonally down and right with edge wrap prevented.
pub fn shift_south_east(bits: u64) -> u64 {
    (bits & NOT_FILE_H) << 9
}

/// Shifts a bitboard diagonally down and left with edge wrap prevented.
pub fn shift_south_west(bits: u64) -> u64 {
    (bits & NOT_FILE_A) << 7
}

/// Converts a coordinate such as `d3` into a one-bit bitboard square.
pub fn bit_from_coord(input: &str) -> Option<u64> {
    let bytes = input.trim().as_bytes();
    if bytes.len() != 2 {
        return None;
    }

    let file = bytes[0].to_ascii_lowercase();
    let rank = bytes[1];
    if !(b'a'..=b'h').contains(&file) || !(b'1'..=b'8').contains(&rank) {
        return None;
    }

    let col = u64::from(file - b'a');
    let row = u64::from(rank - b'1');
    Some(1_u64 << (row * 8 + col))
}

/// Converts a one-bit bitboard square into a coordinate such as `d3`.
pub fn coord_from_bit(square: u64) -> String {
    let index = square.trailing_zeros();
    let row = index / 8;
    let col = index % 8;
    let file = char::from(b'a' + col as u8);
    format!("{file}{}", row + 1)
}

/// Converts all set bits into individual one-bit bitboards.
pub fn bits_to_list(mut bits: u64) -> Vec<u64> {
    let mut list = Vec::new();
    while bits != 0 {
        let square = bits & bits.wrapping_neg();
        list.push(square);
        bits &= bits - 1;
    }
    list
}

fn flips_in_direction(square: u64, player: u64, opponent: u64, dir: fn(u64) -> u64) -> u64 {
    let mut cursor = dir(square);
    let mut captured = 0;

    while cursor != 0 && cursor & opponent != 0 {
        captured |= cursor;
        cursor = dir(cursor);
    }

    if cursor & player != 0 {
        captured
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_position_and_moves_are_correct() {
        let board = Board::new();
        assert_eq!(board.score(), (2, 2));

        let moves = board.legal_moves_list();
        let coords: Vec<String> = moves.into_iter().map(coord_from_bit).collect();
        assert_eq!(coords, ["d3", "c4", "f5", "e6"]);
    }

    #[test]
    fn applying_initial_move_flips_disc_and_switches_player() {
        let board = Board::new();
        let next = board.apply_move(bit_from_coord("d3").expect("valid test coordinate"));

        assert_eq!(next.current_player(), Player::White);
        assert_eq!(
            next.get_cell(bit_from_coord("d4").expect("valid test coordinate")),
            Cell::Black
        );
        assert_eq!(next.score(), (4, 1));
    }

    #[test]
    fn edge_shifts_do_not_wrap() {
        assert_eq!(
            shift_west(bit_from_coord("a1").expect("valid test coordinate")),
            0
        );
        assert_eq!(
            shift_east(bit_from_coord("h1").expect("valid test coordinate")),
            0
        );
        assert_eq!(
            shift_north(bit_from_coord("a1").expect("valid test coordinate")),
            0
        );
        assert_eq!(
            shift_south(bit_from_coord("h8").expect("valid test coordinate")),
            0
        );
    }
}
