use axum::extract::{FromRequestParts, State};
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::routing::post;
use axum::Json;
use axum::{response::IntoResponse, routing::get, Router};
use eins_lib::game::Game;
use eins_lib::infrastructure::Player;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Display;
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

    async fn update_player(&self, player_id: Uuid, update: impl FnOnce(&mut Player)) {
        let mut players = self.players.write().await;
        let player = players
            .get_mut(&player_id)
            .expect("Player should always exist at this point!");
        update(player);
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
    is_ready: bool,
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
            is_ready: false,
        }
    }

    fn set_is_ready(&mut self) {
        self.is_ready = self.players.len() > 1;
    }

    fn join<'a>(&mut self, player: &Player, code: Option<&'a str>) -> JoinGameSetupResult {
        if self.has_correct_code(code) {
            let result = self.add_player(player);
            self.set_is_ready();
            return result;
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
        if self.state == GameSetupState::Started {
            panic!("Game Setup has been started! Code cannot be changed now");
        }
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
                GameSetupState::Started => GameSetupState::Started,
            }
        } else {
            match self.state {
                GameSetupState::Closed => GameSetupState::Open,
                GameSetupState::Open => GameSetupState::Open,
                GameSetupState::Full => GameSetupState::Full,
                GameSetupState::Started => GameSetupState::Started,
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
        self.set_is_ready();
        return true;
    }
}

#[derive(Copy, Clone, PartialEq, Serialize)]
enum GameSetupState {
    Open,
    Closed,
    Full,
    Started,
}

