mod game;
mod cards;

use cards::CardTypes;
use game::{GameSession, Hand};

pub fn test(cards: &'static Vec<CardTypes>) -> GameSession<'static> {

    let hand_one = Hand::new(None);
    let hand_two = Hand::new(None);
    let players = vec![hand_one, hand_two];
    let game_session = GameSession::new(&cards, players);
    game_session.progress()
}
