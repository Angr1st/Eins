pub mod cards;
pub mod game;
pub mod infrastructure;

use game::{GameError, GamePlay, GameSession, GameSetup, Hand};

pub fn test() -> Result<GameSession<GamePlay>, GameError> {
    let hand_one = Hand::default();
    let hand_two = Hand::default();
    let players = vec![hand_one, hand_two];
    let game_session = GameSession::<GameSetup>::new(players)?;
    let first_move = game_session.start_game();
    Ok(first_move)
}
