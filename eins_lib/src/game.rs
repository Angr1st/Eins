use std::fmt::Display;

use rand::{rng, seq::SliceRandom};
use uuid::Uuid;

use crate::cards::{
    create_deck, get_card, CardReference, CardTypes, Color, ColorCard, ColorSymbol, WildCard,
    WildSymbol,
};

pub const INITIAL_HAND_CARDS: usize = 7;
pub const MAX_NUMBER_OF_PLAYERS: usize = 10;
/// Cards a player draws when they are down to one card without having called "Eins".
pub const EINS_PENALTY: usize = 2;

#[derive(Debug)]
pub struct ActualSession {
    /// The discard pile; the last card is the top card.
    stack: Vec<CardReference>,
    /// The draw pile; cards are drawn from the end.
    deck: Vec<CardReference>,
    players: Vec<Hand>,
    game_direction: GameDirection,
    current_player: usize,
    player_number: usize,
    game_id: Uuid,
    creator_id: Uuid,
}

impl ActualSession {
    pub fn get_game_id(&self) -> &Uuid {
        &self.game_id
    }

    pub fn get_players(&self) -> &Vec<Hand> {
        &self.players
    }

    pub fn get_number_of_players(&self) -> usize {
        self.player_number
    }

    fn top_card(&self) -> CardTypes {
        get_card(
            self.stack
                .last()
                .expect("There should always be at least one card on the stack."),
        )
    }

    fn current_hand(&self) -> &Hand {
        &self.players[self.current_player]
    }

    /// Moves the turn `steps` players on in the current direction.
    fn advance(&mut self, steps: usize) {
        let n = self.player_number;
        for _ in 0..steps {
            self.current_player = match self.game_direction {
                GameDirection::Clockwise => (self.current_player + 1) % n,
                GameDirection::CounterClockwise => (self.current_player + n - 1) % n,
            };
        }
    }

    /// Gives `amount` cards to `player`, shuffling the discard pile (except its top card)
    /// back into the draw pile when it runs out. Stops early when no card is left anywhere.
    fn draw(&mut self, player: usize, amount: usize) {
        for _ in 0..amount {
            if self.deck.is_empty() {
                let top = self
                    .stack
                    .pop()
                    .expect("The stack should at least contain a card!");
                self.deck.append(&mut self.stack);
                self.deck.shuffle(&mut rng());
                self.stack.push(top);
            }
            let Some(card) = self.deck.pop() else {
                return;
            };
            self.players[player].held_cards.push(card);
        }
    }
}

pub enum Game {
    Setup(GameSession<GameSetup>),
    Play(GameSession<GamePlay>),
    Finished(GameSession<FinishGame>),
}

impl Game {
    ///By convention the first players Id is also the game id.
    pub fn new(players: Vec<Uuid>) -> Result<Self, GameError> {
        let actual_session = GameSession::new(players)?;
        Ok(Game::Setup(actual_session))
    }

    fn session(&self) -> &ActualSession {
        match self {
            Game::Setup(s) => &s.session_state,
            Game::Play(p) => &p.session_state,
            Game::Finished(f) => &f.session_state,
        }
    }

    pub fn get_game_id(&self) -> &Uuid {
        &self.session().game_id
    }

    pub fn get_creator_id(&self) -> &Uuid {
        &self.session().creator_id
    }

    pub fn get_number_of_players(&self) -> usize {
        self.session().player_number
    }

    pub fn get_game_session_state(&self) -> &str {
        match self {
            Game::Setup(s) => s.get_game_session_state(),
            Game::Play(p) => p.get_game_session_state(),
            Game::Finished(f) => f.get_game_session_state(),
        }
    }

    /// `player` plays `card`. Wild cards need a `color_wish`; other cards must not have one.
    /// `called_eins` is the player's "Eins" call for when this play leaves them with one card.
    /// A rejected move leaves the game unchanged.
    pub fn play_card(
        self,
        player: Uuid,
        card: CardReference,
        color_wish: Option<Color>,
        called_eins: bool,
    ) -> (Game, Result<(), MoveError>) {
        match self {
            Game::Play(session) => session.play_card(player, card, color_wish, called_eins),
            other => (other, Err(MoveError::GameNotRunning)),
        }
    }

