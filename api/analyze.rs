use http_body_util::BodyExt;
use reversi_engine::api_logic::{self, AnalyzeInput, DEFAULT_DEPTH};
use serde::Deserialize;
use vercel_runtime::{run, service_fn, Error, Request, Response};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnalyzePayload {
    black: String,
    white: String,
    current_player: String,
    depth: Option<u32>,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(service_fn(handler)).await
}

async fn handler(req: Request) -> Result<Response<String>, Error> {
    let body = req.into_body().collect().await?.to_bytes();
    let payload = match serde_json::from_slice::<AnalyzePayload>(&body) {
        Ok(p) => p,
        Err(_) => return json_response(400, "{\"error\":\"Invalid JSON payload\"}".to_string()),
    };

    let black = match payload.black.parse::<u64>() {
        Ok(v) => v,
        Err(_) => return json_response(400, error_json("Invalid black bitboard")),
    };
    let white = match payload.white.parse::<u64>() {
        Ok(v) => v,
        Err(_) => return json_response(400, error_json("Invalid white bitboard")),
    };
    let current_player = match api_logic::parse_player(&payload.current_player) {
        Some(p) => p,
        None => return json_response(400, error_json("Invalid current player")),
    };

    let body = api_logic::analyze_json(AnalyzeInput {
        black,
        white,
        current_player,
        depth: payload.depth.unwrap_or(DEFAULT_DEPTH),
    });
    json_response(200, body)
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
