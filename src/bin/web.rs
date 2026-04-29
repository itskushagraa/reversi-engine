//! Minimal HTTP server for the React browser UI and Rust engine API.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

use reversi_engine::board::{bit_from_coord, coord_from_bit, Board, Cell, Player};
use reversi_engine::engine;

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 8080;
const DEFAULT_DEPTH: u32 = 7;

fn main() {
    let host = std::env::var("REVERSI_HOST").unwrap_or_else(|_| DEFAULT_HOST.to_string());
    let port = std::env::var("REVERSI_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(DEFAULT_PORT);
    let address = format!("{host}:{port}");

    let listener = match TcpListener::bind(&address) {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("Failed to bind {address}: {error}");
            return;
        }
    };

    println!("Reversi web UI running at http://{address}");

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => handle_connection(&mut stream),
            Err(error) => eprintln!("Connection failed: {error}"),
        }
    }
}

fn handle_connection(stream: &mut TcpStream) {
    let mut buffer = [0; 16_384];
    let bytes_read = match stream.read(&mut buffer) {
        Ok(0) => return,
        Ok(bytes_read) => bytes_read,
        Err(error) => {
            eprintln!("Failed to read request: {error}");
            return;
        }
    };

    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    let (head, body) = split_request(&request);
    let mut lines = head.lines();
    let request_line = match lines.next() {
        Some(line) => line,
        None => {
            write_response(stream, 400, "text/plain", "Bad request");
            return;
        }
    };

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("/");

    match (method, path) {
        ("GET", "/") => write_response(
            stream,
            200,
            "text/html; charset=utf-8",
            include_str!("../../web/index.html"),
        ),
        ("GET", "/app.js") => write_response(
            stream,
            200,
            "application/javascript; charset=utf-8",
            include_str!("../../web/app.js"),
        ),
        ("GET", "/style.css") => write_response(
            stream,
            200,
            "text/css; charset=utf-8",
            include_str!("../../web/style.css"),
        ),
        ("GET", path) if path.starts_with("/api/new") => {
            let depth = query_field(path, "depth")
                .and_then(|value| value.parse().ok())
                .unwrap_or(DEFAULT_DEPTH);
            let human_player = query_field(path, "human")
                .and_then(parse_player)
                .unwrap_or(Player::Black);
            let (board, messages) = advance_ai_turns(Board::new(), depth, human_player);
            write_json(
                stream,
                200,
                &board_json(&board, &messages, None, human_player),
            );
        }
        ("POST", "/api/move") => match handle_move(body) {
            Ok(response) => write_json(stream, 200, &response),
            Err(message) => write_json(
                stream,
                400,
                &format!("{{\"error\":\"{}\"}}", escape_json(&message)),
            ),
        },
        _ => write_response(stream, 404, "text/plain", "Not found"),
    }
}

fn split_request(request: &str) -> (&str, &str) {
    match request.split_once("\r\n\r\n") {
        Some((head, body)) => (head, body),
        None => (request, ""),
    }
}

fn handle_move(body: &str) -> Result<String, String> {
    let black = json_field(body, "black")
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or_else(|| "Missing or invalid black bitboard".to_string())?;
    let white = json_field(body, "white")
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or_else(|| "Missing or invalid white bitboard".to_string())?;
    let current_player = json_field(body, "currentPlayer")
        .and_then(parse_player)
        .ok_or_else(|| "Missing or invalid current player".to_string())?;
    let move_coord = json_field(body, "move").ok_or_else(|| "Missing move".to_string())?;
    let depth = json_field(body, "depth")
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(DEFAULT_DEPTH);
    let human_player = json_field(body, "humanPlayer")
        .and_then(parse_player)
        .unwrap_or(Player::Black);

    let board = Board::from_parts(black, white, current_player);
    if board.game_over() {
        return Ok(board_json(
            &board,
            &["The game is already over.".to_string()],
            None,
            human_player,
        ));
    }
    if board.current_player() != human_player {
        return Err("It is not the human player's turn.".to_string());
    }

    let square = bit_from_coord(&move_coord)
        .ok_or_else(|| "Move must be a coordinate like d3".to_string())?;
    let legal_mask = board.legal_moves(board.player_bits(), board.opponent_bits());
    if square & legal_mask == 0 {
        return Err(format!("{move_coord} is not legal in this position."));
    }

    let mut messages = vec![format!("You played {}.", coord_from_bit(square))];
    let board = board.apply_move(square);
    let (board, mut ai_messages) = advance_ai_turns(board, depth, human_player);
    messages.append(&mut ai_messages);

    Ok(board_json(
        &board,
        &messages,
        Some(coord_from_bit(square)),
        human_player,
    ))
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

fn query_field(path: &str, field: &str) -> Option<String> {
    let query = path.split_once('?')?.1;
    query.split('&').find_map(|part| {
        let (key, value) = part.split_once('=')?;
        if key == field {
            Some(value.to_string())
        } else {
            None
        }
    })
}

fn json_field(body: &str, field: &str) -> Option<String> {
    let needle = format!("\"{field}\"");
    let field_start = body.find(&needle)? + needle.len();
    let after_name = body[field_start..].trim_start();
    let after_colon = after_name.strip_prefix(':')?.trim_start();

    if let Some(rest) = after_colon.strip_prefix('"') {
        let mut value = String::new();
        let mut escaped = false;
        for ch in rest.chars() {
            if escaped {
                value.push(ch);
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                return Some(value);
            } else {
                value.push(ch);
            }
        }
        None
    } else {
        let value = after_colon
            .chars()
            .take_while(|ch| ch.is_ascii_digit())
            .collect::<String>();
        if value.is_empty() {
            None
        } else {
            Some(value)
        }
    }
}

fn parse_player(value: String) -> Option<Player> {
    match value.as_str() {
        "Black" => Some(Player::Black),
        "White" => Some(Player::White),
        _ => None,
    }
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

fn write_json(stream: &mut TcpStream, status: u16, body: &str) {
    write_response(stream, status, "application/json; charset=utf-8", body);
}

fn write_response(stream: &mut TcpStream, status: u16, content_type: &str, body: &str) {
    let status_text = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "OK",
    };
    let response = format!(
        "HTTP/1.1 {status} {status_text}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );

    if let Err(error) = stream.write_all(response.as_bytes()) {
        eprintln!("Failed to write response: {error}");
    }
}