    /// `player` draws one card, or all pending cards of a +2 stack, which ends their turn.
    pub fn draw(self, player: Uuid) -> (Game, Result<(), MoveError>) {
        match self {
            Game::Play(session) => session.draw(player),
            other => (other, Err(MoveError::GameNotRunning)),
        }
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
pub struct GamePlay {
    /// The color the next card has to match: the top card's color, or the color chosen with
    /// a wild card.
    color: Color,
    /// Cards the current player has to draw unless they stack another +2 or play a +4.
    pending_draw: usize,
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

impl Display for GameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GameError::NotEnoughPlayers => write!(f, "Not enough players"),
            GameError::ToManyPlayers => write!(f, "To many players"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum MoveError {
    GameNotRunning,
    NotYourTurn,
    CardNotInHand,
    CardNotPlayable,
    ColorWishRequired,
    ColorWishNotAllowed,
}

impl Display for MoveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MoveError::GameNotRunning => write!(f, "The game is not running"),
            MoveError::NotYourTurn => write!(f, "It is not your turn"),
            MoveError::CardNotInHand => write!(f, "The card is not in your hand"),
            MoveError::CardNotPlayable => write!(f, "The card cannot be played now"),
            MoveError::ColorWishRequired => write!(f, "A wild card needs a color wish"),
            MoveError::ColorWishNotAllowed => write!(f, "Only wild cards take a color wish"),
        }
    }
}

pub trait GameSessionState {}
impl GameSessionState for GameSetup {}
impl GameSessionState for GamePlay {}
impl GameSessionState for FinishGame {}

impl GameSession<GameSetup> {
    pub fn new(players: Vec<Uuid>) -> Result<Self, GameError> {
        let player_number: usize = players.len();
        if player_number < 2 {
            return Err(GameError::NotEnoughPlayers);
        } else if player_number > MAX_NUMBER_OF_PLAYERS {
            return Err(GameError::ToManyPlayers);
        }

        let mut deck = create_deck();

        let starting_card = GameSession::find_starting_card(&mut deck);

        let creator_id = players[0];
        let session = ActualSession {
            stack: vec![starting_card],
            deck,
            players: players.iter().map(|&id| id.into()).collect(),
            game_direction: GameDirection::Clockwise,
            current_player: rand::random_range(0..player_number),
            player_number,
            game_id: creator_id,
            creator_id,
        };

        let init = Self {
            session_state: Box::new(session),
            game_state: GameSetup {},
        };
        Ok(init.deal_out_hand_cards())
    }

    fn find_starting_card(deck: &mut Vec<CardReference>) -> CardReference {
        let position = deck
            .iter()
            .position(|card_ref| get_card(card_ref).is_possible_initial_card())
            .expect("The deck contains non-wild cards");
        deck.remove(position)
    }

    fn deal_out_hand_cards(mut self) -> Self {
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

    /// Starts the game and applies the starting card: a skip skips the first player, a
    /// reverse reverses the direction, and a +2 starts a stack for the first player.
    pub fn start_game(self) -> GameSession<GamePlay> {
        let mut session = self.session_state;
        let CardTypes::Normal(top) = session.top_card() else {
            unreachable!("The starting card is never a wild card");
        };
        let mut pending_draw = 0;
        match top.symbol {
            ColorSymbol::Skip => session.advance(1),
            ColorSymbol::Reverse => session.game_direction = session.game_direction.reversed(),
            ColorSymbol::DrawTwo => pending_draw = 2,
            _ => {}
        }
        GameSession {
            session_state: session,
            game_state: GamePlay {
                color: top.color,
                pending_draw,
            },
        }
    }

    pub fn get_game_session_state(&self) -> &str {
        "Setup"
    }
}

impl GameSession<GamePlay> {
    pub fn get_game_session_state(&self) -> &str {
        "Play"
    }

    pub fn get_current_player(&self) -> &Uuid {
        &self.session_state.current_hand().player_id
    }

    pub fn get_color(&self) -> Color {
        self.game_state.color
    }

    pub fn get_pending_draw(&self) -> usize {
        self.game_state.pending_draw
    }

    /// The cards the current player may play now.
    pub fn playable_cards(&self) -> Vec<CardReference> {
        self.session_state
            .current_hand()
            .held_cards
            .iter()
            .copied()
            .filter(|card| self.is_playable(card))
            .collect()
    }

