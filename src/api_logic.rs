//! Shared json api logic for the local web server and Vercel functions.

use crate::board::{bit_from_coord, coord_from_bit, Board, Cell, Player};
use crate::engine;

/// Default AI search depth used by the web front ends.
pub const DEFAULT_DEPTH: u32 = 7;

/// Input payload for a human move request.
pub struct MoveInput {
    /// Black bitboard from the client state.
    pub black: u64,
    /// White bitboard from the client state.
    pub white: u64,
    /// Side whose turn it is.
    pub current_player: Player,
    /// Human-controlled side.
    pub human_player: Player,
    /// Human move coordinate, such as `d3`.
    pub move_coord: String,
    /// AI search depth.
    pub depth: u32,
}

/// Creates a new game response as a json string.
pub fn new_game_json(depth: u32, human_player: Player) -> String {
    let (board, messages) = advance_ai_turns(Board::new(), depth, human_player);
    board_json(&board, &messages, None, human_player)
}

/// Applies a human move and returns the updated game response as a json string.
pub fn move_json(input: MoveInput) -> Result<String, String> {
    let board = Board::from_parts(input.black, input.white, input.current_player);
    if board.game_over() {
        return Ok(board_json(
            &board,
            &["The game is already over.".to_string()],
            None,
            input.human_player,
        ));
    }
    if board.current_player() != input.human_player {
        return Err("It is not the human player's turn.".to_string());
    }

    let square = bit_from_coord(&input.move_coord)
        .ok_or_else(|| "Move must be a coordinate like d3".to_string())?;
    let legal_mask = board.legal_moves(board.player_bits(), board.opponent_bits());
    if square & legal_mask == 0 {
        return Err(format!(
            "{} is not legal in this position.",
            input.move_coord
        ));
    }

    let mut messages = vec![format!("You played {}.", coord_from_bit(square))];
    let board = board.apply_move(square);
    let (board, mut ai_messages) = advance_ai_turns(board, input.depth, input.human_player);
    messages.append(&mut ai_messages);

    Ok(board_json(
        &board,
        &messages,
        Some(coord_from_bit(square)),
        input.human_player,
    ))
}

/// Parses a player name from client json/query input.
pub fn parse_player(value: &str) -> Option<Player> {
    match value {
        "Black" => Some(Player::Black),
        "White" => Some(Player::White),
        _ => None,
    }
}

fn advance_ai_turns(mut board: Board, depth: u32, human_player: Player) -> (Board, Vec<String>) {
    let mut messages = Vec::new();

    while !board.game_over() {
        let legal_moves = board.legal_moves_list();

        if board.current_player() == human_player {
            if legal_moves.is_empty() {
                messages.push(format!("{} has no legal moves and passes.", human_player));
                board = board.pass();
            } else {
                break;
            }
        } else if legal_moves.is_empty() {
            messages.push(format!(
                "{} has no legal moves and passes.",
                board.current_player()
            ));
            board = board.pass();
        } else {
            match engine::best_move(&board, depth) {
                Some(square) => {
                    messages.push(format!("AI played {}.", coord_from_bit(square)));
                    board = board.apply_move(square);
                }
                None => {
                    messages.push(format!(
                        "{} has no legal moves and passes.",
                        board.current_player()
                    ));
                    board = board.pass();
                }
            }
        }
    }

    if board.game_over() {
        let (black, white) = board.score();
        let outcome = if black > white {
            "Black wins."
        } else if white > black {
            "White wins."
        } else {
            "The game is a draw."
        };
        messages.push(format!(
            "Game over. Final score: Black {black}, White {white}. {outcome}"
        ));
    }

    (board, messages)
}

fn board_json(
    board: &Board,
    messages: &[String],
    human_move: Option<String>,
    human_player: Player,
) -> String {
    let (black_score, white_score) = board.score();
    let legal_moves = board
        .legal_moves_list()
        .into_iter()
        .map(coord_from_bit)
        .collect::<Vec<_>>();
    let cells = (0..64)
        .map(|index| {
            let square = 1_u64 << index;
            match board.get_cell(square) {
                Cell::Empty => "\"empty\"",
                Cell::Black => "\"black\"",
                Cell::White => "\"white\"",
            }
        })
        .collect::<Vec<_>>()
        .join(",");
    let legal_json = legal_moves
        .iter()
        .map(|coord| format!("\"{}\"", escape_json(coord)))
        .collect::<Vec<_>>()
        .join(",");
    let messages_json = messages
        .iter()
        .map(|message| format!("\"{}\"", escape_json(message)))
        .collect::<Vec<_>>()
        .join(",");
    let winner = if !board.game_over() {
        "null".to_string()
    } else if black_score > white_score {
        "\"Black\"".to_string()
    } else if white_score > black_score {
        "\"White\"".to_string()
    } else {
        "\"Draw\"".to_string()
    };
    let last_human_move = human_move
        .map(|coord| format!("\"{}\"", escape_json(&coord)))
        .unwrap_or_else(|| "null".to_string());

    format!(
        concat!(
            "{{",
            "\"black\":\"{}\",",
            "\"white\":\"{}\",",
            "\"currentPlayer\":\"{}\",",
            "\"humanPlayer\":\"{}\",",
            "\"cells\":[{}],",
            "\"legalMoves\":[{}],",
            "\"score\":{{\"black\":{},\"white\":{}}},",
            "\"gameOver\":{},",
            "\"winner\":{},",
            "\"messages\":[{}],",
            "\"lastHumanMove\":{}",
            "}}"
        ),
        board.black,
        board.white,
        board.current_player(),
        human_player,
        cells,
        legal_json,
        black_score,
        white_score,
        board.game_over(),
        winner,
        messages_json,
        last_human_move
    )
}

fn escape_json(value: &str) -> String {
    value
        .chars()
        .flat_map(|ch| match ch {
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '\n' => "\\n".chars().collect::<Vec<_>>(),
            '\r' => "\\r".chars().collect::<Vec<_>>(),
            '\t' => "\\t".chars().collect::<Vec<_>>(),
            _ => vec![ch],
        })
        .collect()
}