impl Display for GameSetupState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "game setup state: ")?;
        match self {
            GameSetupState::Open => write!(f, "Open"),
            GameSetupState::Closed => write!(f, "Closed"),
            GameSetupState::Full => write!(f, "Full"),
            GameSetupState::Started => write!(f, "Started"),
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

#[derive(Serialize)]
struct GamesResponse {
    games: Vec<GameResponse>,
}

#[derive(Serialize)]
struct GameResponse {
    id: Uuid,
    number_of_players: usize,
    state: String,
}

#[tokio::main]
async fn main() {
    let subscriber = FmtSubscriber::new();

    tracing::subscriber::set_global_default(subscriber).unwrap();
    let app = app(App::new());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

fn app(state: App) -> Router {
    Router::new()
        .route("/player", get(get_players))
        .route("/player/me", get(get_me))
        .route("/player/register", post(register))
        .route("/game/setup", get(get_game_setups).post(setup_game))
        .route("/game/setup/me", get(get_game_setup))
        .route("/game/setup/update", post(update_code))
        .route("/game/setup/join", post(join_setup))
        .route("/game/setup/leave", post(leave_setup))
        .route("/game/setup/start", post(start_game))
        .route("/game", get(get_games))
        .route("/health", get(get_app_health))
        .with_state(state)
}

#[derive(Serialize)]
struct AppHealth {
    status: String,
}

async fn get_app_health() -> impl IntoResponse {
    let status = AppHealth {
        status: "Ready".into(),
    };
    (StatusCode::OK, Json(status))
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

/// A registered player, authenticated by the secret `code` request header.
struct AuthPlayer(Player);

impl FromRequestParts<App> for AuthPlayer {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &App) -> Result<Self, Self::Rejection> {
        let code = parts
            .headers
            .get("code")
            .ok_or((StatusCode::UNAUTHORIZED, "Unauthorized"))?
            .to_str()
            .map_err(|_| (StatusCode::BAD_REQUEST, "code is malformed"))?;
        let player_id = Player::get_id_from_code(code)
            .ok_or((StatusCode::BAD_REQUEST, "code is malformed!"))?;
        let players = state.players.read().await;
        let player = players
            .get(&player_id)
            .ok_or((StatusCode::NOT_FOUND, "Player with id not found!"))?;
        if player.get_code() != code {
            return Err((
                StatusCode::UNAUTHORIZED,
                "Code doesn't match the Players code.",
            ));
        }
        Ok(AuthPlayer(player.clone()))
    }
}

async fn get_me(AuthPlayer(player): AuthPlayer) -> Json<MeResponse> {
    Json(MeResponse { player })
}

async fn setup_game(
    State(state): State<App>,
    AuthPlayer(player): AuthPlayer,
    setup: Option<Json<Setup>>,
) -> Result<(StatusCode, Json<SetupResponse>), (StatusCode, &'static str)> {
    let player_id = player.get_id();
    if state.setups.read().await.contains_key(&player_id) {
        return Err((
            StatusCode::CONFLICT,
            "Player has already started another game setup!",
        ));
    }
    let has_game = state
        .games
        .read()
        .await
        .values()
        .any(|game| game.get_creator_id() == &player_id);
    if has_game {
        return Err((
            StatusCode::CONFLICT,
            "Player has already started another game!",
        ));
    }

    let mut game_setup = GameSetup::new(&player);
    game_setup.update(setup.and_then(|opt| opt.0.game_code));
    state.setups.write().await.insert(player_id, game_setup);
    state
        .update_player(player_id, Player::create_game_setup)
        .await;

    Ok((
        StatusCode::CREATED,
        Json(SetupResponse { game_id: player_id }),
    ))
}

async fn update_code(
    State(state): State<App>,
    AuthPlayer(player): AuthPlayer,
    setup_update: Option<Json<GameSetupUpdate>>,
) -> (StatusCode, Json<SetupResponse>) {
    let player_id = player.get_id();
    if let Some(setup) = state.setups.write().await.get_mut(&player_id) {
        let game_code = setup_update.and_then(|update| update.0.game_code);
        if let Some(game_code) = game_code.as_deref() {
            tracing::info!("Updating code to {game_code}");
        }
        setup.update(game_code);
    }
    state
        .update_player(player_id, Player::update_game_setup)
        .await;
    (StatusCode::OK, Json(SetupResponse { game_id: player_id }))
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

async fn join_setup(
    State(state): State<App>,
    AuthPlayer(player): AuthPlayer,
    Json(join_request): Json<JoinRequest>,
) -> Result<(StatusCode, Json<JoinResponse>), (StatusCode, &'static str)> {
    let game_id = join_request.game_id;
    let mut setups = state.setups.write().await;
    let setup = setups
        .get_mut(&game_id)
        .ok_or((StatusCode::NOT_FOUND, "Game setup not found"))?;
    let join_result = setup.join(&player, join_request.game_code.as_deref());
    state
        .update_player(player.get_id(), |p| p.join_game_setup(game_id))
        .await;
    Ok((
        StatusCode::OK,
        Json(JoinResponse {
            game_id,
            state: join_result.into(),
        }),
    ))
}

async fn leave_setup(
    State(state): State<App>,
    AuthPlayer(player): AuthPlayer,
) -> Result<StatusCode, (StatusCode, &'static str)> {
    let is_creator = player.has_created_game_setup();
    if !is_creator && !player.has_joined_game_setup() {
        return Err((
            StatusCode::CONFLICT,
            "Player is currently not in a game setup",
        ));
    }
    let game_id = player.get_game_id().expect("game_id should be set");
    let player_id = player.get_id();

    let mut setups = state.setups.write().await;
    let game_setup = setups
        .get_mut(&game_id)
        .ok_or((StatusCode::NOT_FOUND, "Game setup doesn't exist"))?;
    if is_creator {
        //Effectively close game setup. Set all players to free
        for member in game_setup.players.iter() {
            state.update_player(member.id, Player::leave).await;
        }
        setups.remove(&game_id);
        return Ok(StatusCode::OK);
    }
    if !game_setup.remove_player(player_id) {
        return Err((
            StatusCode::NOT_FOUND,
            "Player is not part of that game setup",
        ));
    }
    state.update_player(player_id, Player::leave).await;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize)]
struct GameSetupStateResponse {
    id: Uuid,
    players: Vec<GameSetupStatePlayerResponse>,
    is_ready: bool,
}

#[derive(Serialize)]
struct GameSetupStatePlayerResponse {
    id: Uuid,
    nick: String,
}

async fn get_game_setup(
    State(state): State<App>,
    AuthPlayer(player): AuthPlayer,
) -> Result<Json<GameSetupStateResponse>, (StatusCode, &'static str)> {
    if !player.has_joined_game_setup() && !player.has_created_game_setup() {
        return Err((
            StatusCode::CONFLICT,
            "Player is currently not in a game setup",
        ));
    }
    let game_id = player.get_game_id().expect("game_id should be set");

    let setups = state.setups.read().await;
    let game_setup = setups
        .get(&game_id)
        .ok_or((StatusCode::NOT_FOUND, "Game setup doesn't exist"))?;
    let players = game_setup
        .players
        .iter()
        .map(|p| GameSetupStatePlayerResponse {
            id: p.id,
            nick: p.nick.clone(),
        })
        .collect();
    Ok(Json(GameSetupStateResponse {
        id: game_id,
        players,
        is_ready: game_setup.is_ready,
    }))
}

#[derive(Serialize)]
struct GameStateResponse {
    game_id: Uuid,
}

async fn start_game(
    State(state): State<App>,
    AuthPlayer(player): AuthPlayer,
) -> Result<(StatusCode, Json<GameStateResponse>), (StatusCode, Cow<'static, str>)> {
    if player.has_joined_game_setup() {
        return Err((
            StatusCode::BAD_REQUEST,
            Cow::from("Only the player who created the game setup can start the game"),
        ));
    }
    if !player.has_created_game_setup() {
        return Err((
            StatusCode::CONFLICT,
            Cow::from("Player is currently not in a game setup"),
        ));
    }
    let game_id = player.get_game_id().expect("game_id should be set");

    let mut setups = state.setups.write().await;
    let game_setup = setups
        .get_mut(&game_id)
        .ok_or((StatusCode::NOT_FOUND, Cow::from("Game setup doesn't exist")))?;
    if !game_setup.is_ready {
        return Err((
            StatusCode::BAD_REQUEST,
            Cow::from("Game setup is not ready to start yet"),
        ));
    }
    let game = Game::new(game_setup.players.iter().map(|p| p.id).collect())
        .map_err(|e| (StatusCode::BAD_REQUEST, Cow::from(format!("Error: {e}"))))?;
    state.games.write().await.insert(game_setup.creator, game);
    game_setup.set_is_ready();
    Ok((StatusCode::OK, Json(GameStateResponse { game_id })))
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

async fn get_games(State(state): State<App>) -> impl IntoResponse {
    let read_lock = state.games.read_owned().await;
    let games: Vec<GameResponse> = read_lock
        .iter()
        .map(|(_, value)| GameResponse {
            id: value.get_game_id().clone(),
            number_of_players: 1,
            state: String::from("Started"),
        })
        .collect();
    let count = games.len();
    tracing::info!("Currently {count} number of games");
    let response = GamesResponse { games };
    Json(response)
}