    fn is_playable(&self, card: &CardReference) -> bool {
        let card = get_card(card);
        if self.game_state.pending_draw > 0 {
            // Only another +2 or a +4 answers a +2 stack.
            return matches!(
                card,
                CardTypes::Normal(ColorCard {
                    symbol: ColorSymbol::DrawTwo,
                    ..
                }) | CardTypes::Wild(WildCard {
                    symbol: WildSymbol::DrawFour
                })
            );
        }
        card.can_be_played_on(&self.session_state.top_card(), self.game_state.color)
    }

    fn check_turn(&self, player: Uuid) -> Result<(), MoveError> {
        if *self.get_current_player() != player {
            return Err(MoveError::NotYourTurn);
        }
        Ok(())
    }

    fn check_play(
        &self,
        player: Uuid,
        card: CardReference,
        color_wish: Option<Color>,
    ) -> Result<(), MoveError> {
        self.check_turn(player)?;
        if !self.session_state.current_hand().held_cards.contains(&card) {
            return Err(MoveError::CardNotInHand);
        }
        if !self.is_playable(&card) {
            return Err(MoveError::CardNotPlayable);
        }
        match (get_card(&card), color_wish) {
            (CardTypes::Wild(_), None) => Err(MoveError::ColorWishRequired),
            (CardTypes::Normal(_), Some(_)) => Err(MoveError::ColorWishNotAllowed),
            _ => Ok(()),
        }
    }

    fn play_card(
        self,
        player: Uuid,
        card: CardReference,
        color_wish: Option<Color>,
        called_eins: bool,
    ) -> (Game, Result<(), MoveError>) {
        if let Err(error) = self.check_play(player, card, color_wish) {
            return (Game::Play(self), Err(error));
        }
        let GameSession {
            session_state: mut session,
            game_state: mut state,
        } = self;

        let current = session.current_player;
        let hand = &mut session.players[current].held_cards;
        let position = hand.iter().position(|c| *c == card).expect("checked above");
        hand.remove(position);
        let cards_left = hand.len();
        session.stack.push(card);

        if cards_left == 0 {
            let finished = GameSession {
                session_state: session,
                game_state: FinishGame { winner: player },
            };
            return (Game::Finished(finished), Ok(()));
        }
        if cards_left == 1 && !called_eins {
            session.draw(current, EINS_PENALTY);
        }

        match get_card(&card) {
            CardTypes::Normal(played) => {
                state.color = played.color;
                match played.symbol {
                    ColorSymbol::Skip => session.advance(2),
                    ColorSymbol::Reverse => {
                        session.game_direction = session.game_direction.reversed();
                        // With two players a reverse works like a skip.
                        session.advance(if session.player_number == 2 { 2 } else { 1 });
                    }
                    ColorSymbol::DrawTwo => {
                        state.pending_draw += 2;
                        session.advance(1);
                    }
                    _ => session.advance(1),
                }
            }
            CardTypes::Wild(played) => {
                state.color = color_wish.expect("checked above");
                session.advance(1);
                if played.symbol == WildSymbol::DrawFour {
                    // A +4 ends any +2 stack: the next player draws everything and loses
                    // their turn; the player after them continues with the chosen color.
                    let victim = session.current_player;
                    session.draw(victim, state.pending_draw + 4);
                    state.pending_draw = 0;
                    session.advance(1);
                }
            }
        }

        let next = GameSession {
            session_state: session,
            game_state: state,
        };
        (Game::Play(next), Ok(()))
    }

    fn draw(self, player: Uuid) -> (Game, Result<(), MoveError>) {
        if let Err(error) = self.check_turn(player) {
            return (Game::Play(self), Err(error));
        }
        let GameSession {
            session_state: mut session,
            game_state: mut state,
        } = self;

        let current = session.current_player;
        session.draw(current, state.pending_draw.max(1));
        state.pending_draw = 0;
        session.advance(1);

        let next = GameSession {
            session_state: session,
            game_state: state,
        };
        (Game::Play(next), Ok(()))
    }
}

impl GameSession<FinishGame> {
    pub fn get_game_session_state(&self) -> &str {
        "Finished"
    }

    pub fn get_winner(&self) -> &Uuid {
        &self.game_state.winner
    }
}

impl<G: GameSessionState> GameSession<G> {
    pub fn get_players(&self) -> &Vec<Hand> {
        &self.session_state.players
    }

