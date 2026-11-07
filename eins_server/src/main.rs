use axum::extract::{FromRequestParts, State};
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::post;
use axum::Json;
use axum::{response::IntoResponse, routing::get, Router};
use eins_lib::api::*;
use eins_lib::game::{Game, GameView, MoveError};
use eins_lib::infrastructure::Player;
use serde::Serialize;
use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::{Stream, StreamExt};
use tracing_subscriber::FmtSubscriber;
use uuid::Uuid;

#[derive(Clone)]
struct App {
    players: Arc<RwLock<HashMap<Uuid, Player>>>,
    setups: Arc<RwLock<HashMap<Uuid, GameSetup>>>,
    games: Arc<RwLock<HashMap<Uuid, Game>>>,
    events: broadcast::Sender<ServerEvent>,
}

impl App {
    fn new() -> Self {
        Self {
            players: Arc::new(RwLock::new(HashMap::new())),
            setups: Arc::new(RwLock::new(HashMap::new())),
            games: Arc::new(RwLock::new(HashMap::new())),
            events: broadcast::channel(64).0,
        }
    }

    /// Tells every connected client that something changed.
    fn notify(&self, event: ServerEvent) {
        // Sending only fails when no client is listening.
        let _ = self.events.send(event);
    }

    async fn update_player(&self, player_id: Uuid, update: impl FnOnce(&mut Player)) {
        let mut players = self.players.write().await;
        let player = players
            .get_mut(&player_id)
            .expect("Player should always exist at this point!");
        update(player);
    }
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
            creator: player_id,
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

    fn join(&mut self, player: &Player, code: Option<&str>) -> JoinGameSetupResult {
        if self.has_correct_code(code) {
            let result = self.add_player(player);
            self.set_is_ready();
            return result;
        }
        JoinGameSetupResult::WrongCode
    }

