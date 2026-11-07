//! Request and response bodies of the server's HTTP API, shared with the client.

use std::fmt::Display;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::cards::{CardReference, Color};
use crate::infrastructure::Player;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registration {
    pub nick: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationResponse {
    pub player: Player,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeResponse {
    pub player: Player,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Setup {
    pub game_code: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum GameSetupState {
    Open,
    Closed,
    Full,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupResponse {
    pub game_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayersResponse {
    pub players: Vec<PlayerResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerResponse {
    pub nick: String,
    pub id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSetupsResponse {
    pub setups: Vec<GameSetupResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSetupResponse {
    pub id: Uuid,
    /// Nick of the player who created the setup and can start the game.
    pub host_nick: String,
    pub number_of_players: usize,
    pub state: GameSetupState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSetupUpdate {
    pub game_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamesResponse {
    pub games: Vec<GameResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameResponse {
    pub id: Uuid,
    pub number_of_players: usize,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinRequest {
    pub game_id: Uuid,
    pub game_code: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum JoinResponseState {
    Success,
    Failure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinResponse {
    pub game_id: Uuid,
    pub state: JoinResponseState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSetupStateResponse {
    pub id: Uuid,
    pub players: Vec<GameSetupStatePlayerResponse>,
    pub is_ready: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSetupStatePlayerResponse {
    pub id: Uuid,
    pub nick: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStateResponse {
    pub game_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayRequest {
    pub card: CardReference,
    /// Required for wild cards, not allowed for any other card.
    pub color: Option<Color>,
    /// The player's "Eins" call; leaving it out with one card left costs two cards.
    #[serde(default)]
    pub eins: bool,
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

impl From<&Player> for PlayerResponse {
    fn from(value: &Player) -> Self {
        Self {
            nick: value.get_nick().to_string(),
            id: value.get_id(),
        }
    }
}

/// Pushed over `GET /events` whenever server state changes. An event only says what changed;
/// clients fetch the details through the regular endpoints, so nothing private is pushed.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ServerEvent {
    /// Players, game setups or waiting rooms changed.
    Lobby,
    /// The game with this id changed.
    Game { game_id: Uuid },
    /// Events were dropped for this client; it should refresh everything.
    Resync,
}
