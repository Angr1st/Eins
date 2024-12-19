use jiff::Timestamp;
use serde::Serialize;
use uuid::Uuid;

use crate::game::Hand;

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PlayerState {
    Free,
    GameSetupCreated,
    GameSetupUpdated,
    GameSetupJoined(Uuid),
    GameStarted,
    GameJoined(Uuid),
}

#[derive(Clone, Serialize)]
pub struct Player {
    id: Uuid,
    nick: String,
    last_activity: Timestamp,
    code: String,
    state: PlayerState,
}

impl Player {
    pub fn new(nick: String) -> Self {
        let id = Uuid::new_v4();
        let mut code = String::new();
        Player::generate_code(&id, &mut code);
        Self {
            nick,
            id,
            last_activity: Timestamp::now(),
            code,
            state: PlayerState::Free,
        }
    }

    pub fn get_id(&self) -> Uuid {
        self.id.clone()
    }

    pub fn get_nick(&self) -> &str {
        &self.nick
    }

    pub fn get_code(&self) -> &str {
        &self.code
    }

    pub fn is_free(&self) -> bool {
        match self.state {
            PlayerState::Free => true,
            _ => false,
        }
    }

    pub fn has_joined_game_setup(&self) -> bool {
        match self.state {
            PlayerState::GameSetupJoined(_) => true,
            _ => false,
        }
    }

    pub fn get_game_id(&self) -> Option<Uuid> {
        match self.state {
            PlayerState::Free => None,
            PlayerState::GameSetupCreated => Some(self.get_id()),
            PlayerState::GameSetupUpdated => Some(self.get_id()),
            PlayerState::GameSetupJoined(uuid) => Some(uuid),
            PlayerState::GameStarted => Some(self.get_id()),
            PlayerState::GameJoined(uuid) => Some(uuid),
        }
    }

    fn generate_code(id: &Uuid, buffer: &mut String) {
        buffer.push_str(&id.to_string());
        for _ in 0..3 {
            buffer.push_str(&Uuid::new_v4().to_string())
        }
    }

    pub fn get_id_from_code(code: &str) -> Option<Uuid> {
        if code.len() < 36 {
            return None;
        }
        let first_uuid = &code[0..36];
        let first_uuid = Uuid::parse_str(first_uuid);
        first_uuid.ok()
    }

    pub fn leave(&mut self) {
        self.update_activity(PlayerState::Free);
    }

    pub fn create_game_setup(&mut self) {
        self.update_activity(PlayerState::GameSetupCreated);
    }

    pub fn update_game_setup(&mut self) {
        self.update_activity(PlayerState::GameSetupUpdated);
    }

    pub fn join_game_setup(&mut self, game_id: Uuid) {
        self.update_activity(PlayerState::GameSetupJoined(game_id));
    }

    pub fn start_game(&mut self) {
        self.update_activity(PlayerState::GameStarted);
    }

    pub fn join_game(&mut self, game_id: Uuid) {
        self.update_activity(PlayerState::GameJoined(game_id));
    }

    fn update_activity(&mut self, state: PlayerState) {
        self.state = state;
        self.last_activity = Timestamp::now();
    }
}

impl From<&Player> for Hand {
    fn from(value: &Player) -> Self {
        Hand::new(Some(value.id))
    }
}
