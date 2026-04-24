//! Terminal game loop for Human Black vs AI White Reversi.

use std::io::{self, Write};

use reversi_engine::board::{bit_from_coord, coord_from_bit, Board, Player};
use reversi_engine::engine;

const AI_DEPTH: u32 = 7;

fn main() {
    let mut board = Board::new();

    println!("Reversi: Human is Black, AI is White.");
    println!("Enter moves as coordinates such as d3. Type quit to exit.");

    while !board.game_over() {
        board.display();
        let (black, white) = board.score();
        println!("Score: Black {black}, White {white}");
        println!("Turn: {}", board.current_player());

        let legal_moves = board.legal_moves_list();
        if legal_moves.is_empty() {
            println!(
                "{} has no legal moves and must pass.",
                board.current_player()
            );
            board = board.pass();
            continue;
        }

        match board.current_player() {
            Player::Black => match read_human_move(&board) {
                Some(square) => board = board.apply_move(square),
                None => {
                    println!("Goodbye.");
                    return;
                }
            },
            Player::White => {
                println!("AI is thinking at depth {AI_DEPTH}...");
                match engine::best_move(&board, AI_DEPTH) {
                    Some(square) => {
                        println!("AI plays {}", coord_from_bit(square));
                        board = board.apply_move(square);
                    }
                    None => {
                        println!("AI has no legal moves and must pass.");
                        board = board.pass();
                    }
                }
            }
        }
    }

    board.display();
    let (black, white) = board.score();
    println!("Final score: Black {black}, White {white}");
    if black > white {
        println!("Black wins.");
    } else if white > black {
        println!("White wins.");
    } else {
        println!("Draw.");
    }
}

/// Reads, parses, and validates a human move, returning `None` when the player quits.
fn read_human_move(board: &Board) -> Option<u64> {
    let legal_moves = board.legal_moves_list();
    let legal_mask = legal_moves.iter().fold(0, |acc, mv| acc | mv);
    let legal_labels: Vec<String> = legal_moves.iter().copied().map(coord_from_bit).collect();

    loop {
        println!("Legal moves: {}", legal_labels.join(", "));
        print!("Your move: ");
        if let Err(error) = io::stdout().flush() {
            eprintln!("Failed to flush prompt: {error}");
        }

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => return None,
            Ok(_) => {}
            Err(error) => {
                eprintln!("Failed to read input: {error}");
                continue;
            }
        }

        let trimmed = input.trim();
        if trimmed.eq_ignore_ascii_case("quit") || trimmed.eq_ignore_ascii_case("q") {
            return None;
        }

        match bit_from_coord(trimmed) {
            Some(square) if square & legal_mask != 0 => return Some(square),
            Some(_) => println!("{trimmed} is not legal in this position."),
            None => println!("Enter a coordinate from a1 through h8, or quit."),
        }
    }
}
