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
        let id = Uuid::new_v4();
        let mut code = String::new();
        Player::generate_code(&id, &mut code);
        Self {
            nick,
            id,
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
}

impl From<&Player> for Hand {
    fn from(value: &Player) -> Self {
        Hand::new(Some(value.id))
    }
}
