use std::fmt::Display;

use uuid::Uuid;

use crate::cards::{CardReference, create_deck, DrawAction, Color};

pub const INITIAL_HAND_CARDS: usize = 7;
pub struct GameSession {
    game_state:GameState,
    stack:Vec<CardReference>,
    deck:Vec<CardReference>,
    players:Vec<Hand>,
    game_direction:GameDirection,
    current_player:u8
}

impl GameSession {
    pub fn new(players:Vec<Hand>) -> Self {
        GameSession {
            game_state:GameState::Init,
            stack : vec![],
            deck: create_deck(),
            players,
            game_direction:GameDirection::Clockwise,
            current_player:0
        }
    }

    fn deal_out_hand_cards(mut self: Self) -> Self {
        let player_count = self.players.len();
        for _ in 0..INITIAL_HAND_CARDS {
            for player_nr in 0..player_count {
                let card = self.deck.pop();
                let player = &mut self.players[player_nr];
                player.held_cards.push(card.expect("there should be a cardreference here!"))
            }
        }
        self.game_state = GameState::Regular { turn_state : TurnState::default() };
        self
    }

    fn draw_phase(mut self: Self, draw_amount: u8) -> Self {
        let player = &mut self.players[self.current_player as usize];
        for _ in 0..draw_amount {
            let card = self.deck.pop();
            player.held_cards.push(card.expect("there should be a cardreference here!"))
        }
        self.game_state = GameState::Regular { turn_state: TurnState::PlayCard };

        self
    }

    fn play_card(mut self: Self) -> Self {
        todo!()
    }

    pub fn progress(self: Self) -> Self {
        match self.game_state {
            GameState::Init => self.deal_out_hand_cards(),
            GameState::Regular { ref turn_state } =>  match turn_state {
                TurnState::Skip => todo!(),
                TurnState::PlayCard => self.play_card(),
                TurnState::Draw { draw_action } => {
                    let draw_amount = draw_action.into_iter().fold(0 as u8, |acc, x| acc + <&DrawAction as Into<u8>>::into(&x));
                    self.draw_phase(draw_amount)
                }
                TurnState::ColorWish { color } => todo!(),
                TurnState::ChangeDirection => todo!(),
            },
            GameState::Finished => todo!()
        }
    }
}

impl Display for GameSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.game_state)
    }
}

pub struct Hand {
    player_id:Uuid ,
    held_cards:Vec<CardReference>,
    status:HandState
}

impl Hand {
    pub fn new(id_opt: Option<Uuid>) -> Self {
        let player_id = id_opt.unwrap_or_else(||Uuid::new_v4());

        Hand {
            held_cards: vec![],
            status: HandState::Playing,
            player_id
        }
    }
}

#[derive(Debug)]
pub enum GameState {
    Init,
    Regular { turn_state: TurnState },
    Finished
}

pub enum HandState {
    Playing,
    Won,
    Lost
}

#[derive(Debug)]
pub enum TurnState {
    PlayCard,
    Draw { draw_action: Vec<DrawAction> },
    Skip,
    ColorWish { color: Color },
    ChangeDirection
}

impl TurnState {
    fn new_draw(draw_actions: Vec<DrawAction>) -> Self {
        TurnState::Draw { draw_action: draw_actions }
    }

    fn new_default_draw() -> Self {
        TurnState::Draw { draw_action: vec![DrawAction::default()] }
    }

    fn add_draw(self, next_draw_action: DrawAction) -> Self {
        match self {
            TurnState::Draw { mut draw_action } => {
                draw_action.push(next_draw_action);
                TurnState::Draw { draw_action }
            },
            _ => panic!()
        }
    }
}

impl Default for TurnState {
    fn default() -> Self {
        TurnState::PlayCard
    }
}



pub enum GameDirection {
    Clockwise,
    CounterClockwise
}

impl Default for GameDirection {
    fn default() -> Self {
        GameDirection::Clockwise
    }
}