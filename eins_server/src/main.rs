use axum::extract::State;
use axum::handler::Handler;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use axum::Json;
use axum::{response::IntoResponse, routing::get, Router};
use eins_lib::cards;
use eins_lib::game::{Create_Game, Game, GamePlay};
use eins_lib::infrastructure::Player;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt::{write, Display, Write};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::trace;
use tracing_subscriber::FmtSubscriber;
use uuid::Uuid;

#[derive(Clone)]
struct App {
    players: Arc<RwLock<HashMap<Uuid, Player>>>,
    setups: Arc<RwLock<HashMap<Uuid, GameSetup>>>,
    games: Arc<RwLock<HashMap<Uuid, Game>>>,
}

impl App {
    fn new() -> Self {
        Self {
            players: Arc::new(RwLock::new(HashMap::new())),
            setups: Arc::new(RwLock::new(HashMap::new())),
            games: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[derive(Deserialize)]
struct Registration {
    nick: String,
}

#[derive(Serialize)]
struct RegistrationResponse {
    player: Player,
}

#[derive(Deserialize)]
struct Setup {
    first_player: Uuid,
    initial_code: Option<String>,
}

struct GameSetup {
    creator: Uuid,
    players: Vec<Uuid>,
    code: Option<String>,
    state: GameSetupState,
}

impl GameSetup {
    fn new(creator: Uuid) -> Self {
        Self {
            creator,
            players: vec![],
            code: None,
            state: GameSetupState::Open,
        }
    }

    fn has_correct_code(&self, code: &str) -> bool {
        if let Some(current_code) = &self.code {
            return current_code == code;
        } else {
            return false;
        }
    }

    fn update(&mut self, new_code: Option<String>) {
        self.code = new_code;
        let new_state = if self.code.is_some() {
            match self.state {
                GameSetupState::Open => GameSetupState::Closed,
                GameSetupState::Closed => GameSetupState::Closed,
                GameSetupState::Full => GameSetupState::Full,
            }
        } else {
            match self.state {
                GameSetupState::Closed => GameSetupState::Open,
                GameSetupState::Open => GameSetupState::Open,
                GameSetupState::Full => GameSetupState::Full,
            }
        };
        self.state = new_state;
    }

    fn add_player(&mut self, player: Uuid) -> Result<(), String> {
        if self.state == GameSetupState::Full {
            return Err("Game is full".to_string());
        }
        self.players.push(player);
        if self.players.len() == eins_lib::game::MAX_NUMBER_OF_PLAYERS {
            self.state = GameSetupState::Full;
        }
        Ok(())
    }
}

#[derive(Copy, Clone, PartialEq, Serialize)]
enum GameSetupState {
    Open,
    Closed,
    Full,
}

impl Display for GameSetupState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "game setup state: ")?;
        match self {
            GameSetupState::Open => write!(f, "Open"),
            GameSetupState::Closed => write!(f, "Closed"),
            GameSetupState::Full => write!(f, "Full"),
        }
    }
}

#[derive(Serialize)]
struct SetupResponse {
    game_id: Uuid,
}

#[derive(Serialize)]
struct PlayersResponse {
    players: Vec<PlayerResponse>,
}

#[derive(Serialize)]
struct PlayerResponse {
    nick: String,
    id: Uuid,
}

impl From<&Player> for PlayerResponse {
    fn from(value: &Player) -> Self {
        Self {
            nick: value.get_nick().to_string(),
            id: value.get_id(),
        }
    }
}

#[derive(Serialize)]
struct GameSetupsResponse {
    setups: Vec<GameSetupResponse>,
}

#[derive(Serialize)]
struct GameSetupResponse {
    id: Uuid,
    number_of_players: usize,
    state: GameSetupState,
}

impl From<&GameSetup> for GameSetupResponse {
    fn from(value: &GameSetup) -> Self {
        Self {
            id: value.creator.clone(),
            number_of_players: value.players.len(),
            state: value.state,
        }
    }
}

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::new();

    tracing::subscriber::set_global_default(subscriber).unwrap();
    let state = App::new();
    let app = Router::new()
        .route("/player", get(get_players).with_state(state.clone()))
        .route("/player/register", post(register).with_state(state.clone()))
        .route(
            "/game/setup",
            get(get_game_setups).with_state(state.clone()),
        )
        .route("/game/setup", post(setup_game).with_state(state.clone()))
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
) -> Result<(StatusCode, Json<RegistrationResponse>), impl IntoResponse> {
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
    let response = RegistrationResponse { player };
    Ok((StatusCode::CREATED, Json(response)))
}

#[axum::debug_handler]
async fn setup_game(
    headers: HeaderMap,
    State(state): State<App>,
    setup: Json<Setup>,
) -> Result<(StatusCode, Json<SetupResponse>), impl IntoResponse> {
    let setup = setup.0;
    let header_code = headers.get("code");
    if let Some(code) = header_code {
        //Check if player exists with the same code
        {
            let read_lock = state.players.read_owned().await;
            let existing_user = read_lock.get(&setup.first_player);
            if let Some(user) = existing_user {
                if user.get_code() != code {
                    return Err((
                        StatusCode::UNAUTHORIZED,
                        "Code doesn't match the Players code.",
                    ));
                }
            } else {
                return Err((StatusCode::NOT_FOUND, "Player with id not found!"));
            }
        }
        //Check for existing game_setups created by same user
        {
            let read_lock = state.setups.read().await;
            let setup_option = read_lock.get(&setup.first_player);
            if setup_option.is_some() {
                return Err((
                    StatusCode::CONFLICT,
                    "Player has already started another game setup!",
                ));
            }
        }

        //Check for existing games created by same user
        {
            let read_lock = state.games.read_owned().await;
            let existing_games = read_lock
                .iter()
                .filter(|entry| entry.1.get_creator_id() == &setup.first_player);
            if existing_games.count() > 0 {
                return Err((
                    StatusCode::CONFLICT,
                    "Player has already started another game!",
                ));
            }
        }
        //Start a new game setup
        let mut game_setup = GameSetup::new(setup.first_player.clone());
        game_setup.update(setup.initial_code);
        let mut write_lock = state.setups.write_owned().await;
        write_lock.insert(setup.first_player.clone(), game_setup);
        Ok((
            StatusCode::CREATED,
            Json(SetupResponse {
                game_id: setup.first_player,
            }),
        ))
    } else {
        Err((StatusCode::UNAUTHORIZED, "Unauthorized"))
    }
}

async fn get_players(State(state): State<App>) -> impl IntoResponse {
    let read_lock = state.players.read_owned().await;
    let players: Vec<PlayerResponse> = read_lock.iter().map(|(_, value)| value.into()).collect();
    let count = players.len();
    tracing::info!("Currently {count} number of players");
    let response = PlayersResponse { players };
    Json(response)
}

async fn get_game_setups(State(state): State<App>) -> impl IntoResponse {
    let read_lock = state.setups.read_owned().await;
    let setups: Vec<GameSetupResponse> = read_lock.iter().map(|(_, value)| value.into()).collect();
    let count = setups.len();
    tracing::info!("Currently {count} number of game setups");
    let response = GameSetupsResponse { setups };
    Json(response)
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
        GamePlay::PossibleCards { options } => {
            for card_ref in options {
                let card = cards::get_card(card_ref);
                writeln!(output, "Card: {:?}", card).unwrap();
            }
        }
        GamePlay::DrawCards { draw_amount } => {
            writeln!(output, "you have to draw: {:?}", draw_amount).unwrap()
        }
    }
    output
}