    fn has_correct_code(&self, code: Option<&str>) -> bool {
        if let Some(current_code) = &self.code {
            if let Some(check_code) = code {
                return current_code == check_code;
            }
            false
        } else {
            code.is_none()
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
        if !self.players.iter().any(|gsp| gsp.id == player) {
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
        true
    }
}

impl From<&GameSetup> for GameSetupResponse {
    fn from(value: &GameSetup) -> Self {
        Self {
            id: value.creator,
            number_of_players: value.players.len(),
            state: value.state,
        }
    }
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
        .route("/game/me", get(get_my_game))
        .route("/game/play", post(play_card))
        .route("/game/draw", post(draw_card))
        .route("/game/leave", post(leave_game))
        .route("/events", get(events))
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

    let mut write_lock = state.players.write().await;
    if write_lock
        .values()
        .any(|p| p.get_nick() == registration.nick)
    {
        return Err((
            StatusCode::CONFLICT,
            "Nick already in use, please choose another!",
        ));
    }
    let player = Player::new(registration.nick);
    let id = player.get_id();
    write_lock.insert(id, player.clone());
    state.notify(ServerEvent::Lobby);
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
    state.notify(ServerEvent::Lobby);

    Ok((
        StatusCode::CREATED,
        Json(SetupResponse { game_id: player_id }),
    ))
}

async fn update_code(
    State(state): State<App>,
    AuthPlayer(player): AuthPlayer,
    setup_update: Option<Json<GameSetupUpdate>>,
) -> Result<(StatusCode, Json<SetupResponse>), (StatusCode, &'static str)> {
    let player_id = player.get_id();
    {
        let mut setups = state.setups.write().await;
        let setup = setups
            .get_mut(&player_id)
            .ok_or((StatusCode::NOT_FOUND, "Player has not created a game setup"))?;
        let game_code = setup_update.and_then(|update| update.0.game_code);
        if let Some(game_code) = game_code.as_deref() {
            tracing::info!("Updating code to {game_code}");
        }
        setup.update(game_code);
    }
    state
        .update_player(player_id, Player::update_game_setup)
        .await;
    state.notify(ServerEvent::Lobby);
    Ok((StatusCode::OK, Json(SetupResponse { game_id: player_id })))
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

async fn join_setup(
    State(state): State<App>,
    AuthPlayer(player): AuthPlayer,
    Json(join_request): Json<JoinRequest>,
) -> Result<(StatusCode, Json<JoinResponse>), (StatusCode, &'static str)> {
    if !player.is_free() {
        return Err((
            StatusCode::CONFLICT,
            "Player is already in a game setup or game",
        ));
    }
    let game_id = join_request.game_id;
    let mut setups = state.setups.write().await;
    let setup = setups
        .get_mut(&game_id)
        .ok_or((StatusCode::NOT_FOUND, "Game setup not found"))?;
    let join_result = setup.join(&player, join_request.game_code.as_deref());
    if matches!(join_result, JoinGameSetupResult::Success) {
        state
            .update_player(player.get_id(), |p| p.join_game_setup(game_id))
            .await;
        state.notify(ServerEvent::Lobby);
    }
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
        state.notify(ServerEvent::Lobby);
        return Ok(StatusCode::OK);
    }
    if !game_setup.remove_player(player_id) {
        return Err((
            StatusCode::NOT_FOUND,
            "Player is not part of that game setup",
        ));
    }
    state.update_player(player_id, Player::leave).await;
    state.notify(ServerEvent::Lobby);
    Ok(StatusCode::NO_CONTENT)
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
        .get(&game_id)
        .ok_or((StatusCode::NOT_FOUND, Cow::from("Game setup doesn't exist")))?;
    if !game_setup.is_ready {
        return Err((
            StatusCode::BAD_REQUEST,
            Cow::from("Game setup is not ready to start yet"),
        ));
    }
    let game = Game::new(game_setup.players.iter().map(|p| p.id).collect())
        .map_err(|e| (StatusCode::BAD_REQUEST, Cow::from(format!("Error: {e}"))))?
        .start();
    let game_setup = setups.remove(&game_id).expect("checked above");
    state.games.write().await.insert(game_id, game);
    for member in game_setup.players.iter() {
        if member.id == game_setup.creator {
            state.update_player(member.id, Player::start_game).await;
        } else {
            state
                .update_player(member.id, |p| p.join_game(game_id))
                .await;
        }
    }
    state.notify(ServerEvent::Lobby);
    state.notify(ServerEvent::Game { game_id });
    Ok((StatusCode::OK, Json(GameStateResponse { game_id })))
}

type ApiError = (StatusCode, String);

fn running_game_id(player: &Player) -> Result<Uuid, ApiError> {
    if !player.is_in_game() {
        return Err((
            StatusCode::CONFLICT,
            "Player is currently not in a game".into(),
        ));
    }
    Ok(player
        .get_game_id()
        .expect("players in a game have a game id"))
}

fn move_error(error: MoveError) -> ApiError {
    let status = match error {
        MoveError::GameNotRunning | MoveError::NotYourTurn => StatusCode::CONFLICT,
        MoveError::NotInGame => StatusCode::FORBIDDEN,
        MoveError::CardNotInHand
        | MoveError::CardNotPlayable
        | MoveError::ColorWishRequired
        | MoveError::ColorWishNotAllowed => StatusCode::BAD_REQUEST,
    };
    (status, error.to_string())
}

/// Applies `make_move` to the player's game and returns their view of the result.
async fn apply_move(
    state: &App,
    player: &Player,
    make_move: impl FnOnce(Game) -> (Game, Result<(), MoveError>),
) -> Result<Json<GameView>, ApiError> {
    let game_id = running_game_id(player)?;
    let mut games = state.games.write().await;
    let game = games
        .remove(&game_id)
        .ok_or((StatusCode::NOT_FOUND, "Game doesn't exist".to_string()))?;
    let (game, result) = make_move(game);
    let view = game.view(player.get_id());
    games.insert(game_id, game);
    result.map_err(move_error)?;
    state.notify(ServerEvent::Game { game_id });
    Ok(Json(view.expect("the player is part of the game")))
}

async fn get_my_game(
    State(state): State<App>,
    AuthPlayer(player): AuthPlayer,
) -> Result<Json<GameView>, ApiError> {
    let game_id = running_game_id(&player)?;
    let games = state.games.read().await;
    let game = games
        .get(&game_id)
        .ok_or((StatusCode::NOT_FOUND, "Game doesn't exist".to_string()))?;
    let view = game
        .view(player.get_id())
        .ok_or((StatusCode::FORBIDDEN, MoveError::NotInGame.to_string()))?;
    Ok(Json(view))
}

async fn play_card(
    State(state): State<App>,
    AuthPlayer(player): AuthPlayer,
    Json(request): Json<PlayRequest>,
) -> Result<Json<GameView>, ApiError> {
    let id = player.get_id();
    apply_move(&state, &player, |game| {
        game.play_card(id, request.card, request.color, request.eins)
    })
    .await
}

async fn draw_card(
    State(state): State<App>,
    AuthPlayer(player): AuthPlayer,
) -> Result<Json<GameView>, ApiError> {
    let id = player.get_id();
    apply_move(&state, &player, |game| game.draw(id)).await
}

/// Leaves the current game. In a running game the others play on; the game is removed once
/// every player has left it.
async fn leave_game(
    State(state): State<App>,
    AuthPlayer(player): AuthPlayer,
) -> Result<StatusCode, ApiError> {
    let game_id = running_game_id(&player)?;
    let player_id = player.get_id();
    let mut games = state.games.write().await;
    if let Some(game) = games.remove(&game_id) {
        let game = match game {
            running @ Game::Play(_) => {
                let (game, result) = running.leave(player_id);
                if let Err(error) = result {
                    games.insert(game_id, game);
                    return Err(move_error(error));
                }
                game
            }
            other => other,
        };
        games.insert(game_id, game);
    }

    let mut players = state.players.write().await;
    players
        .get_mut(&player_id)
        .expect("Player should always exist at this point!")
        .leave();
    let anyone_left = players
        .values()
        .any(|p| p.is_in_game() && p.get_game_id() == Some(game_id));
    if !anyone_left {
        games.remove(&game_id);
    }
    state.notify(ServerEvent::Game { game_id });
    Ok(StatusCode::NO_CONTENT)
}

/// Server-sent events: one `ServerEvent` as JSON data per change.
async fn events(
    State(state): State<App>,
    AuthPlayer(_): AuthPlayer,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = BroadcastStream::new(state.events.subscribe()).map(|received| {
        // A client that falls too far behind has missed events and must refresh everything.
        let event = received.unwrap_or(ServerEvent::Resync);
        Ok(Event::default()
            .json_data(event)
            .expect("server events serialize"))
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn get_players(State(state): State<App>) -> impl IntoResponse {
    let read_lock = state.players.read_owned().await;
    let players: Vec<PlayerResponse> = read_lock.values().map(|value| value.into()).collect();
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
    let setups: Vec<GameSetupResponse> = read_lock.values().map(|value| value.into()).collect();
    let count = setups.len();
    tracing::info!("Currently {count} number of game setups");
    let response = GameSetupsResponse { setups };
    Json(response)
}

async fn get_games(State(state): State<App>) -> impl IntoResponse {
    let read_lock = state.games.read_owned().await;
    let games: Vec<GameResponse> = read_lock
        .values()
        .map(|game| GameResponse {
            id: *game.get_game_id(),
            number_of_players: game.get_number_of_players(),
            state: game.get_game_session_state().to_string(),
        })
        .collect();
    let count = games.len();
    tracing::info!("Currently {count} number of games");
    let response = GamesResponse { games };
    Json(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use serde_json::{json, Value};
    use tower::ServiceExt;

    struct Client {
        app: Router,
    }

    impl Client {
        fn new() -> Self {
            Self {
                app: app(App::new()),
            }
        }

        async fn call(
            &self,
            method: &str,
            uri: &str,
            code: Option<&str>,
            body: Option<Value>,
        ) -> (StatusCode, Value) {
            let mut request = Request::builder().method(method).uri(uri);
            if let Some(code) = code {
                request = request.header("code", code);
            }
            let request = match body {
                Some(body) => request
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string())),
                None => request.body(Body::empty()),
            }
            .unwrap();
            let response = self.app.clone().oneshot(request).await.unwrap();
            let status = response.status();
            let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            let body = serde_json::from_slice(&bytes)
                .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into()));
            (status, body)
        }

        /// Registers a player and returns (id, code).
        async fn register(&self, nick: &str) -> (String, String) {
            let (status, body) = self
                .call(
                    "POST",
                    "/player/register",
                    None,
                    Some(json!({ "nick": nick })),
                )
                .await;
            assert_eq!(status, StatusCode::CREATED);
            let code = body["player"]["code"].as_str().unwrap().to_string();
            (code[..36].to_string(), code)
        }

        async fn state_of(&self, code: &str) -> Value {
            self.call("GET", "/player/me", Some(code), None).await.1["player"]["state"].clone()
        }

        async fn join(
            &self,
            code: &str,
            game_id: &str,
            game_code: Option<&str>,
        ) -> (StatusCode, Value) {
            let body = json!({ "game_id": game_id, "game_code": game_code });
            self.call("POST", "/game/setup/join", Some(code), Some(body))
                .await
        }

        async fn setup_players(&self, game_id: &str, code: &str) -> Vec<String> {
            let (_, body) = self.call("GET", "/game/setup/me", Some(code), None).await;
            assert_eq!(body["id"], game_id);
            body["players"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| p["nick"].as_str().unwrap().to_string())
                .collect()
        }
    }

    #[tokio::test]
    async fn joined_player_can_leave_setup() {
        let c = Client::new();
        let (alice_id, alice) = c.register("alice").await;
        let (_, bob) = c.register("bob").await;
        c.call("POST", "/game/setup", Some(&alice), None).await;
        c.join(&bob, &alice_id, None).await;

        let (status, _) = c.call("POST", "/game/setup/leave", Some(&bob), None).await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert_eq!(c.state_of(&bob).await, "Free");
        assert_eq!(c.setup_players(&alice_id, &alice).await, vec!["alice"]);
    }

    #[tokio::test]
    async fn join_only_succeeds_for_free_players_with_the_right_code() {
        let c = Client::new();
        let (alice_id, alice) = c.register("alice").await;
        let (_, bob) = c.register("bob").await;
        let (carol_id, carol) = c.register("carol").await;
        let body = json!({ "game_code": "secret" });
        c.call("POST", "/game/setup", Some(&alice), Some(body))
            .await;

        let (status, body) = c.join(&bob, &alice_id, Some("wrong")).await;
        assert_eq!(
            (status, &body["state"]),
            (StatusCode::OK, &json!("Failure"))
        );
        assert_eq!(c.state_of(&bob).await, "Free");

        assert_eq!(
            c.join(&bob, &alice_id, Some("secret")).await.1["state"],
            "Success"
        );
        assert_eq!(
            c.join(&bob, &alice_id, Some("secret")).await.0,
            StatusCode::CONFLICT
        );
        assert_eq!(
            c.join(&alice, &alice_id, Some("secret")).await.0,
            StatusCode::CONFLICT
        );

        // A player who is in another setup cannot join as well.
        c.call("POST", "/game/setup", Some(&carol), None).await;
        assert_eq!(
            c.join(&carol, &alice_id, Some("secret")).await.0,
            StatusCode::CONFLICT
        );
        assert_eq!(c.state_of(&carol).await, "GameSetupCreated");
        assert_eq!(c.setup_players(&carol_id, &carol).await, vec!["carol"]);

        assert_eq!(
            c.setup_players(&alice_id, &alice).await,
            vec!["alice", "bob"]
        );
    }

    #[tokio::test]
    async fn starting_moves_the_setup_into_a_game() {
        let c = Client::new();
        let (alice_id, alice) = c.register("alice").await;
        let (_, bob) = c.register("bob").await;
        c.call("POST", "/game/setup", Some(&alice), None).await;
        c.join(&bob, &alice_id, None).await;

        let (status, body) = c
            .call("POST", "/game/setup/start", Some(&alice), None)
            .await;
        assert_eq!(
            (status, &body["game_id"]),
            (StatusCode::OK, &json!(alice_id))
        );
        let (status, _) = c
            .call("POST", "/game/setup/start", Some(&alice), None)
            .await;
        assert_eq!(status, StatusCode::CONFLICT);

        assert_eq!(c.state_of(&alice).await, "GameStarted");
        assert_eq!(c.state_of(&bob).await, json!({ "GameJoined": alice_id }));
        let (_, setups) = c.call("GET", "/game/setup", None, None).await;
        assert_eq!(setups["setups"], json!([]));
        let (_, games) = c.call("GET", "/game", None, None).await;
        assert_eq!(
            games["games"],
            json!([{ "id": alice_id, "number_of_players": 2, "state": "Play" }])
        );
    }

    #[tokio::test]
    async fn update_without_setup_is_rejected() {
        let c = Client::new();
        let (_, dave) = c.register("dave").await;
        let body = json!({ "game_code": "x" });
        let (status, _) = c
            .call("POST", "/game/setup/update", Some(&dave), Some(body))
            .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(c.state_of(&dave).await, "Free");
    }

    /// Registers alice and bob and starts a game between them; returns their (id, code).
    async fn two_player_game(c: &Client) -> [(String, String); 2] {
        let alice = c.register("alice").await;
        let bob = c.register("bob").await;
        c.call("POST", "/game/setup", Some(&alice.1), None).await;
        c.join(&bob.1, &alice.0, None).await;
        let (status, _) = c
            .call("POST", "/game/setup/start", Some(&alice.1), None)
            .await;
        assert_eq!(status, StatusCode::OK);
        [alice, bob]
    }

    fn card_ids(view: &Value) -> Vec<Value> {
        view["hand"]
            .as_array()
            .unwrap()
            .iter()
            .map(|h| h["id"].clone())
            .collect()
    }

    #[tokio::test]
    async fn players_only_see_their_own_hand() {
        let c = Client::new();
        let [(alice_id, alice), (bob_id, bob)] = two_player_game(&c).await;
        let (status, alices) = c.call("GET", "/game/me", Some(&alice), None).await;
        assert_eq!(status, StatusCode::OK);
        let (_, bobs) = c.call("GET", "/game/me", Some(&bob), None).await;

        assert_eq!(alices["state"], "Play");
        assert_eq!(
            alices["players"],
            json!([{ "id": alice_id, "card_count": 7 }, { "id": bob_id, "card_count": 7 }])
        );
        let (alice_cards, bob_cards) = (card_ids(&alices), card_ids(&bobs));
        assert_eq!((alice_cards.len(), bob_cards.len()), (7, 7));
        assert!(alice_cards.iter().all(|card| !bob_cards.contains(card)));

        let current = &alices["turn"]["current_player"];
        assert_eq!(current, &bobs["turn"]["current_player"]);
        let waiting = if current == &json!(alice_id) {
            &bobs
        } else {
            &alices
        };
        assert_eq!(waiting["turn"]["playable"], json!([]));

        let (_, carol) = c.register("carol").await;
        assert_eq!(
            c.call("GET", "/game/me", Some(&carol), None).await.0,
            StatusCode::CONFLICT
        );
    }

    #[tokio::test]
    async fn moves_are_checked_and_applied() {
        let c = Client::new();
        let players = two_player_game(&c).await;
        let view_of = |i: usize| c.call("GET", "/game/me", Some(&players[i].1), None);
        let current_index = |view: &Value| {
            players
                .iter()
                .position(|(id, _)| view["turn"]["current_player"] == json!(id))
                .unwrap()
        };
        let (_, view) = view_of(0).await;
        let (now, other) = if current_index(&view) == 0 {
            (0, 1)
        } else {
            (1, 0)
        };
        let (_, current_view) = view_of(now).await;
        let (_, other_view) = view_of(other).await;

        let other_code = Some(players[other].1.as_str());
        let now_code = Some(players[now].1.as_str());
        assert_eq!(
            c.call("POST", "/game/draw", other_code, None).await.0,
            StatusCode::CONFLICT
        );
        let foreign_card = json!({ "card": card_ids(&other_view)[0], "color": "Red" });
        let (status, _) = c
            .call("POST", "/game/play", now_code, Some(foreign_card))
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        let (status, _) = c
            .call("POST", "/game/play", now_code, Some(json!({ "card": 999 })))
            .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

        let to_draw = current_view["turn"]["pending_draw"]
            .as_u64()
            .unwrap()
            .max(1) as usize;
        let (status, after_draw) = c.call("POST", "/game/draw", now_code, None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(card_ids(&after_draw).len(), 7 + to_draw);
        assert_eq!(
            after_draw["turn"]["current_player"],
            json!(players[other].0)
        );

        // Whoever can play first plays their first playable card.
        for _ in 0..40 {
            let (_, view) = view_of(0).await;
            let i = current_index(&view);
            let (_, view) = view_of(i).await;
            let code = Some(players[i].1.as_str());
            let Some(card) = view["turn"]["playable"]
                .as_array()
                .unwrap()
                .first()
                .cloned()
            else {
                c.call("POST", "/game/draw", code, None).await;
                continue;
            };
            let hand_card = view["hand"]
                .as_array()
                .unwrap()
                .iter()
                .find(|h| h["id"] == card)
                .unwrap();
            let color = hand_card["card"].get("Wild").map(|_| "Green");
            let body = json!({ "card": card, "color": color, "eins": true });
            let (status, after) = c.call("POST", "/game/play", code, Some(body)).await;
            assert_eq!(status, StatusCode::OK, "{after}");
            assert_eq!(after["top_card"], hand_card["card"]);
            assert!(!card_ids(&after).contains(&card));
            return;
        }
        panic!("nobody could play a card");
    }

    #[tokio::test]
    async fn leaving_finishes_and_finally_removes_the_game() {
        let c = Client::new();
        let [(alice_id, alice), (_, bob)] = two_player_game(&c).await;

        let (status, _) = c.call("POST", "/game/leave", Some(&bob), None).await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert_eq!(c.state_of(&bob).await, "Free");
        let (_, view) = c.call("GET", "/game/me", Some(&alice), None).await;
        assert_eq!(
            (&view["state"], &view["winner"]),
            (&json!("Finished"), &json!(alice_id))
        );
        assert_eq!(
            c.call("GET", "/game", None, None).await.1["games"]
                .as_array()
                .unwrap()
                .len(),
            1
        );

        c.call("POST", "/game/leave", Some(&alice), None).await;
        assert_eq!(
            c.call("GET", "/game", None, None).await.1["games"],
            json!([])
        );
        assert_eq!(
            c.call("GET", "/game/me", Some(&alice), None).await.0,
            StatusCode::CONFLICT
        );
        // The creator can set up a new game afterwards.
        assert_eq!(
            c.call("POST", "/game/setup", Some(&alice), None).await.0,
            StatusCode::CREATED
        );
    }

    /// Reads the next pushed `ServerEvent`, or `None` if nothing arrives within `wait`.
    async fn next_event(body: &mut Body, wait: std::time::Duration) -> Option<Value> {
        use http_body_util::BodyExt;
        loop {
            let frame = tokio::time::timeout(wait, body.frame())
                .await
                .ok()??
                .unwrap();
            let Ok(data) = frame.into_data() else {
                continue;
            };
            let text = String::from_utf8(data.to_vec()).unwrap();
            if let Some(json) = text.lines().find_map(|line| line.strip_prefix("data: ")) {
                return Some(serde_json::from_str(json).unwrap());
            }
        }
    }

    #[tokio::test]
    async fn changes_are_pushed_to_connected_clients() {
        use std::time::Duration;
        let c = Client::new();
        let request = |code: Option<&str>| {
            let mut builder = Request::builder().uri("/events");
            if let Some(code) = code {
                builder = builder.header("code", code);
            }
            builder.body(Body::empty()).unwrap()
        };
        let anonymous = c.app.clone().oneshot(request(None)).await.unwrap();
        assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);

        let (alice_id, alice) = c.register("alice").await;
        let response = c.app.clone().oneshot(request(Some(&alice))).await.unwrap();
        assert_eq!(response.headers()["content-type"], "text/event-stream");
        let mut events = response.into_body();
        let wait = Duration::from_secs(2);

        let (_, bob) = c.register("bob").await;
        assert_eq!(next_event(&mut events, wait).await, Some(json!("Lobby")));
        c.call("POST", "/game/setup", Some(&alice), None).await;
        assert_eq!(next_event(&mut events, wait).await, Some(json!("Lobby")));
        c.join(&bob, &alice_id, None).await;
        assert_eq!(next_event(&mut events, wait).await, Some(json!("Lobby")));

        c.call("POST", "/game/setup/start", Some(&alice), None)
            .await;
        let game = json!({ "Game": { "game_id": alice_id } });
        assert_eq!(next_event(&mut events, wait).await, Some(json!("Lobby")));
        assert_eq!(next_event(&mut events, wait).await, Some(game.clone()));

        // A rejected move changes nothing, so nothing is pushed; an accepted one is.
        let (_, view) = c.call("GET", "/game/me", Some(&alice), None).await;
        let (current, waiting) = if view["turn"]["current_player"] == json!(alice_id) {
            (&alice, &bob)
        } else {
            (&bob, &alice)
        };
        let (status, _) = c.call("POST", "/game/draw", Some(waiting), None).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(
            next_event(&mut events, Duration::from_millis(200)).await,
            None
        );
        c.call("POST", "/game/draw", Some(current), None).await;
        assert_eq!(next_event(&mut events, wait).await, Some(game));
    }
}
