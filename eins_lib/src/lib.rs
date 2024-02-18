pub mod cards;
mod game;

use cards::CardTypes;
use game::{GameSession, Hand};

pub fn test() -> GameSession<'static> {
    let cards = &cards::ALL_CARDS;
    let length_of_cards = cards.len();
    let hand_one = Hand::default();
    let hand_two = Hand::default();
    let players = vec![hand_one, hand_two];
    let game_session = GameSession::new(&cards, players);
    game_session.progress()
}
