use serde::Serialize;
use uuid::Uuid;

use crate::game::Hand;

#[derive(Clone, Serialize)]
pub struct Player {
    id: Uuid,
    nick: String,
}

impl Player {
    pub fn new(nick: String) -> Self {
        Self {
            nick,
            id: Uuid::new_v4(),
        }
    }

    pub fn get_id(&self) -> Uuid {
        self.id.clone()
    }

    pub fn get_nick(&self) -> &str {
        &self.nick
    }
}

impl From<&Player> for Hand {
    fn from(value: &Player) -> Self {
        Hand::new(Some(value.id))
    }
}
