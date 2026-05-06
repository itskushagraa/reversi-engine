use reversi_engine::api_logic::{self, DEFAULT_DEPTH};
use reversi_engine::board::Player;
use vercel_runtime::{run, service_fn, Error, Request, Response};

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(service_fn(handler)).await
}

async fn handler(req: Request) -> Result<Response<String>, Error> {
    let depth = query_field(&req, "depth")
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_DEPTH);
    let human_player = query_field(&req, "human")
        .and_then(|value| api_logic::parse_player(&value))
        .unwrap_or(Player::Black);

    json_response(200, api_logic::new_game_json(depth, human_player))
}

fn query_field(req: &Request, field: &str) -> Option<String> {
    req.uri().query()?.split('&').find_map(|part| {
        let (key, value) = part.split_once('=')?;
        if key == field {
            Some(value.to_string())
        } else {
            None
        }
    })
}

fn json_response(status: u16, body: String) -> Result<Response<String>, Error> {
    Ok(Response::builder()
        .status(status)
        .header("content-type", "application/json; charset=utf-8")
        .body(body)?)
}
