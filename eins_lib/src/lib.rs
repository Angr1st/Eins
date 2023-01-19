mod game;
mod cards;

use game::{GameSession, Hand};

pub fn test() -> GameSession<'static> {
    let cards= cards::init_deck();
    let hand_one = Hand::new(None);
    let hand_two = Hand::new(None);
    let players = vec![hand_one, hand_two];
    let game_session = GameSession::new(&cards, players);
    game_session.progress()
}
