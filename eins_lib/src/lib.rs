pub mod cards;
mod game;

use game::{GameError, GameSession, GameSetup, Hand, Play};

pub fn test() -> Result<GameSession<Play>, GameError> {
    let hand_one = Hand::default();
    let hand_two = Hand::default();
    let players = vec![hand_one, hand_two];
    let game_session = GameSession::<GameSetup>::new(players)?;
    let first_move = game_session.start_game();
    Ok(first_move)
}
