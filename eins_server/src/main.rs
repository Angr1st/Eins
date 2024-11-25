use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use axum::Json;
use axum::{response::IntoResponse, routing::get, Router};
use eins_lib::cards;
use eins_lib::game::Play;
use eins_lib::infrastructure::Player;
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt::Write;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Clone)]
struct App {
    players: Arc<RwLock<HashMap<Uuid, Player>>>,
}

impl App {
    fn new() -> Self {
        Self {
            players: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[derive(Deserialize)]
struct Registration {
    nick: String,
}

#[tokio::main]
async fn main() {
    let state = App::new();
    let app = Router::new()
        .route("/register", post(register).with_state(state.clone()))
        .route("/", get(index).with_state(state));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn register(
    State(state): State<App>,
    registration: Json<Registration>,
) -> Result<(StatusCode, Json<Player>), impl IntoResponse> {
    let registration: Registration = registration.0;
    if registration.nick.is_empty() || registration.nick.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Nick cannot be empty"));
    }

    {
        let read_lock = state.players.read().await;
        let existing_nick = read_lock
            .values()
            .any(|p| p.get_nick() == &registration.nick);
        if existing_nick {
            return Err((
                StatusCode::CONFLICT,
                "Nick already in use, please choose another!",
            ));
        }
    }

    let mut write_lock = state.players.write_owned().await;
    let player = Player::new(registration.nick);
    let id = player.get_id();
    write_lock.insert(id, player.clone());
    Ok((StatusCode::CREATED, Json(player)))
}

async fn index(State(_state): State<App>) -> impl IntoResponse {
    let game = eins_lib::test().expect("Test should succeed!");
    let first_player = game
        .get_players()
        .first()
        .expect("There is always a first player");
    // format!("{}", game);
    let mut output = String::new();
    writeln!(output, "Current card: {:?}", game.get_current_card()).unwrap();
    writeln!(output, "Current hand cards:").unwrap();
    let hand = first_player.get_held_cards();
    for card_ref in hand.iter() {
        let card = cards::get_card(card_ref);
        writeln!(output, "Card: {:?}", card).unwrap();
    }
    writeln!(output, "Choices of the current player.").unwrap();
    let choices = game.get_available_choices();
    match choices {
        Play::PossibleCards { options } => {
            for card_ref in options {
                let card = cards::get_card(card_ref);
                writeln!(output, "Card: {:?}", card).unwrap();
            }
        }
        Play::DrawCards { draw_amount } => {
            writeln!(output, "you have to draw: {:?}", draw_amount).unwrap()
        }
    }
    output
}
