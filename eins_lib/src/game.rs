use std::{fmt::Display, borrow::BorrowMut};

use uuid::Uuid;

use crate::cards::{self, create_deck, CardAction, CardReference, CardTypes, Color, DrawAction};

pub const INITIAL_HAND_CARDS: usize = 7;
pub const MAX_NUMBER_OF_PLAYERS: usize = 10;

fn possible_next_card(currentCard: CardReference, hand: &Vec<CardReference>, color_constraint_opt: Option<Color>) -> Play {
    let actual_card = cards::retrieve_card(currentCard);
    hand.into_iter().filter(|next_card| )
}

pub struct ActualSession {
    stack: Vec<CardReference>,
    deck: Vec<CardReference>,
    players: Vec<Hand>,
    game_direction: GameDirection,
    current_player: u8,
    player_number: u8,
}

pub struct GameSession<G: GameSessionState> {
    game_state: G,
    session_state: Box<ActualSession>,
}

enum GameSetup {}
enum Play {
    PossibleCards { options: Vec<CardReference> },
    DrawCards { draw_amount: DrawAction },
}
struct ColorWish {
    color: Color,
}
struct FinishGame {
    winner: Uuid,
}

trait GameSessionState {}
impl GameSessionState for GameSetup {}
impl GameSessionState for Play {}
impl GameSessionState for ColorWish {}
impl GameSessionState for FinishGame {}

impl GameSession<GameSetup> {
    pub fn new(players: Vec<Hand>) -> Self {
        let player_number: u8 = players
            .len()
            .try_into()
            .expect("The maximum number of players is 10");
        let mut deck = create_deck();

        debug_assert!(cards.len() > 0);

        let starting_card = GameSession::find_starting_card(cards, &mut deck);

        let session = ActualSession {
            stack: vec![starting_card],
            deck,
            players,
            game_direction: GameDirection::Clockwise,
            current_player: 0,
            player_number,
        };

        let mut init = Self {
            session_state: Box::new(session),
            game_state: GameSetup {},
        };
        init.deal_out_hand_cards()
    }

    fn find_starting_card(deck: &mut Vec<CardReference>) -> CardReference {
        let mut first_valid_card_position = 0;

        for card_ref in deck.iter() {
            let index: usize = card_ref.into();
            let card: &CardTypes = cards.get(index).expect("Card should always exist!");

            first_valid_card_position = first_valid_card_position + 1;

            if card.is_possible_initial_card() {
                break;
            }
        }

        deck.remove(first_valid_card_position)
    }

    fn deal_out_hand_cards(mut self: Self) -> Self {
        let player_count = self.players.len();
        for _ in 0..INITIAL_HAND_CARDS {
            for player_nr in 0..player_count {
                let card = self.deck.pop();
                let player = &mut self.players[player_nr];
                player
                    .held_cards
                    .push(card.expect("there should be a cardreference here!"))
            }
        }
        self.game_state = GameState::Regular {
            turn_state: TurnState::default(),
        };
        self
    }
}

    fn draw_phase(mut self: Self, draw_amount: u8) -> Self {
        let player = &mut self.players[self.current_player as usize];
        for _ in 0..draw_amount {
            let card = self.deck.pop();
            player
                .held_cards
                .push(card.expect("there should be a cardreference here!"))
        }
        self.game_state = GameState::Regular {
            turn_state: TurnState::default(),
        };

        self
    }

    fn play_card(mut self: Self) -> Self {
        todo!()
    }

    fn next_player(mut self: Self) -> Self {
        match self.game_direction {
            GameDirection::Clockwise => {
                if self.current_player + 1 == self.player_number {
                    self.current_player = 0;
                } else {
                    self.current_player = self.current_player + 1;
                }
            }
            GameDirection::CounterClockwise => {
                if self.current_player - 1 == 0 {
                    self.current_player = self.player_number - 1;
                } else {
                    self.current_player = self.current_player - 1;
                }
            }
        };

        self
    }

    pub fn progress(self: Self) -> Self {
        match self.game_state {
            GameState::Init => self.deal_out_hand_cards(),
            GameState::Regular { ref turn_state } => match turn_state {
                TurnState::Init => todo!(),
                TurnState::Skip => todo!(),
                TurnState::PlayCard { card_action } => self.play_card(),
                TurnState::Draw { draw_action } => {
                    let draw_amount = draw_action
                        .iter()
                        .fold(0 as u8, |acc, x| acc + <&DrawAction as Into<u8>>::into(&x));
                    self.draw_phase(draw_amount)
                }
                TurnState::ColorWish { color } => todo!(),
                TurnState::ChangeDirection => todo!(),
                TurnState::NextPlayer => self.next_player(),
            },
            GameState::Finished => todo!(),
        }
    }
}

impl<G> Display for GameSession<G> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.game_state)
    }
}

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

pub enum GameDirection {
    Clockwise,
    CounterClockwise,
}

impl Default for GameDirection {
    fn default() -> Self {
        GameDirection::Clockwise
    }
}
