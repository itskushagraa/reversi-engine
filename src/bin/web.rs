//! Minimal HTTP server for local development of the React UI and Rust engine API.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

use reversi_engine::api_logic::{self, AnalyzeInput, MoveInput, StartFromPositionInput, DEFAULT_DEPTH};
use reversi_engine::board::Player;

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 8080;

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
            include_str!("../../public/index.html"),
        ),
        ("GET", "/app.js") => write_response(
            stream,
            200,
            "application/javascript; charset=utf-8",
            include_str!("../../public/app.js"),
        ),
        ("GET", "/style.css") => write_response(
            stream,
            200,
            "text/css; charset=utf-8",
            include_str!("../../public/style.css"),
        ),
        ("GET", path) if path.starts_with("/api/new") => {
            let depth = query_field(path, "depth")
                .and_then(|value| value.parse().ok())
                .unwrap_or(DEFAULT_DEPTH);
            let human_player = query_field(path, "human")
                .and_then(|value| api_logic::parse_player(&value))
                .unwrap_or(Player::Black);
            write_json(stream, 200, &api_logic::new_game_json(depth, human_player));
        }
        ("POST", "/api/move") => match handle_move(body) {
            Ok(response) => write_json(stream, 200, &response),
            Err(message) => write_json(
                stream,
                400,
                &format!("{{\"error\":\"{}\"}}", escape_json(&message)),
            ),
        },
        ("POST", "/api/analyze") => match handle_analyze(body) {
            Ok(response) => write_json(stream, 200, &response),
            Err(message) => write_json(
                stream,
                400,
                &format!("{{\"error\":\"{}\"}}", escape_json(&message)),
            ),
        },
        ("POST", "/api/start") => match handle_start(body) {
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
        .and_then(|value| api_logic::parse_player(&value))
        .ok_or_else(|| "Missing or invalid current player".to_string())?;
    let human_player = json_field(body, "humanPlayer")
        .and_then(|value| api_logic::parse_player(&value))
        .unwrap_or(Player::Black);
    let move_coord = json_field(body, "move").ok_or_else(|| "Missing move".to_string())?;
    let depth = json_field(body, "depth")
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(DEFAULT_DEPTH);

    api_logic::move_json(MoveInput {
        black,
        white,
        current_player,
        human_player,
        move_coord,
        depth,
    })
}

fn handle_analyze(body: &str) -> Result<String, String> {
    let black = json_field(body, "black")
        .and_then(|v| v.parse::<u64>().ok())
        .ok_or_else(|| "Missing or invalid black bitboard".to_string())?;
    let white = json_field(body, "white")
        .and_then(|v| v.parse::<u64>().ok())
        .ok_or_else(|| "Missing or invalid white bitboard".to_string())?;
    let current_player = json_field(body, "currentPlayer")
        .and_then(|v| api_logic::parse_player(&v))
        .ok_or_else(|| "Missing or invalid current player".to_string())?;
    let depth = json_field(body, "depth")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(DEFAULT_DEPTH);

    Ok(api_logic::analyze_json(AnalyzeInput { black, white, current_player, depth }))
}

fn handle_start(body: &str) -> Result<String, String> {
    let black = json_field(body, "black")
        .and_then(|v| v.parse::<u64>().ok())
        .ok_or_else(|| "Missing or invalid black bitboard".to_string())?;
    let white = json_field(body, "white")
        .and_then(|v| v.parse::<u64>().ok())
        .ok_or_else(|| "Missing or invalid white bitboard".to_string())?;
    let current_player = json_field(body, "currentPlayer")
        .and_then(|v| api_logic::parse_player(&v))
        .ok_or_else(|| "Missing or invalid current player".to_string())?;
    let human_player = json_field(body, "humanPlayer")
        .and_then(|v| api_logic::parse_player(&v))
        .unwrap_or(reversi_engine::board::Player::Black);
    let depth = json_field(body, "depth")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(DEFAULT_DEPTH);

    Ok(api_logic::start_from_position_json(StartFromPositionInput {
        black,
        white,
        current_player,
        human_player,
        depth,
    }))
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
