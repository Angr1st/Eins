use jiff::Timestamp;
use serde::Serialize;
use uuid::Uuid;

use crate::game::Hand;

#[derive(Clone, Serialize)]
pub struct Player {
    id: Uuid,
    nick: String,
    last_activity: Timestamp,
    code: String,
}

impl Player {
    pub fn new(nick: String) -> Self {
        let mut code = String::new();
        Player::generate_code(&mut code);
        Self {
            nick,
            id: Uuid::new_v4(),
            last_activity: Timestamp::now(),
            code,
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

    fn generate_code(buffer: &mut String) {
        for _ in 0..4 {
            buffer.push_str(&Uuid::new_v4().to_string())
        }
    }
}

impl From<&Player> for Hand {
    fn from(value: &Player) -> Self {
        Hand::new(Some(value.id))
    }
}
