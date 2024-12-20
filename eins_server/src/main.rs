use axum::extract::State;
use axum::handler::Handler;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use axum::Json;
use axum::{response::IntoResponse, routing::get, Router};
use eins_lib::cards;
use eins_lib::game::{Game, GamePlay};
use eins_lib::infrastructure::Player;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{Display, Write};
use std::sync::Arc;
use tokio::sync::RwLock;
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

#[derive(Serialize)]
struct MeResponse {
    player: Player,
}

#[derive(Deserialize)]
struct Setup {
    game_code: Option<String>,
}

struct GameSetup {
    creator: Uuid,
    players: Vec<GameSetupPlayer>,
    code: Option<String>,
    state: GameSetupState,
}

struct GameSetupPlayer {
    id: Uuid,
    nick: String,
}

enum JoinGameSetupResult {
    Success,
    WrongCode,
    SetupFull,
}

impl GameSetup {
    fn new(player: &Player) -> Self {
        let player_id = player.get_id();
        Self {
            creator: player_id.clone(),
            players: vec![GameSetupPlayer {
                id: player_id,
                nick: player.get_nick().to_string(),
            }],
            code: None,
            state: GameSetupState::Open,
        }
    }

    fn join<'a>(&mut self, player: &Player, code: Option<&'a str>) -> JoinGameSetupResult {
        if self.has_correct_code(code) {
            return self.add_player(player);
        }
        JoinGameSetupResult::WrongCode
    }

    fn has_correct_code<'a>(&self, code: Option<&'a str>) -> bool {
        if let Some(current_code) = &self.code {
            if let Some(check_code) = code {
                return current_code == check_code;
            }
            return false;
        } else {
            return code.is_none();
        }
    }

    fn update(&mut self, new_code: Option<String>) {
        if let Some(code) = new_code {
            if code.is_empty() || code.trim().is_empty() {
                self.code = None;
            } else {
                self.code = Some(code);
            }
        } else {
            self.code = new_code;
        }
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

    fn add_player(&mut self, player: &Player) -> JoinGameSetupResult {
        if self.state == GameSetupState::Full {
            return JoinGameSetupResult::SetupFull;
        }
        let player = GameSetupPlayer {
            id: player.get_id(),
            nick: player.get_nick().to_string(),
        };
        self.players.push(player);
        if self.players.len() == eins_lib::game::MAX_NUMBER_OF_PLAYERS {
            self.state = GameSetupState::Full;
        }
        JoinGameSetupResult::Success
    }

    fn remove_player(&mut self, player: Uuid) -> bool {
        if !self.players.iter().find(|gsp| gsp.id == player).is_none() {
            return false;
        }
        self.players.retain(|el| el.id != player);
        if self.state == GameSetupState::Full {
            if self.code.is_some() {
                self.state = GameSetupState::Closed;
            } else {
                self.state = GameSetupState::Open;
            }
        }
        return true;
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

#[derive(Deserialize)]
struct GameSetupUpdate {
    game_code: Option<String>,
}

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::new();

    tracing::subscriber::set_global_default(subscriber).unwrap();
    let state = App::new();
    let app = Router::new()
        .route("/player", get(get_players).with_state(state.clone()))
        .route("/player/me", get(get_me).with_state(state.clone()))
        .route("/player/register", post(register).with_state(state.clone()))
        .route(
            "/game/setup",
            get(get_game_setups).with_state(state.clone()),
        )
        .route(
            "/game/setup/me",
            get(get_game_setup).with_state(state.clone()),
        )
        .route("/game/setup", post(setup_game).with_state(state.clone()))
        .route(
            "/game/setup/update",
            post(update_code).with_state(state.clone()),
        )
        .route(
            "/game/setup/join",
            post(join_setup).with_state(state.clone()),
        )
        .route(
            "/game/setup/leave",
            post(leave_setup).with_state(state.clone()),
        );

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

async fn get_me(
    headers: HeaderMap,
    State(state): State<App>,
) -> Result<(StatusCode, Json<MeResponse>), impl IntoResponse> {
    let header_code = headers.get("code");
    if let Some(code) = header_code {
        let code = code.to_str();
        if code.is_err() {
            return Err((StatusCode::BAD_REQUEST, "code is malformed"));
        }
        let code = code.unwrap();
        let player_id = Player::get_id_from_code(code);
        if player_id.is_none() {
            return Err((StatusCode::BAD_REQUEST, "code is malformed!"));
        }
        let player_id = player_id.unwrap();

        //Check if player exists with the same code
        {
            let read_lock = state.players.read().await;
            let existing_user = read_lock.get(&player_id);
            if let Some(user) = existing_user {
                if user.get_code() != code {
                    return Err((
                        StatusCode::UNAUTHORIZED,
                        "Code doesn't match the Players code.",
                    ));
                } else {
                    return Ok((
                        StatusCode::OK,
                        Json(MeResponse {
                            player: user.clone(),
                        }),
                    ));
                }
            } else {
                return Err((StatusCode::NOT_FOUND, "Player with id not found!"));
            }
        }
    } else {
        return Err((StatusCode::UNAUTHORIZED, "Unauthorized"));
    }
}

async fn setup_game(
    headers: HeaderMap,
    State(state): State<App>,
    setup: Option<Json<Setup>>,
) -> Result<(StatusCode, Json<SetupResponse>), impl IntoResponse> {
    let header_code = headers.get("code");
    if let Some(code) = header_code {
        let code = code.to_str();
        if code.is_err() {
            return Err((StatusCode::BAD_REQUEST, "code is malformed"));
        }
        let code = code.unwrap();
        let player_id = Player::get_id_from_code(code);
        if player_id.is_none() {
            return Err((StatusCode::BAD_REQUEST, "code is malformed!"));
        }
        let player_id = player_id.unwrap();
        let player;
        //Check if player exists with the same code
        {
            let read_lock = state.players.read().await;
            let existing_user = read_lock.get(&player_id);
            if let Some(user) = existing_user {
                if user.get_code() != code {
                    return Err((
                        StatusCode::UNAUTHORIZED,
                        "Code doesn't match the Players code.",
                    ));
                }
                player = user.clone();
            } else {
                return Err((StatusCode::NOT_FOUND, "Player with id not found!"));
            }
        }
        //Check for existing game_setups created by same user
        {
            let read_lock = state.setups.read().await;
            let setup_option = read_lock.get(&player_id);
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
                .filter(|entry| entry.1.get_creator_id() == &player_id);
            if existing_games.count() > 0 {
                return Err((
                    StatusCode::CONFLICT,
                    "Player has already started another game!",
                ));
            }
        }
        //Start a new game setup
        {
            let mut game_setup = GameSetup::new(&player);
            game_setup.update(setup.and_then(|opt| opt.0.game_code));
            let mut write_lock = state.setups.write_owned().await;
            write_lock.insert(player_id.clone(), game_setup);
        }

        //Update Player State
        {
            let mut write_lock = state.players.write_owned().await;
            let player_option = write_lock.get_mut(&player_id);
            if let Some(player) = player_option {
                player.create_game_setup();
            } else {
                unreachable!("Player should always exist at this point!");
            }
        }

        Ok((
            StatusCode::CREATED,
            Json(SetupResponse { game_id: player_id }),
        ))
    } else {
        Err((StatusCode::UNAUTHORIZED, "Unauthorized"))
    }
}

async fn update_code(
    headers: HeaderMap,
    State(state): State<App>,
    setup_update: Option<Json<GameSetupUpdate>>,
) -> Result<(StatusCode, Json<SetupResponse>), impl IntoResponse> {
    let header_code = headers.get("code");
    if let Some(code) = header_code {
        let code = code.to_str();
        if code.is_err() {
            return Err((StatusCode::BAD_REQUEST, "code is malformed"));
        }
        let code = code.unwrap();
        let player_id = Player::get_id_from_code(code);
        if player_id.is_none() {
            return Err((StatusCode::BAD_REQUEST, "code is malformed!"));
        }
        let player_id = player_id.unwrap();

        //Check if player exists with the same code
        {
            let read_lock = state.players.read().await;
            let existing_user = read_lock.get(&player_id);
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
        //Check for existing game_setup created by same user
        {
            let mut write_lock = state.setups.write_owned().await;
            let setup_option = write_lock.get_mut(&player_id);
            if let Some(setup) = setup_option {
                if let Some(update) = setup_update {
                    let mut game_code_option = update.0.game_code;
                    if let Some(game_code) = game_code_option.as_deref() {
                        tracing::info!("Updating code to {game_code}");
                    }
                    setup.update(game_code_option.take());
                } else {
                    setup.update(None);
                }
            }
        }
        //Update Player State
        {
            let mut write_lock = state.players.write_owned().await;
            let player_option = write_lock.get_mut(&player_id);
            if let Some(player) = player_option {
                player.update_game_setup();
            } else {
                unreachable!("Player should always exist at this point!");
            }
        }
        Ok((StatusCode::OK, Json(SetupResponse { game_id: player_id })))
    } else {
        Err((StatusCode::UNAUTHORIZED, "Unauthorized"))
    }
}

#[derive(Deserialize)]
struct JoinRequest {
    game_id: Uuid,
    game_code: Option<String>,
}

#[derive(Serialize)]
enum JoinResponseState {
    Success,
    Failure,
}

impl From<JoinGameSetupResult> for JoinResponseState {
    fn from(value: JoinGameSetupResult) -> Self {
        match value {
            JoinGameSetupResult::Success => JoinResponseState::Success,
            JoinGameSetupResult::WrongCode => JoinResponseState::Failure,
            JoinGameSetupResult::SetupFull => JoinResponseState::Failure,
        }
    }
}

#[derive(Serialize)]
struct JoinResponse {
    game_id: Uuid,
    state: JoinResponseState,
}

impl From<&JoinResponse> for bool {
    fn from(val: &JoinResponse) -> Self {
        match val.state {
            JoinResponseState::Success => true,
            JoinResponseState::Failure => false,
        }
    }
}

async fn join_setup(
    headers: HeaderMap,
    State(state): State<App>,
    join_request: Json<JoinRequest>,
) -> Result<(StatusCode, Json<JoinResponse>), impl IntoResponse> {
    let join_request = join_request.0;
    let header_code = headers.get("code");
    if let Some(code) = header_code {
        let code = code.to_str();
        if code.is_err() {
            return Err((StatusCode::BAD_REQUEST, "code is malformed"));
        }
        let code = code.unwrap();
        let player_id = Player::get_id_from_code(code);
        if player_id.is_none() {
            return Err((StatusCode::BAD_REQUEST, "code is malformed!"));
        }
        let player_id = player_id.unwrap();
        let player;
        //Check if player exists with the same code
        {
            let read_lock = state.players.read().await;
            let existing_user = read_lock.get(&player_id);
            if let Some(user) = existing_user {
                if user.get_code() != code {
                    return Err((
                        StatusCode::UNAUTHORIZED,
                        "Code doesn't match the Players code.",
                    ));
                }
                player = user.clone();
            } else {
                return Err((StatusCode::NOT_FOUND, "Player with id not found!"));
            }
        }
        //Check for existing game_setup
        {
            let mut write_lock = state.setups.write_owned().await;
            let setup_option = write_lock.get_mut(&join_request.game_id);
            if let Some(setup) = setup_option {
                let join_result = setup.join(&player, join_request.game_code.as_deref());
                let join_response = JoinResponse {
                    game_id: join_request.game_id,
                    state: join_result.into(),
                };
                //Update Player State
                {
                    let mut write_lock = state.players.write_owned().await;
                    let player_option = write_lock.get_mut(&player_id);
                    if let Some(player) = player_option {
                        player.join_game_setup(join_response.game_id.clone());
                    } else {
                        unreachable!("Player should always exist at this point!");
                    }
                }
                return Ok((StatusCode::OK, Json(join_response)));
            } else {
                return Err((StatusCode::NOT_FOUND, "Game setup not found"));
            }
        }
    } else {
        Err((StatusCode::UNAUTHORIZED, "Unauthorized"))
    }
}

async fn leave_setup(
    headers: HeaderMap,
    State(state): State<App>,
) -> Result<StatusCode, impl IntoResponse> {
    let header_code = headers.get("code");
    if let Some(code) = header_code {
        let code = code.to_str();
        if code.is_err() {
            return Err((StatusCode::BAD_REQUEST, "code is malformed"));
        }
        let code = code.unwrap();
        let player_id = Player::get_id_from_code(code);
        if player_id.is_none() {
            return Err((StatusCode::BAD_REQUEST, "code is malformed!"));
        }
        let player_id = player_id.unwrap();
        let game_id: Option<Uuid>;
        let is_creator;
        //Check if player exists with the same code
        {
            let read_lock = state.players.read().await;
            let existing_user = read_lock.get(&player_id);
            if let Some(user) = existing_user {
                if user.get_code() != code {
                    return Err((
                        StatusCode::UNAUTHORIZED,
                        "Code doesn't match the Players code.",
                    ));
                } else if user.has_joined_game_setup() || user.has_created_game_setup() {
                    game_id = user.get_game_id();
                    is_creator = user.has_created_game_setup();
                } else {
                    return Err((
                        StatusCode::CONFLICT,
                        "Player is currently not in a game setup",
                    ));
                }
            } else {
                return Err((StatusCode::NOT_FOUND, "Player with id not found!"));
            }
        }

        let game_id = game_id.expect("game_id should be set");
        //Check if player exists as member of specified game session
        {
            let mut write_lock = state.setups.write_owned().await;
            let game_setup_option = write_lock.get_mut(&game_id);
            if let Some(game_setup) = game_setup_option {
                if is_creator {
                    //Effectively close game setup. Set all players to free
                    for player in game_setup.players.iter() {
                        //Update Player State
                        {
                            let mut write_lock = state.players.write().await;
                            let player_option = write_lock.get_mut(&player.id);
                            if let Some(player) = player_option {
                                player.leave();
                            } else {
                                unreachable!("Player should always exist at this point!");
                            }
                        }
                    }
                    write_lock.remove_entry(&game_id);
                    return Ok(StatusCode::OK);
                } else {
                    let remove_success = game_setup.remove_player(player_id);
                    if remove_success {
                        //Update Player State
                        {
                            let mut write_lock = state.players.write_owned().await;
                            let player_option = write_lock.get_mut(&player_id);
                            if let Some(player) = player_option {
                                player.leave();
                            } else {
                                unreachable!("Player should always exist at this point!");
                            }
                        }
                        return Ok(StatusCode::NO_CONTENT);
                    } else {
                        return Err((
                            StatusCode::NOT_FOUND,
                            "Player is not part of that game setup",
                        ));
                    }
                }
            } else {
                return Err((StatusCode::NOT_FOUND, "Game setup doesn't exist"));
            }
        }
    } else {
        return Err((StatusCode::UNAUTHORIZED, "Unauthorized"));
    }
}

#[derive(Serialize)]
struct GameSetupStateResponse {
    id: Uuid,
    players: Vec<GameSetupStatePlayerResponse>,
}

#[derive(Serialize)]
struct GameSetupStatePlayerResponse {
    id: Uuid,
    nick: String,
}

async fn get_game_setup(
    headers: HeaderMap,
    State(state): State<App>,
) -> Result<(StatusCode, Json<GameSetupStateResponse>), impl IntoResponse> {
    let header_code = headers.get("code");
    if let Some(code) = header_code {
        let code = code.to_str();
        if code.is_err() {
            return Err((StatusCode::BAD_REQUEST, "code is malformed"));
        }
        let code = code.unwrap();
        let player_id = Player::get_id_from_code(code);
        if player_id.is_none() {
            return Err((StatusCode::BAD_REQUEST, "code is malformed!"));
        }
        let player_id = player_id.unwrap();
        let game_id: Option<Uuid>;
        //Check if player exists with the same code
        {
            let read_lock = state.players.read().await;
            let existing_user = read_lock.get(&player_id);
            if let Some(user) = existing_user {
                if user.get_code() != code {
                    return Err((
                        StatusCode::UNAUTHORIZED,
                        "Code doesn't match the Players code.",
                    ));
                } else if user.has_joined_game_setup() || user.has_created_game_setup() {
                    game_id = user.get_game_id();
                } else {
                    return Err((
                        StatusCode::CONFLICT,
                        "Player is currently not in a game setup",
                    ));
                }
            } else {
                return Err((StatusCode::NOT_FOUND, "Player with id not found!"));
            }
        }

        let game_id = game_id.expect("game_id should be set");
        //Check if player exists as member of specified game session
        {
            let read_lock = state.setups.read_owned().await;
            let game_setup_option = read_lock.get(&game_id);
            if let Some(game_setup) = game_setup_option {
                let players = game_setup
                    .players
                    .iter()
                    .map(|p| GameSetupStatePlayerResponse {
                        id: p.id.clone(),
                        nick: p.nick.clone(),
                    })
                    .collect();
                let game_setup_state_response = GameSetupStateResponse {
                    id: game_id,
                    players,
                };
                return Ok((StatusCode::OK, Json(game_setup_state_response)));
            } else {
                return Err((StatusCode::NOT_FOUND, "Game setup doesn't exist"));
            }
        }
    } else {
        return Err((StatusCode::UNAUTHORIZED, "Unauthorized"));
    }
}

#[derive(Serialize)]
struct GameStateResponse {}

async fn start_game(
    headers: HeaderMap,
    State(state): State<App>,
) -> Result<(StatusCode, Json<GameStateResponse>), impl IntoResponse> {
    return Err(StatusCode::INTERNAL_SERVER_ERROR);
}

async fn get_players(State(state): State<App>) -> impl IntoResponse {
    let read_lock = state.players.read_owned().await;
    let players: Vec<PlayerResponse> = read_lock.iter().map(|(_, value)| value.into()).collect();
    let count = players.len();
    if count == 0 {
        tracing::info!("No players registered");
    } else if count == 1 {
        tracing::info!("Currently {count} registered player");
    } else {
        tracing::info!("Currently {count} registered players");
    }
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
