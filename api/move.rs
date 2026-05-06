use http_body_util::BodyExt;
use reversi_engine::api_logic::{self, MoveInput, DEFAULT_DEPTH};
use reversi_engine::board::Player;
use serde::Deserialize;
use vercel_runtime::{run, service_fn, Error, Request, Response};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MovePayload {
    black: String,
    white: String,
    current_player: String,
    human_player: Option<String>,
    #[serde(rename = "move")]
    move_coord: String,
    depth: Option<u32>,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(service_fn(handler)).await
}

async fn handler(req: Request) -> Result<Response<String>, Error> {
    let body = req.into_body().collect().await?.to_bytes();
    let payload = match serde_json::from_slice::<MovePayload>(&body) {
        Ok(payload) => payload,
        Err(_) => return json_response(400, "{\"error\":\"Invalid json payload\"}".to_string()),
    };

    let input = match move_input(payload) {
        Ok(input) => input,
        Err(message) => return json_response(400, error_json(&message)),
    };

    match api_logic::move_json(input) {
        Ok(body) => json_response(200, body),
        Err(message) => json_response(400, error_json(&message)),
    }
}

fn move_input(payload: MovePayload) -> Result<MoveInput, String> {
    let black = payload
        .black
        .parse::<u64>()
        .map_err(|_| "Missing or invalid black bitboard".to_string())?;
    let white = payload
        .white
        .parse::<u64>()
        .map_err(|_| "Missing or invalid white bitboard".to_string())?;
    let current_player = api_logic::parse_player(&payload.current_player)
        .ok_or_else(|| "Missing or invalid current player".to_string())?;
    let human_player = payload
        .human_player
        .as_deref()
        .and_then(api_logic::parse_player)
        .unwrap_or(Player::Black);

    Ok(MoveInput {
        black,
        white,
        current_player,
        human_player,
        move_coord: payload.move_coord,
        depth: payload.depth.unwrap_or(DEFAULT_DEPTH),
    })
}

fn json_response(status: u16, body: String) -> Result<Response<String>, Error> {
    Ok(Response::builder()
        .status(status)
        .header("content-type", "application/json; charset=utf-8")
        .body(body)?)
}

fn error_json(message: &str) -> String {
    serde_json::json!({ "error": message }).to_string()
}