    pub fn get_current_card(&self) -> CardTypes {
        self.session_state.top_card()
    }
}

impl<G: GameSessionState + std::fmt::Debug> Display for GameSession<G> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "GameState: {:?}", self.game_state)?;
        write!(f, "SessionState: {:?}", self.session_state)
    }
}

#[derive(Debug)]
pub struct Hand {
    player_id: Uuid,
    held_cards: Vec<CardReference>,
}

impl Hand {
    pub fn new(id: Uuid) -> Self {
        Hand {
            held_cards: vec![],
            player_id: id,
        }
    }

    pub fn get_player_id(&self) -> &Uuid {
        &self.player_id
    }

    pub fn get_held_cards(&self) -> &Vec<CardReference> {
        &self.held_cards
    }
}

impl From<Uuid> for Hand {
    fn from(value: Uuid) -> Self {
        Hand::new(value)
    }
}

impl Default for Hand {
    fn default() -> Self {
        Self::new(Uuid::new_v4())
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum GameDirection {
    #[default]
    Clockwise,
    CounterClockwise,
}

impl GameDirection {
    fn reversed(self) -> Self {
        match self {
            GameDirection::Clockwise => GameDirection::CounterClockwise,
            GameDirection::CounterClockwise => GameDirection::Clockwise,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::{ALL_CARDS, MAX_CARD_NUMBER};
    use std::collections::HashSet;
    use Color::*;
    use ColorSymbol::*;

    fn c(color: Color, symbol: ColorSymbol) -> CardTypes {
        CardTypes::Normal(ColorCard { color, symbol })
    }
    const WILD: CardTypes = CardTypes::Wild(WildCard {
        symbol: WildSymbol::ChooseColor,
    });
    const PLUS4: CardTypes = CardTypes::Wild(WildCard {
        symbol: WildSymbol::DrawFour,
    });

    /// Builds a session from fixed cards: `stack` has the top card last, player 0 moves first.
    fn table<G: GameSessionState>(
        game_state: G,
        stack: &[CardTypes],
        hands: &[&[CardTypes]],
        deck: &[CardTypes],
    ) -> GameSession<G> {
        let mut used = [false; MAX_CARD_NUMBER];
        let mut take = |card: &CardTypes| {
            let i = (0..MAX_CARD_NUMBER)
                .find(|&i| !used[i] && ALL_CARDS[i] == *card)
                .expect("no such card left in the deck");
            used[i] = true;
            CardReference::try_from(i).unwrap()
        };
        let stack = stack.iter().map(&mut take).collect();
        let players: Vec<Hand> = hands
            .iter()
            .map(|hand| Hand {
                player_id: Uuid::new_v4(),
                held_cards: hand.iter().map(&mut take).collect(),
            })
            .collect();
        let deck = deck.iter().map(&mut take).collect();
        let creator_id = players[0].player_id;
        GameSession {
            game_state,
            session_state: Box::new(ActualSession {
                stack,
                deck,
                player_number: players.len(),
                players,
                game_direction: GameDirection::Clockwise,
                current_player: 0,
                game_id: creator_id,
                creator_id,
            }),
        }
    }

    fn playing(stack: &[CardTypes], hands: &[&[CardTypes]], deck: &[CardTypes]) -> Game {
        let color = match stack.last().unwrap() {
            CardTypes::Normal(top) => top.color,
            CardTypes::Wild(_) => Red,
        };
        let state = GamePlay {
            color,
            pending_draw: 0,
        };
        Game::Play(table(state, stack, hands, deck))
    }

    fn play_session(game: &Game) -> &GameSession<GamePlay> {
        match game {
            Game::Play(session) => session,
            _ => panic!("the game is not running"),
        }
    }

    fn id(game: &Game, player: usize) -> Uuid {
        game.session().players[player].player_id
    }

    fn current(game: &Game) -> usize {
        game.session().current_player
    }

    fn hand(game: &Game, player: usize) -> Vec<CardTypes> {
        game.session().players[player]
            .held_cards
            .iter()
            .map(get_card)
            .collect()
    }

    /// The current player tries to play the card at `index` of their hand.
    fn try_play(
        game: Game,
        index: usize,
        wish: Option<Color>,
        eins: bool,
    ) -> (Game, Result<(), MoveError>) {
        let player = current(&game);
        let card = game.session().players[player].held_cards[index];
        let player = id(&game, player);
        game.play_card(player, card, wish, eins)
    }

    /// Plays with "Eins" called, so the penalty only shows up where a test asks for it.
    fn play(game: Game, index: usize, wish: Option<Color>) -> Game {
        let (game, result) = try_play(game, index, wish, true);
        result.unwrap();
        game
    }

    fn draw(game: Game) -> Game {
        let player = id(&game, current(&game));
        let (game, result) = game.draw(player);
        result.unwrap();
        game
    }

    fn filler(n: usize) -> Vec<CardTypes> {
        [One, Two, Three, Four, Six, Seven, Eight, Nine]
            .iter()
            .cycle()
            .take(n)
            .map(|&symbol| c(Orange, symbol))
            .collect()
    }

    #[test]
    fn game_id_is_creator_id() {
        let ids: Vec<Uuid> = (0..3).map(|_| Uuid::new_v4()).collect();
        let game = Game::new(ids.clone()).unwrap();
        assert_eq!(game.get_game_id(), &ids[0]);
        assert_eq!(game.get_creator_id(), &ids[0]);
    }

    #[test]
    fn game_reports_session_state() {
        let game = Game::new(vec![Uuid::new_v4(), Uuid::new_v4()]).unwrap();
        assert_eq!(game.get_game_session_state(), "Setup");
        let Game::Setup(setup) = game else {
            unreachable!()
        };
        assert_eq!(
            Game::Play(setup.start_game()).get_game_session_state(),
            "Play"
        );
    }

    #[test]
    fn moves_need_a_running_game() {
        let ids = vec![Uuid::new_v4(), Uuid::new_v4()];
        let game = Game::new(ids.clone()).unwrap();
        let (game, result) = game.draw(ids[0]);
        assert_eq!(result, Err(MoveError::GameNotRunning));
        assert_eq!(game.get_game_session_state(), "Setup");
    }

    #[test]
    fn advance_wraps_in_both_directions() {
        let mut session = table(GameSetup {}, &[c(Red, Five)], &[&[], &[], &[]], &[]).session_state;
        let mut order = vec![];
        for _ in 0..4 {
            session.advance(1);
            order.push(session.current_player);
        }
        assert_eq!(order, vec![1, 2, 0, 1]);

        session.game_direction = GameDirection::CounterClockwise;
        order.clear();
        for _ in 0..4 {
            session.advance(1);
            order.push(session.current_player);
        }
        assert_eq!(order, vec![0, 2, 1, 0]);
    }

    #[test]
    fn only_the_current_player_may_move() {
        let game = playing(
            &[c(Red, Five)],
            &[&[c(Red, Seven)], &[c(Red, Eight)]],
            &[c(Blue, One)],
        );
        let bob = id(&game, 1);
        let bobs_card = game.session().players[1].held_cards[0];

        let (game, result) = game.play_card(bob, bobs_card, None, false);
        assert_eq!(result, Err(MoveError::NotYourTurn));
        let (game, result) = game.draw(bob);
        assert_eq!(result, Err(MoveError::NotYourTurn));
        assert_eq!(current(&game), 0);
        assert_eq!(hand(&game, 1), vec![c(Red, Eight)]);
        assert_eq!(game.session().deck.len(), 1);
    }

    #[test]
    fn cards_must_come_from_the_own_hand() {
        let game = playing(
            &[c(Red, Five)],
            &[&[c(Blue, Seven)], &[c(Red, Eight)]],
            &[c(Red, One)],
        );
        let alice = id(&game, 0);
        let others = [
            game.session().players[1].held_cards[0],
            game.session().deck[0],
        ];
        let mut game = game;
        for card in others {
            let (next, result) = game.play_card(alice, card, None, false);
            assert_eq!(result, Err(MoveError::CardNotInHand));
            game = next;
        }
        assert_eq!(game.session().stack.len(), 1);
    }

    #[test]
    fn cards_must_match_color_or_symbol() {
        let game = playing(
            &[c(Red, Five)],
            &[
                &[c(Blue, Seven), c(Blue, Five), c(Red, Two)],
                &[c(Green, One)],
            ],
            &[],
        );
        let playable: Vec<CardTypes> = play_session(&game)
            .playable_cards()
            .iter()
            .map(get_card)
            .collect();
        assert_eq!(playable, vec![c(Blue, Five), c(Red, Two)]);

        let (game, result) = try_play(game, 0, None, false);
        assert_eq!(result, Err(MoveError::CardNotPlayable));
        let game = play(game, 1, None);
        assert_eq!(play_session(&game).get_color(), Blue);
        assert_eq!(current(&game), 1);
    }

    #[test]
    fn wild_cards_need_a_color_wish_that_sets_the_color() {
        let game = playing(
            &[c(Red, Five)],
            &[
                &[WILD, c(Red, Two), c(Red, Three)],
                &[c(Red, Seven), c(Green, Nine)],
            ],
            &[],
        );
        let (game, result) = try_play(game, 0, None, false);
        assert_eq!(result, Err(MoveError::ColorWishRequired));
        let (game, result) = try_play(game, 1, Some(Blue), false);
        assert_eq!(result, Err(MoveError::ColorWishNotAllowed));

        let game = play(game, 0, Some(Green));
        assert_eq!(play_session(&game).get_color(), Green);
        let playable: Vec<CardTypes> = play_session(&game)
            .playable_cards()
            .iter()
            .map(get_card)
            .collect();
        assert_eq!(playable, vec![c(Green, Nine)]);
    }

    #[test]
    fn skip_skips_the_next_player() {
        let game = playing(
            &[c(Red, Five)],
            &[&[c(Red, Skip), c(Red, One)], &[], &[]],
            &[],
        );
        assert_eq!(current(&play(game, 0, None)), 2);
    }

    #[test]
    fn reverse_changes_the_direction() {
        let game = playing(
            &[c(Red, Five)],
            &[
                &[c(Red, Reverse), c(Red, One)],
                &[],
                &[c(Red, Three), c(Red, Four)],
            ],
            &[],
        );
        let game = play(game, 0, None);
        assert_eq!(current(&game), 2);
        let game = play(game, 0, None);
        assert_eq!(current(&game), 1);
    }

    #[test]
    fn reverse_with_two_players_works_like_skip() {
        let game = playing(
            &[c(Red, Five)],
            &[&[c(Red, Reverse), c(Red, One)], &[c(Red, Two)]],
            &[],
        );
        assert_eq!(current(&play(game, 0, None)), 0);
    }

    #[test]
    fn draw_two_stacks_until_someone_draws() {
        let game = playing(
            &[c(Red, Five)],
            &[
                &[c(Red, DrawTwo), c(Red, One)],
                &[c(Red, Six), c(Blue, DrawTwo), c(Blue, One)],
                &[c(Blue, Five), c(Red, Nine)],
            ],
            &filler(5),
        );
        let game = play(game, 0, None);
        assert_eq!(
            (current(&game), play_session(&game).get_pending_draw()),
            (1, 2)
        );
        // A red card no longer fits: only another +2 (of any color) or a +4 answers the stack.
        let playable: Vec<CardTypes> = play_session(&game)
            .playable_cards()
            .iter()
            .map(get_card)
            .collect();
        assert_eq!(playable, vec![c(Blue, DrawTwo)]);

        let game = play(game, 1, None);
        assert_eq!(
            (current(&game), play_session(&game).get_pending_draw()),
            (2, 4)
        );
        let (game, result) = try_play(game, 0, None, false);
        assert_eq!(result, Err(MoveError::CardNotPlayable));

        let game = draw(game);
        assert_eq!(hand(&game, 2).len(), 2 + 4);
        assert_eq!(
            (current(&game), play_session(&game).get_pending_draw()),
            (0, 0)
        );
    }

    #[test]
    fn draw_four_ends_the_stack_and_skips_the_drawing_player() {
        let game = playing(
            &[c(Red, Five)],
            &[
                &[c(Red, DrawTwo), c(Red, One), c(Green, One)],
                &[PLUS4, c(Blue, One)],
                &[c(Red, Two)],
            ],
            &filler(6),
        );
        let game = play(game, 0, None);
        let game = play(game, 0, Some(Green));
        assert_eq!(hand(&game, 2).len(), 1 + 6);
        assert_eq!(current(&game), 0);
        let session = play_session(&game);
        assert_eq!(
            (session.get_pending_draw(), session.get_color()),
            (0, Green)
        );
        let playable: Vec<CardTypes> = session.playable_cards().iter().map(get_card).collect();
        assert_eq!(playable, vec![c(Green, One)]);
    }

    #[test]
    fn draw_four_without_a_stack() {
        let game = playing(
            &[c(Red, Five)],
            &[&[PLUS4, c(Red, One)], &[c(Red, Two)], &[]],
            &filler(4),
        );
        let game = play(game, 0, Some(Blue));
        assert_eq!(hand(&game, 1).len(), 1 + 4);
        assert_eq!(current(&game), 2);
    }

    #[test]
    fn drawing_ends_the_turn_even_if_the_card_fits() {
        let game = playing(
            &[c(Red, Five)],
            &[&[c(Blue, One)], &[c(Green, Two)]],
            &[c(Red, Seven)],
        );
        let game = draw(game);
        assert_eq!(hand(&game, 0), vec![c(Blue, One), c(Red, Seven)]);
        assert_eq!(current(&game), 1);
    }

    #[test]
    fn forgetting_to_call_eins_costs_two_cards() {
        let hands: &[&[CardTypes]] = &[&[c(Red, One), c(Red, Two)], &[c(Green, Two)]];
        let (forgot, result) =
            try_play(playing(&[c(Red, Five)], hands, &filler(2)), 0, None, false);
        result.unwrap();
        assert_eq!(hand(&forgot, 0).len(), 1 + EINS_PENALTY);

        let (called, result) = try_play(playing(&[c(Red, Five)], hands, &filler(2)), 0, None, true);
        result.unwrap();
        assert_eq!(hand(&called, 0), vec![c(Red, Two)]);
    }

    #[test]
    fn playing_the_last_card_wins_without_card_effects() {
        let game = playing(
            &[c(Red, Five)],
            &[&[c(Red, DrawTwo)], &[c(Green, Two)]],
            &filler(2),
        );
        let alice = id(&game, 0);
        let game = play(game, 0, None);
        let Game::Finished(finished) = &game else {
            panic!("the game should be finished");
        };
        assert_eq!(finished.get_winner(), &alice);
        assert_eq!(hand(&game, 1).len(), 1);
    }

    #[test]
    fn an_empty_deck_is_refilled_from_the_discard_pile() {
        let game = playing(
            &[c(Blue, One), c(Blue, Two), c(Red, Five)],
            &[&[c(Green, One)], &[c(Green, Two)]],
            &[],
        );
        let game = draw(game);
        let drawn = hand(&game, 0)[1];
        assert!(drawn == c(Blue, One) || drawn == c(Blue, Two));
        assert_eq!(game.session().deck.len(), 1);
        assert_eq!(play_session(&game).get_current_card(), c(Red, Five));
        assert_eq!(game.session().stack.len(), 1);

        // With no cards left anywhere, drawing gives nothing but still ends the turn.
        let game = draw(draw(game));
        assert_eq!(hand(&game, 1).len(), 2);
        let game = draw(game);
        assert_eq!(hand(&game, 1).len(), 2);
        assert_eq!(current(&game), 0);
    }

    #[test]
    fn the_starting_card_takes_effect() {
        let start =
            |top: CardTypes| table(GameSetup {}, &[top], &[&[], &[], &[]], &[]).start_game();

        let skip = start(c(Red, Skip));
        assert_eq!(skip.session_state.current_player, 1);

        let reverse = start(c(Blue, Reverse));
        assert_eq!(
            reverse.session_state.game_direction,
            GameDirection::CounterClockwise
        );
        assert_eq!(reverse.session_state.current_player, 0);

        let draw_two = start(c(Green, DrawTwo));
        assert_eq!(
            (draw_two.get_pending_draw(), draw_two.get_color()),
            (2, Green)
        );
        assert_eq!(draw_two.session_state.current_player, 0);

        let number = start(c(Orange, Four));
        assert_eq!((number.get_pending_draw(), number.get_color()), (0, Orange));
        assert_eq!(number.session_state.current_player, 0);
    }

    #[test]
    fn the_first_player_is_random() {
        let first: HashSet<usize> = (0..50)
            .map(|_| {
                let ids = (0..4).map(|_| Uuid::new_v4()).collect();
                GameSession::<GameSetup>::new(ids)
                    .unwrap()
                    .session_state
                    .current_player
            })
            .collect();
        assert!(first.len() > 1);
    }
}
