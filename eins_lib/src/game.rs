use std::{borrow::BorrowMut, fmt::Display};

use uuid::Uuid;

use crate::cards::{
    self, create_deck, CardAction, CardReference, CardTypes, Color, DrawAction, ALL_CARDS,
};

pub const INITIAL_HAND_CARDS: usize = 7;
pub const MAX_NUMBER_OF_PLAYERS: usize = 10;

fn possible_next_card(
    current_card: CardReference,
    hand: &Vec<CardReference>,
    color_constraint_opt: Option<Color>,
) -> Play {
    let actual_card = cards::retrieve_card(&current_card);
    let possible_cards: Vec<CardReference> = hand
        .into_iter()
        .filter(|next_card| {
            let next_card_ref = cards::retrieve_card(next_card);
            actual_card.is_possible_next_card(&next_card_ref, color_constraint_opt)
        })
        .cloned()
        .collect();
    if possible_cards.len() == 0 {
        Play::DrawCards {
            draw_amount: DrawAction::DrawOne,
        }
    } else {
        Play::PossibleCards {
            options: possible_cards,
        }
    }
}

#[derive(Debug)]
pub struct ActualSession {
    stack: Vec<CardReference>,
    deck: Vec<CardReference>,
    players: Vec<Hand>,
    game_direction: GameDirection,
    current_player: usize,
    player_number: usize,
    game_id: Uuid,
}

impl ActualSession {
    pub fn get_game_id(self: &Self) -> &Uuid {
        &self.game_id
    }
}

#[derive(Debug)]
pub struct GameSession<G: GameSessionState> {
    game_state: G,
    session_state: Box<ActualSession>,
}

#[derive(Debug)]
pub struct GameSetup {}

#[derive(Debug)]
pub enum Play {
    PossibleCards { options: Vec<CardReference> },
    DrawCards { draw_amount: DrawAction },
}

#[derive(Debug)]
pub struct ColorWish {
    color: Color,
}

#[derive(Debug)]
pub struct FinishGame {
    winner: Uuid,
}

#[derive(Debug)]
pub enum GameError {
    NotEnoughPlayers,
    ToManyPlayers,
}

trait GameSessionState {}
impl GameSessionState for GameSetup {}
impl GameSessionState for Play {}
impl GameSessionState for ColorWish {}
impl GameSessionState for FinishGame {}

impl GameSession<GameSetup> {
    pub fn new(players: Vec<Hand>) -> Result<Self, GameError> {
        let player_number: usize = players.len();
        if player_number < 2 {
            return Err(GameError::NotEnoughPlayers);
        } else if player_number > MAX_NUMBER_OF_PLAYERS {
            return Err(GameError::ToManyPlayers);
        }

        let mut deck = create_deck();

        let starting_card = GameSession::find_starting_card(&mut deck);

        let session = ActualSession {
            stack: vec![starting_card],
            deck,
            players,
            game_direction: GameDirection::Clockwise,
            current_player: 0,
            player_number,
            game_id: uuid::Uuid::new_v4(),
        };

        let mut init = Self {
            session_state: Box::new(session),
            game_state: GameSetup {},
        };
        Ok(init.deal_out_hand_cards())
    }

    fn find_starting_card(deck: &mut Vec<CardReference>) -> CardReference {
        let mut first_valid_card_position = 0;

        for card_ref in deck.iter() {
            let index: usize = card_ref.into();
            let card: &CardTypes = &ALL_CARDS.get(index).expect("Card should always exist!");

            first_valid_card_position = first_valid_card_position + 1;

            if card.is_possible_initial_card() {
                break;
            }
        }

        deck.remove(first_valid_card_position)
    }

    fn deal_out_hand_cards(mut self: Self) -> Self {
        let player_count = self.session_state.players.len();
        for _ in 0..INITIAL_HAND_CARDS {
            for player_nr in 0..player_count {
                let card = self.session_state.deck.pop();
                let player = &mut self.session_state.players[player_nr];
                player
                    .held_cards
                    .push(card.expect("there should be a cardreference here!"))
            }
        }

        self
    }

    pub fn start_game(mut self: Self) -> GameSession<Play> {
        let current_card = self
            .session_state
            .stack
            .pop()
            .expect("The stack should at least contain a card!");
        let current_hand = self
            .session_state
            .players
            .get(self.session_state.current_player as usize)
            .expect(&format!(
                "The current player's {} hand is missing.",
                self.session_state.current_player
            ));
        let next_play = possible_next_card(current_card, &current_hand.held_cards, None);
        GameSession::<Play> {
            session_state: self.session_state,
            game_state: next_play,
        }
    }
}

impl GameSession<Play> {}

impl<G: GameSessionState> GameSession<G> {
    fn next_player(mut self: Self) -> Self {
        match self.session_state.game_direction {
            GameDirection::Clockwise => {
                if self.session_state.current_player + 1 == self.session_state.player_number {
                    self.session_state.current_player = 0;
                } else {
                    self.session_state.current_player = self.session_state.current_player + 1;
                }
            }
            GameDirection::CounterClockwise => {
                if self.session_state.current_player - 1 == 0 {
                    self.session_state.current_player = self.session_state.player_number - 1;
                } else {
                    self.session_state.current_player = self.session_state.current_player - 1;
                }
            }
        };

        self
    }
}

impl<G: GameSessionState + std::fmt::Debug> Display for GameSession<G> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.game_state)
    }
}

#[derive(Debug)]
pub struct Hand {
    player_id: Uuid,
    held_cards: Vec<CardReference>,
    status: HandState,
}

impl Hand {
    pub fn new(id_opt: Option<Uuid>) -> Self {
        let player_id = id_opt.unwrap_or_else(|| Uuid::new_v4());

        Hand {
            held_cards: vec![],
            status: HandState::Playing,
            player_id,
        }
    }
}

impl Default for Hand {
    fn default() -> Self {
        Hand::new(Some(Uuid::new_v4()))
    }
}

#[derive(Debug)]
pub enum GameState {
    Init,
    Regular { turn_state: TurnState },
    Finished,
}

#[derive(Debug)]
pub enum HandState {
    Playing,
    Won,
    Lost,
}

#[derive(Debug)]
pub enum TurnState {
    Init,
    PlayCard { card_action: CardAction },
    Draw { draw_action: Vec<DrawAction> },
    Skip,
    ColorWish { color: Color },
    ChangeDirection,
    NextPlayer,
}

impl TurnState {
    fn new_draw(draw_actions: Vec<DrawAction>) -> Self {
        TurnState::Draw {
            draw_action: draw_actions,
        }
    }

    fn new_default_draw() -> Self {
        TurnState::Draw {
            draw_action: vec![DrawAction::default()],
        }
    }

    fn add_draw(self, next_draw_action: DrawAction) -> Self {
        match self {
            TurnState::Draw { mut draw_action } => {
                draw_action.push(next_draw_action);
                TurnState::Draw { draw_action }
            }
            _ => panic!(),
        }
    }
}

impl Default for TurnState {
    fn default() -> Self {
        TurnState::Init
    }
}

#[derive(Debug)]
pub enum GameDirection {
    Clockwise,
    CounterClockwise,
}

impl Default for GameDirection {
    fn default() -> Self {
        GameDirection::Clockwise
    }
}
